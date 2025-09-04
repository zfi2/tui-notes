use std::io;
use serde::{Deserialize, Serialize};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Key, Nonce
};
use argon2::{Argon2, PasswordHasher, password_hash::{rand_core::RngCore, SaltString}};
use zeroize::Zeroize;
use base64::{Engine as _, engine::general_purpose};
use subtle::ConstantTimeEq;

pub const MIN_PASSWORD_LENGTH: usize = 8;
pub const MAX_PASSWORD_LENGTH: usize = 256;
const MAX_CONTENT_SIZE: usize = 100 * 1024 * 1024; // 100MB limit

const MAGIC_HEADER: &str = "ENCRYPTED_NOTES";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedFile {
    pub magic: String,
    pub salt: String,
    pub nonce: String,
    pub data: String,
}

#[derive(Debug)]
pub struct EncryptionManager {
    key: Option<Key>,
}

impl Default for EncryptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EncryptionManager {
    pub fn new() -> Self {
        Self {
            key: None,
        }
    }

    // derive key from password using argon2 (constant time operation)
    fn derive_key(&self, password: &str, salt: &[u8]) -> Result<Key, io::Error> {
        let salt_string = SaltString::encode_b64(salt).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid salt")
        })?;
        
        // use stronger argon2 parameters for better security
        let params = argon2::Params::new(
            65536, // 64MB memory cost
            3,     // time cost 
            1,     // parallelism
            Some(32) // output length
        ).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "parameter error")
        })?;
        
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id, // most secure variant
            argon2::Version::V0x13,
            params,
        );
        
        let password_hash = argon2.hash_password(password.as_bytes(), &salt_string).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "key derivation failed")
        })?;

        let hash = password_hash.hash.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "key derivation failed")
        })?;

        if hash.len() != 32 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "key derivation failed"));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(hash.as_bytes());
        Ok(Key::from(key_bytes))
    }

    // unlock the encryption manager with a password
    pub fn unlock(&mut self, password: &str, salt: &[u8]) -> Result<(), io::Error> {
        // validate password length for security
        if password.len() < MIN_PASSWORD_LENGTH {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "password too short"));
        }
        if password.len() > MAX_PASSWORD_LENGTH {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "password too long"));
        }
        
        // validate salt length
        if salt.len() != 16 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid salt length"));
        }
        
        let key = self.derive_key(password, salt)?;
        self.key = Some(key);
        Ok(())
    }

    // lock the manager and clear keys from memory
    pub fn lock(&mut self) {
        if let Some(mut key) = self.key.take() {
            key.zeroize();
        }
    }

    // check if we have a valid key
    pub fn is_unlocked(&self) -> bool {
        self.key.is_some()
    }

    // encrypt plaintext data (salt must be provided from unlock)
    pub fn encrypt(&self, data: &[u8], salt: &[u8]) -> Result<EncryptedFile, io::Error> {
        // validate input size to prevent resource exhaustion
        if data.len() > MAX_CONTENT_SIZE {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "content too large"));
        }
        
        if salt.len() != 16 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid salt length"));
        }
        
        let key = self.key.as_ref().ok_or_else(|| {
            io::Error::new(io::ErrorKind::PermissionDenied, "not unlocked")
        })?;

        let cipher = ChaCha20Poly1305::new(key);
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        
        let ciphertext = cipher.encrypt(&nonce, data).map_err(|_| {
            // don't leak detailed error information
            io::Error::new(io::ErrorKind::InvalidData, "encryption failed")
        })?;

        Ok(EncryptedFile {
            magic: MAGIC_HEADER.to_string(),
            salt: general_purpose::STANDARD.encode(&salt),
            nonce: general_purpose::STANDARD.encode(&nonce),
            data: general_purpose::STANDARD.encode(&ciphertext),
        })
    }

    // decrypt encrypted file
    pub fn decrypt(&self, encrypted: &EncryptedFile) -> Result<Vec<u8>, io::Error> {
        let key = self.key.as_ref().ok_or_else(|| {
            io::Error::new(io::ErrorKind::PermissionDenied, "not unlocked")
        })?;

        // constant time comparison to prevent timing attacks
        if !bool::from(encrypted.magic.as_bytes().ct_eq(MAGIC_HEADER.as_bytes())) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid format"));
        }

        let nonce_bytes = general_purpose::STANDARD.decode(&encrypted.nonce).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid format")
        })?;

        let ciphertext = general_purpose::STANDARD.decode(&encrypted.data).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid format")
        })?;

        // validate sizes to prevent attacks
        if nonce_bytes.len() != 12 || ciphertext.len() > MAX_CONTENT_SIZE + 16 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid format"));
        }

        let nonce = Nonce::from_slice(&nonce_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        
        cipher.decrypt(nonce, ciphertext.as_slice()).map_err(|_| {
            // don't leak information about why decryption failed
            io::Error::new(io::ErrorKind::InvalidData, "decryption failed")
        })
    }

    // check if a file is encrypted
    pub fn is_file_encrypted(content: &str) -> bool {
        if let Ok(encrypted) = serde_json::from_str::<EncryptedFile>(content) {
            encrypted.magic == MAGIC_HEADER
        } else {
            false
        }
    }


    // generate a random salt for initial encryption (16 bytes for Argon2)
    pub fn generate_salt() -> [u8; 16] {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        salt
    }
}

impl Drop for EncryptionManager {
    fn drop(&mut self) {
        self.lock();
    }
}