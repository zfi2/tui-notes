use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::io;
use std::path::Path;
use crate::config::{Config, key_matches_any};
use crate::note::{Note, NoteManager};
use tui_textarea::TextArea;
use secrecy::{SecretString, ExposeSecret};
use chrono::Utc;

#[derive(Debug, PartialEq)]
pub enum AppMode {
    PasswordPrompt,
    PasswordSetup,
    NoteList,
    Searching,
    ViewingNote,
    EditingNote,
    CreatingNote,
    ConfirmingDelete,
    ConfirmingUnsavedExit,
    ConfirmingExport,
    SelectingExportLocation,
    EncryptedFileWarning,
}

#[derive(Debug, PartialEq)]
pub enum EditMode {
    Title,
    Content,
}

pub struct App {
    pub mode: AppMode,
    pub edit_mode: EditMode,
    pub note_manager: NoteManager,
    pub selected_note_index: usize,
    pub title_textarea: TextArea<'static>,
    pub content_textarea: TextArea<'static>,
    pub current_note_id: Option<String>,
    pub viewing_note: Option<Note>,
    pub search_query: String,
    pub search_cursor_position: usize,
    pub search_results: Vec<String>,
    pub delete_note_title: String,
    pub scroll_offset: usize,
    pub should_quit: bool,
    pub highlighting_enabled: bool,
    pub help_visible: bool,
    pub original_title: String,
    pub original_content: String,
    pub password_input: SecretString,
    pub password_error: Option<String>,
    pub password_limit_reached: bool,
    pub export_file_input: String,
    pub export_cursor_position: usize,
}

impl App {
    pub fn new(config: &Config) -> io::Result<Self> {
        let note_manager_result = NoteManager::new(&config.behavior.default_notes_file, config.behavior.encryption_enabled);
        
        let (note_manager, mode) = match note_manager_result {
            Ok(manager) => {
                let mode = if config.behavior.encryption_enabled {
                    let notes_path = Path::new(&config.behavior.default_notes_file);
                    if notes_path.exists() {
                        // check if existing file is encrypted
                        match std::fs::read_to_string(notes_path) {
                            Ok(content) if crate::encryption::EncryptionManager::is_file_encrypted(&content) => {
                                AppMode::PasswordPrompt // existing encrypted file
                            }
                            Ok(_) => {
                                AppMode::PasswordSetup // existing unencrypted file - setup encryption
                            }
                            Err(_) => {
                                AppMode::PasswordSetup // can't read file - treat as new
                            }
                        }
                    } else {
                        AppMode::PasswordSetup // no file exists
                    }
                } else {
                    AppMode::NoteList
                };
                (manager, mode)
            }
            Err(e) => {
                // check if this is the encrypted file with encryption disabled error
                if e.to_string().contains("ENCRYPTED_FILE_DETECTED") {
                    // create an empty note manager for the warning screen
                    let empty_manager = NoteManager::new("/dev/null", false)?;
                    (empty_manager, AppMode::EncryptedFileWarning)
                } else {
                    return Err(e);
                }
            }
        };
        
        Ok(App {
            mode,
            edit_mode: EditMode::Title,
            note_manager,
            selected_note_index: 0,
            title_textarea: TextArea::default(),
            content_textarea: TextArea::default(),
            current_note_id: None,
            viewing_note: None,
            search_query: String::new(),
            search_cursor_position: 0,
            search_results: Vec::new(),
            delete_note_title: String::new(),
            scroll_offset: 0,
            should_quit: false,
            highlighting_enabled: config.behavior.highlighting_enabled,
            help_visible: true,
            original_title: String::new(),
            original_content: String::new(),
            password_input: SecretString::new("".into()),
            password_error: None,
            password_limit_reached: false,
            export_file_input: String::new(),
            export_cursor_position: 0,
        })
    }

    pub fn handle_input(&mut self, key: KeyEvent, config: &Config) -> io::Result<()> {
        if config.keybindings.toggle_help.matches(key.code, key.modifiers) {
            self.help_visible = !self.help_visible;
            return Ok(());
        }
        
        if config.keybindings.manual_save.matches(key.code, key.modifiers) {
            match self.mode {
                AppMode::EditingNote => {
                    self.save_current_note()?;
                    return Ok(());
                }
                AppMode::CreatingNote => {
                    if !self.title_textarea.lines().join("").trim().is_empty() || 
                       !self.content_textarea.lines().join("").trim().is_empty() {
                        self.save_new_note()?;
                        self.return_to_list();
                    }
                    return Ok(());
                }
                _ => {} // ignore manual save in other modes
            }
        }
        
        if config.keybindings.export_plaintext.matches(key.code, key.modifiers) {
            match self.mode {
                AppMode::NoteList => {
                    self.mode = AppMode::ConfirmingExport;
                    return Ok(());
                }
                _ => {} // only allow export from note list
            }
        }

        match self.mode {
            AppMode::PasswordPrompt => self.handle_password_input(key, config),
            AppMode::PasswordSetup => self.handle_password_setup_input(key, config),
            AppMode::NoteList => self.handle_list_input(key, config),
            AppMode::Searching => self.handle_search_input(key, config),
            AppMode::ViewingNote => self.handle_viewing_input(key, config),
            AppMode::EditingNote | AppMode::CreatingNote => self.handle_editor_input(key, config),
            AppMode::ConfirmingDelete => self.handle_delete_confirmation_input(key, config),
            AppMode::ConfirmingUnsavedExit => self.handle_unsaved_exit_confirmation_input(key, config),
            AppMode::ConfirmingExport => self.handle_export_confirmation_input(key, config),
            AppMode::SelectingExportLocation => self.handle_export_location_input(key, config),
            AppMode::EncryptedFileWarning => self.handle_encrypted_file_warning_input(key, config),
        }
    }

    fn handle_password_input(&mut self, key: KeyEvent, _config: &Config) -> io::Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Enter => {
                if !self.password_input.expose_secret().is_empty() {
                    match self.note_manager.unlock_encryption(self.password_input.expose_secret()) {
                        Ok(()) => {
                            self.mode = AppMode::NoteList;
                            self.password_input = SecretString::new("".into());
                            self.password_error = None;
                        }
                        Err(e) => {
                            self.password_error = Some(e.to_string());
                            self.password_input = SecretString::new("".into());
                        }
                    }
                }
            }
            KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Backspace => {
                let mut temp = self.password_input.expose_secret().to_string();
                temp.pop();
                self.password_input = SecretString::new(temp.into());
                self.password_error = None;
                self.password_limit_reached = false;
            }
            KeyCode::Char(c) => {
                if self.password_input.expose_secret().len() < 64 {
                    let mut temp = self.password_input.expose_secret().to_string();
                    temp.push(c);
                    self.password_input = SecretString::new(temp.into());
                    self.password_error = None;
                    self.password_limit_reached = self.password_input.expose_secret().len() >= 64;
                } else {
                    self.password_limit_reached = true;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_password_setup_input(&mut self, key: KeyEvent, _config: &Config) -> io::Result<()> {
        use crossterm::event::KeyCode;
        
        match key.code {
            KeyCode::Enter => {
                if !self.password_input.expose_secret().is_empty() {
                    match self.note_manager.unlock_encryption(self.password_input.expose_secret()) {
                        Ok(()) => {
                            self.mode = AppMode::NoteList;
                            self.password_input = SecretString::new("".into());
                            self.password_error = None;
                        }
                        Err(e) => {
                            self.password_error = Some(e.to_string());
                            self.password_input = SecretString::new("".into());
                        }
                    }
                }
            }
            KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Backspace => {
                let mut temp = self.password_input.expose_secret().to_string();
                temp.pop();
                self.password_input = SecretString::new(temp.into());
                self.password_error = None;
                self.password_limit_reached = false;
            }
            KeyCode::Char(c) => {
                if self.password_input.expose_secret().len() < 64 {
                    let mut temp = self.password_input.expose_secret().to_string();
                    temp.push(c);
                    self.password_input = SecretString::new(temp.into());
                    self.password_error = None;
                    self.password_limit_reached = self.password_input.expose_secret().len() >= 64;
                } else {
                    self.password_limit_reached = true;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_list_input(&mut self, key: KeyEvent, config: &Config) -> io::Result<()> {
        let kb = &config.keybindings;
        
        if kb.quit.matches(key.code, key.modifiers) {
            self.should_quit = true;
        } else if kb.create_note.matches(key.code, key.modifiers) {
            self.start_creating_note();
        } else if kb.view_note.matches(key.code, key.modifiers) {
            self.start_viewing_selected_note();
        } else if kb.search_notes.matches(key.code, key.modifiers) {
            self.start_searching();
        } else if kb.edit_note.matches(key.code, key.modifiers) {
            self.start_editing_selected_note();
        } else if kb.delete_note.matches(key.code, key.modifiers) && config.behavior.confirm_delete {
            self.confirm_delete_selected_note();
        } else if kb.delete_note.matches(key.code, key.modifiers) && !config.behavior.confirm_delete {
            self.confirm_and_delete_note()?;
        } else if kb.move_up.matches(key.code, key.modifiers) {
            self.move_selection_up();
        } else if kb.move_down.matches(key.code, key.modifiers) {
            self.move_selection_down();
        } else if kb.toggle_pin.matches(key.code, key.modifiers) {
            self.toggle_pin_selected_note()?;
        }
        
        Ok(())
    }

    fn handle_search_input(&mut self, key: KeyEvent, config: &Config) -> io::Result<()> {
        let kb = &config.keybindings;
        
        if kb.exit_search.matches(key.code, key.modifiers) {
            self.exit_search();
        } else if kb.search_select.matches(key.code, key.modifiers) {
            if !self.search_results.is_empty() {
                self.start_viewing_filtered_note();
            }
        } else if kb.search_view.matches(key.code, key.modifiers) {
            if !self.search_results.is_empty() {
                self.start_viewing_filtered_note();
            }
        } else {
            match key.code {
                KeyCode::Backspace => {
                    if self.search_cursor_position > 0 {
                        self.search_query.remove(self.search_cursor_position - 1);
                        self.search_cursor_position -= 1;
                        self.update_search_filter();
                    }
                }
                KeyCode::Delete => {
                    if self.search_cursor_position < self.search_query.len() {
                        self.search_query.remove(self.search_cursor_position);
                        self.update_search_filter();
                    }
                }
                KeyCode::Left => {
                    if self.search_cursor_position > 0 {
                        self.search_cursor_position -= 1;
                    }
                }
                KeyCode::Right => {
                    if self.search_cursor_position < self.search_query.len() {
                        self.search_cursor_position += 1;
                    }
                }
                KeyCode::Up => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.scroll_up();
                    } else {
                        self.move_selection_up_filtered();
                    }
                }
                KeyCode::Down => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.scroll_down();
                    } else {
                        self.move_selection_down_filtered();
                    }
                }
                KeyCode::PageUp => self.page_up(),
                KeyCode::PageDown => self.page_down(),
                KeyCode::Char(c) => {
                    self.search_query.insert(self.search_cursor_position, c);
                    self.search_cursor_position += 1;
                    self.update_search_filter();
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_viewing_input(&mut self, key: KeyEvent, config: &Config) -> io::Result<()> {
        let kb = &config.keybindings;
        
        if kb.return_to_list.matches(key.code, key.modifiers) {
            self.return_to_list();
        } else if kb.edit_from_view.matches(key.code, key.modifiers) {
            self.start_editing_from_viewing();
        } else if kb.quit.matches(key.code, key.modifiers) {
            self.should_quit = true;
        } else if kb.move_up.matches(key.code, key.modifiers) {
            self.scroll_up();
        } else if kb.move_down.matches(key.code, key.modifiers) {
            self.scroll_down();
        } else if kb.page_up.matches(key.code, key.modifiers) {
            self.page_up();
        } else if kb.page_down.matches(key.code, key.modifiers) {
            self.page_down();
        }
        Ok(())
    }

    fn handle_delete_confirmation_input(&mut self, key: KeyEvent, config: &Config) -> io::Result<()> {
        let kb = &config.keybindings;
        
        if key_matches_any(&kb.confirm_delete, key.code, key.modifiers) {
            self.confirm_and_delete_note()?;
        } else if key_matches_any(&kb.cancel_delete, key.code, key.modifiers) {
            self.cancel_delete_confirmation();
        }
        Ok(())
    }

    fn handle_unsaved_exit_confirmation_input(&mut self, key: KeyEvent, config: &Config) -> io::Result<()> {
        let kb = &config.keybindings;
        
        if key_matches_any(&kb.save_and_exit_unsaved, key.code, key.modifiers) {
            self.save_current_note()?;
            self.return_to_list();
        } else if key_matches_any(&kb.discard_and_exit, key.code, key.modifiers) {
            self.return_to_list();
        } else if key_matches_any(&kb.cancel_exit, key.code, key.modifiers) {
            self.mode = AppMode::EditingNote;
        }
        Ok(())
    }

    fn handle_encrypted_file_warning_input(&mut self, key: KeyEvent, _config: &Config) -> io::Result<()> {
        // only allow quitting from this screen
        if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
            self.should_quit = true;
        }
        Ok(())
    }

    fn handle_export_confirmation_input(&mut self, key: KeyEvent, config: &Config) -> io::Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Generate default filename with timestamp
                let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
                let default_filename = format!("notes_backup_{}.json", timestamp);
                
                if config.behavior.use_native_dialog {
                    // Try to use native file dialog first
                    match std::panic::catch_unwind(|| {
                        rfd::FileDialog::new()
                            .set_title("Export Notes Backup")
                            .set_file_name(&default_filename)
                            .add_filter("JSON files", &["json"])
                            .add_filter("All files", &["*"])
                            .save_file()
                    }) {
                        Ok(Some(file_path)) => {
                            // Native dialog succeeded and user selected a path
                            if let Err(e) = self.note_manager.export_plaintext(&file_path) {
                                // TODO: Show error message in UI
                                eprintln!("Export failed: {}", e);
                            }
                            self.mode = AppMode::NoteList;
                        }
                        Ok(None) => {
                            // Native dialog succeeded but user cancelled
                            self.mode = AppMode::NoteList;
                        }
                        Err(_) => {
                            // Native dialog failed (e.g., no GUI, missing dependencies)
                            // Fall back to terminal input with home directory as default
                            self.mode = AppMode::SelectingExportLocation;
                            
                            let home_dir = dirs::home_dir()
                                .unwrap_or_else(|| std::path::PathBuf::from("."));
                            let default_path = home_dir.join(&default_filename);
                            self.export_file_input = default_path.to_string_lossy().to_string();
                            self.export_cursor_position = self.export_file_input.len();
                        }
                    }
                } else {
                    // User prefers terminal dialog - go directly to terminal input
                    self.mode = AppMode::SelectingExportLocation;
                    
                    let home_dir = dirs::home_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."));
                    let default_path = home_dir.join(&default_filename);
                    self.export_file_input = default_path.to_string_lossy().to_string();
                    self.export_cursor_position = self.export_file_input.len();
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.mode = AppMode::NoteList;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_export_location_input(&mut self, key: KeyEvent, _config: &Config) -> io::Result<()> {
        match key.code {
            KeyCode::Enter => {
                if !self.export_file_input.trim().is_empty() {
                    if let Err(e) = self.note_manager.export_plaintext(&self.export_file_input) {
                        // TODO: Show error message in UI
                        eprintln!("Export failed: {}", e);
                    }
                    self.export_file_input.clear();
                    self.export_cursor_position = 0;
                    self.mode = AppMode::NoteList;
                }
            }
            KeyCode::Esc => {
                self.export_file_input.clear();
                self.export_cursor_position = 0;
                self.mode = AppMode::NoteList;
            }
            KeyCode::Backspace => {
                if self.export_cursor_position > 0 {
                    self.export_file_input.remove(self.export_cursor_position - 1);
                    self.export_cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                if self.export_cursor_position < self.export_file_input.len() {
                    self.export_file_input.remove(self.export_cursor_position);
                }
            }
            KeyCode::Left => {
                if self.export_cursor_position > 0 {
                    self.export_cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.export_cursor_position < self.export_file_input.len() {
                    self.export_cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.export_cursor_position = 0;
            }
            KeyCode::End => {
                self.export_cursor_position = self.export_file_input.len();
            }
            KeyCode::Char(c) => {
                self.export_file_input.insert(self.export_cursor_position, c);
                self.export_cursor_position += 1;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_editor_input(&mut self, key: KeyEvent, config: &Config) -> io::Result<()> {
        let kb = &config.keybindings;
        
        if kb.save_and_exit.matches(key.code, key.modifiers) {
            match self.mode {
                AppMode::EditingNote => {
                    if !config.behavior.auto_save && self.has_unsaved_changes() {
                        self.mode = AppMode::ConfirmingUnsavedExit;
                    } else {
                        if !config.behavior.auto_save {
                            self.save_current_note()?;
                        }
                        self.return_to_list();
                    }
                }
                AppMode::CreatingNote => {
                    if !self.title_textarea.lines().join("").trim().is_empty() || 
                       !self.content_textarea.lines().join("").trim().is_empty() {
                        self.save_new_note()?;
                    }
                    self.return_to_list();
                }
                _ => {}
            }
        } else if kb.switch_field.matches(key.code, key.modifiers) {
            self.edit_mode = match self.edit_mode {
                EditMode::Title => EditMode::Content,
                EditMode::Content => EditMode::Title,
            };
        } else if kb.title_to_content.matches(key.code, key.modifiers) && self.edit_mode == EditMode::Title {
            self.edit_mode = EditMode::Content;
        } else if kb.toggle_highlighting.matches(key.code, key.modifiers) {
            self.highlighting_enabled = !self.highlighting_enabled;
        } else {
            let text_changed = match self.edit_mode {
                EditMode::Title => {
                    let old_content = self.title_textarea.lines().join("");
                    self.title_textarea.input(key);
                    let new_content = self.title_textarea.lines().join("");
                    old_content != new_content
                }
                EditMode::Content => {
                    let old_content = self.content_textarea.lines().join("\n");
                    self.content_textarea.input(key);
                    let new_content = self.content_textarea.lines().join("\n");
                    old_content != new_content
                }
            };
            
            if text_changed && config.behavior.auto_save && self.mode == AppMode::EditingNote && self.current_note_id.is_some() {
                if let Err(_) = self.save_current_note() {
                    // if saving fails just keep typing
                }
            }
        }
        Ok(())
    }

    fn start_creating_note(&mut self) {
        self.mode = AppMode::CreatingNote;
        self.edit_mode = EditMode::Title;
        self.title_textarea = TextArea::default();
        self.content_textarea = TextArea::default();
        self.current_note_id = None;
        self.viewing_note = None;
        self.scroll_offset = 0;
    }

    fn start_searching(&mut self) {
        self.mode = AppMode::Searching;
        self.search_query.clear();
        self.search_cursor_position = 0;
        self.selected_note_index = 0;
        self.update_search_filter();
    }

    fn exit_search(&mut self) {
        self.mode = AppMode::NoteList;
        self.search_query.clear();
        self.search_cursor_position = 0;
        self.search_results.clear();
        self.selected_note_index = 0;
    }

    fn update_search_filter(&mut self) {
        let search_notes = self.note_manager.search_notes(&self.search_query);
        self.search_results = search_notes.iter().map(|note| note.id.clone()).collect();
        
        if self.selected_note_index >= self.search_results.len() && !self.search_results.is_empty() {
            self.selected_note_index = 0;
        }
    }

    fn move_selection_up_filtered(&mut self) {
        if self.selected_note_index > 0 {
            self.selected_note_index -= 1;
        }
    }

    fn move_selection_down_filtered(&mut self) {
        if self.selected_note_index < self.search_results.len().saturating_sub(1) {
            self.selected_note_index += 1;
        }
    }

    fn start_viewing_filtered_note(&mut self) {
        if let Some(note_id) = self.search_results.get(self.selected_note_index) {
            let all_notes = self.note_manager.get_all_notes();
            if let Some(note) = all_notes.iter().find(|n| &n.id == note_id) {
                self.mode = AppMode::ViewingNote;
                self.viewing_note = Some((*note).clone());
                self.current_note_id = Some(note.id.clone());
                self.scroll_offset = 0;
            }
        }
    }

    fn start_viewing_selected_note(&mut self) {
        let notes = self.note_manager.get_all_notes();
        if let Some(note) = notes.get(self.selected_note_index) {
            self.mode = AppMode::ViewingNote;
            self.viewing_note = Some((*note).clone());
            self.current_note_id = Some(note.id.clone());
            self.scroll_offset = 0;
        }
    }

    fn start_editing_from_viewing(&mut self) {
        if let Some(note) = &self.viewing_note {
            self.mode = AppMode::EditingNote;
            self.edit_mode = EditMode::Title;
            self.title_textarea = TextArea::from(vec![note.title.clone()]);
            self.content_textarea = TextArea::from(note.content.lines().map(|s| s.to_string()).collect::<Vec<_>>());
            self.original_title = note.title.clone();
            self.original_content = note.content.clone();
        }
    }

    fn start_editing_selected_note(&mut self) {
        let notes = self.note_manager.get_all_notes();
        if let Some(note) = notes.get(self.selected_note_index) {
            self.mode = AppMode::EditingNote;
            self.edit_mode = EditMode::Title;
            self.title_textarea = TextArea::from(vec![note.title.clone()]);
            self.content_textarea = TextArea::from(note.content.lines().map(|s| s.to_string()).collect::<Vec<_>>());
            self.current_note_id = Some(note.id.clone());
            self.viewing_note = None;
            self.scroll_offset = 0;
            self.original_title = note.title.clone();
            self.original_content = note.content.clone();
        }
    }

    fn confirm_delete_selected_note(&mut self) {
        let notes = self.note_manager.get_all_notes();
        if let Some(note) = notes.get(self.selected_note_index) {
            self.delete_note_title = note.title.clone();
            self.mode = AppMode::ConfirmingDelete;
        }
    }

    fn confirm_and_delete_note(&mut self) -> io::Result<()> {
        let notes = self.note_manager.get_all_notes();
        if let Some(note) = notes.get(self.selected_note_index) {
            let id = note.id.clone();
            self.note_manager.delete_note(&id);
            self.note_manager.save_notes()?;
            
            let new_count = self.note_manager.get_all_notes().len();
            if self.selected_note_index >= new_count && new_count > 0 {
                self.selected_note_index = new_count - 1;
            }
        }
        self.cancel_delete_confirmation();
        Ok(())
    }

    fn cancel_delete_confirmation(&mut self) {
        self.mode = AppMode::NoteList;
        self.delete_note_title.clear();
    }

    fn move_selection_up(&mut self) {
        if self.selected_note_index > 0 {
            self.selected_note_index -= 1;
        }
    }

    fn move_selection_down(&mut self) {
        let notes = self.note_manager.get_all_notes();
        if self.selected_note_index < notes.len().saturating_sub(1) {
            self.selected_note_index += 1;
        }
    }

    fn save_current_note(&mut self) -> io::Result<()> {
        if let Some(id) = &self.current_note_id {
            if let Some(note) = self.note_manager.get_note_mut(id) {
                let title = self.title_textarea.lines().join("");
                let content = self.content_textarea.lines().join("\n");
                note.update_title(title);
                note.update_content(content);
            }
        }
        self.note_manager.save_notes()
    }

    fn save_new_note(&mut self) -> io::Result<()> {
        let title_text = self.title_textarea.lines().join("");
        let content_text = self.content_textarea.lines().join("\n");
        
        let title = if title_text.trim().is_empty() {
            content_text
                .lines()
                .next()
                .unwrap_or("Untitled")
                .to_string()
        } else {
            title_text
        };

        self.note_manager.add_note(title, content_text);
        self.note_manager.save_notes()
    }

    fn return_to_list(&mut self) {
        self.mode = AppMode::NoteList;
        self.edit_mode = EditMode::Title;
        self.title_textarea = TextArea::default();
        self.content_textarea = TextArea::default();
        self.current_note_id = None;
        self.viewing_note = None;
        self.scroll_offset = 0;
    }

    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }

    fn page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(10);
    }

    fn page_down(&mut self) {
        self.scroll_offset += 10;
    }

    fn toggle_pin_selected_note(&mut self) -> io::Result<()> {
        let notes = self.note_manager.get_all_notes();
        if let Some(note) = notes.get(self.selected_note_index) {
            let id = note.id.clone();
            if let Some(note_mut) = self.note_manager.get_note_mut(&id) {
                note_mut.toggle_pin();
            }
            self.note_manager.save_notes()?;
        }
        Ok(())
    }

    fn has_unsaved_changes(&self) -> bool {
        let current_title = self.title_textarea.lines().join("");
        let current_content = self.content_textarea.lines().join("\n");
        current_title != self.original_title || current_content != self.original_content
    }


    pub fn get_notes(&mut self) -> Vec<&Note> {
        self.note_manager.get_all_notes()
    }

    pub fn get_search_results(&mut self) -> Vec<&Note> {
        let all_notes = self.note_manager.get_all_notes();
        self.search_results
            .iter()
            .filter_map(|id| {
                all_notes.iter().find(|note| &note.id == id).copied()
            })
            .collect()
    }

}