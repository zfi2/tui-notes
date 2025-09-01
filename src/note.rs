use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use uuid::Uuid;
use crate::encryption::{EncryptionManager, EncryptedFile};
use base64::Engine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub pinned: bool,
}

impl Note {
    pub fn new(title: String, content: String) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();
        
        Note {
            id,
            title,
            content,
            created_at: now,
            updated_at: now,
            pinned: false,
        }
    }

    pub fn update_content(&mut self, content: String) {
        self.content = content;
        self.updated_at = Utc::now();
    }

    pub fn update_title(&mut self, title: String) {
        self.title = title;
        self.updated_at = Utc::now();
    }

    pub fn toggle_pin(&mut self) {
        self.pinned = !self.pinned;
        self.updated_at = Utc::now();
    }
}

#[derive(Debug)]
pub struct NoteManager {
    notes: HashMap<String, Note>,
    sorted_note_ids: Vec<String>,
    notes_file: PathBuf,
    cache_dirty: bool,
    encryption: EncryptionManager,
    encryption_enabled: bool,
    salt: Option<Vec<u8>>,
}

impl NoteManager {
    pub fn new<P: Into<PathBuf>>(notes_file: P, encryption_enabled: bool) -> io::Result<Self> {
        let mut manager = NoteManager {
            notes: HashMap::new(),
            sorted_note_ids: Vec::new(),
            notes_file: notes_file.into(),
            cache_dirty: true,
            encryption: EncryptionManager::new(),
            encryption_enabled,
            salt: None,
        };
        
        if !encryption_enabled {
            manager.load_notes()?;
        }
        Ok(manager)
    }

    // unlock encryption with password (only call this for encrypted vaults)
    pub fn unlock_encryption(&mut self, password: &str) -> io::Result<()> {
        if !self.encryption_enabled {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "encryption not enabled"));
        }

        // validate password on our end too for defense in depth
        if password.len() < 8 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "password too short"));
        }
        if password.len() > 256 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "password too long"));
        }

        if !self.notes_file.exists() {
            // new encrypted vault - generate salt and enable encryption
            let salt = EncryptionManager::generate_salt();
            self.encryption.unlock(password, &salt)?;
            self.salt = Some(salt.to_vec());
            return Ok(());
        }

        let content = fs::read_to_string(&self.notes_file).map_err(|_| {
            io::Error::new(io::ErrorKind::PermissionDenied, "cannot read file")
        })?;

        // validate file size to prevent resource exhaustion
        if content.len() > 110 * 1024 * 1024 { // slightly larger than MAX_CONTENT_SIZE to account for base64
            return Err(io::Error::new(io::ErrorKind::InvalidData, "file too large"));
        }

        if EncryptionManager::is_file_encrypted(&content) {
            let encrypted: EncryptedFile = serde_json::from_str(&content).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid file format")
            })?;
            
            let salt = base64::engine::general_purpose::STANDARD.decode(&encrypted.salt).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid file format")
            })?;

            // validate salt length before using it
            if salt.len() != 16 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid file format"));
            }

            self.encryption.unlock(password, &salt)?;
            self.salt = Some(salt);
            self.load_notes()?;
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "file is not encrypted"));
        }

        Ok(())
    }

    // check if this manager is ready to use (unlocked if encrypted)
    pub fn is_ready(&self) -> bool {
        if self.encryption_enabled {
            self.encryption.is_unlocked()
        } else {
            true
        }
    }


    pub fn add_note(&mut self, title: String, content: String) -> &Note {
        let note = Note::new(title, content);
        let id = note.id.clone();
        self.notes.insert(id.clone(), note);
        self.cache_dirty = true;
        &self.notes[&id]
    }


    pub fn get_note_mut(&mut self, id: &str) -> Option<&mut Note> {
        if let Some(note) = self.notes.get_mut(id) {
            self.cache_dirty = true;
            Some(note)
        } else {
            None
        }
    }

    pub fn delete_note(&mut self, id: &str) -> Option<Note> {
        let result = self.notes.remove(id);
        if result.is_some() {
            self.cache_dirty = true;
        }
        result
    }

    pub fn get_all_notes(&mut self) -> Vec<&Note> {
        self.update_sorted_cache();
        self.sorted_note_ids
            .iter()
            .filter_map(|id| self.notes.get(id))
            .collect()
    }


    pub fn search_notes(&mut self, query: &str) -> Vec<&Note> {
        if query.is_empty() {
            return self.get_all_notes();
        }
        
        self.update_sorted_cache();
        let query_lower = query.to_lowercase();
        
        self.sorted_note_ids
            .iter()
            .filter_map(|id| self.notes.get(id))
            .filter(|note| {
                note.title.to_lowercase().contains(&query_lower) ||
                note.content.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    fn update_sorted_cache(&mut self) {
        if !self.cache_dirty {
            return;
        }
        
        // pinned stuff goes first, then newest shit on top
        let mut note_refs: Vec<(&String, &Note)> = self.notes.iter().collect();
        note_refs.sort_by(|(_, a), (_, b)| {
            match b.pinned.cmp(&a.pinned) {
                std::cmp::Ordering::Equal => {
                    b.updated_at.cmp(&a.updated_at)
                }
                other => other,
            }
        });
        
        self.sorted_note_ids = note_refs.into_iter().map(|(id, _)| id.clone()).collect();
        self.cache_dirty = false;
    }

    pub fn save_notes(&self) -> io::Result<()> {
        if !self.is_ready() {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "notes manager is not ready"));
        }

        let json = serde_json::to_string_pretty(&self.notes)?;
        
        if self.encryption_enabled {
            let salt = self.salt.as_ref().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "no salt available for encryption")
            })?;
            let encrypted = self.encryption.encrypt(json.as_bytes(), salt)?;
            let encrypted_json = serde_json::to_string_pretty(&encrypted)?;
            fs::write(&self.notes_file, encrypted_json)?;
        } else {
            fs::write(&self.notes_file, json)?;
        }
        Ok(())
    }

    pub fn export_plaintext<P: Into<PathBuf>>(&self, export_file: P) -> io::Result<()> {
        if !self.is_ready() {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "notes manager is not ready"));
        }

        let json = serde_json::to_string_pretty(&self.notes)?;
        let export_path = export_file.into();
        
        // ensure parent directory exists
        if let Some(parent) = export_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        
        fs::write(&export_path, json)?;
        Ok(())
    }

    fn load_notes(&mut self) -> io::Result<()> {
        if !self.notes_file.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.notes_file)?;
        if content.trim().is_empty() {
            return Ok(());
        }

        let (json, needs_migration) = if self.encryption_enabled {
            if !self.encryption.is_unlocked() {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "encryption key not available"));
            }
            
            // check if file is already encrypted
            if EncryptionManager::is_file_encrypted(&content) {
                let encrypted: EncryptedFile = serde_json::from_str(&content).map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("failed to parse encrypted file: {}", e))
                })?;
                
                let decrypted_bytes = self.encryption.decrypt(&encrypted)?;
                let json = String::from_utf8(decrypted_bytes).map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("decrypted data is not valid utf-8: {}", e))
                })?;
                (json, false)
            } else {
                // file contains unencrypted notes - load them and mark for encryption migration
                (content, true)
            }
        } else {
            // check if file contains encrypted data when encryption is disabled
            if EncryptionManager::is_file_encrypted(&content) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData, 
                    "ENCRYPTED_FILE_DETECTED: The notes file appears to be encrypted, but encryption is disabled in config. Please enable encryption in config or use a different notes file."
                ));
            }
            (content, false)
        };

        self.notes = serde_json::from_str(&json).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse notes data: {}", e),
            )
        })?;
        self.cache_dirty = true;
        
        // if we loaded unencrypted notes but encryption is enabled, migrate them immediately
        if needs_migration {
            self.save_notes()?;
        }
        
        Ok(())
    }
}