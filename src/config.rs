use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub keybindings: KeyBindings,
    pub colors: ColorTheme,
    pub behavior: Behavior,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyBindings {
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub quit: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub create_note: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub edit_note: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub view_note: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub delete_note: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub search_notes: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub move_up: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub move_down: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub save_and_exit: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub switch_field: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub title_to_content: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub edit_from_view: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub return_to_list: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub page_up: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub page_down: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub exit_search: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub search_select: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub search_view: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding_vec", deserialize_with = "deserialize_keybinding_vec")]
    pub confirm_delete: Vec<KeyBinding>,
    #[serde(serialize_with = "serialize_keybinding_vec", deserialize_with = "deserialize_keybinding_vec")]
    pub cancel_delete: Vec<KeyBinding>,
    #[serde(serialize_with = "serialize_keybinding_vec", deserialize_with = "deserialize_keybinding_vec")]
    pub save_and_exit_unsaved: Vec<KeyBinding>,
    #[serde(serialize_with = "serialize_keybinding_vec", deserialize_with = "deserialize_keybinding_vec")]
    pub discard_and_exit: Vec<KeyBinding>,
    #[serde(serialize_with = "serialize_keybinding_vec", deserialize_with = "deserialize_keybinding_vec")]
    pub cancel_exit: Vec<KeyBinding>,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub toggle_highlighting: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub toggle_pin: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub toggle_help: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub manual_save: KeyBinding,
    #[serde(serialize_with = "serialize_keybinding", deserialize_with = "deserialize_keybinding")]
    pub export_plaintext: KeyBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    pub key: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub ctrl: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub alt: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub shift: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorTheme {
    #[serde(serialize_with = "serialize_color", deserialize_with = "deserialize_color")]
    pub title_bar: ColorConfig,
    #[serde(serialize_with = "serialize_color", deserialize_with = "deserialize_color")]
    pub border_active: ColorConfig,
    #[serde(serialize_with = "serialize_color", deserialize_with = "deserialize_color")]
    pub border_inactive: ColorConfig,
    #[serde(serialize_with = "serialize_color", deserialize_with = "deserialize_color")]
    pub text: ColorConfig,
    #[serde(serialize_with = "serialize_color", deserialize_with = "deserialize_color")]
    pub text_secondary: ColorConfig,
    #[serde(serialize_with = "serialize_color", deserialize_with = "deserialize_color")]
    pub text_highlight: ColorConfig,
    #[serde(serialize_with = "serialize_color", deserialize_with = "deserialize_color")]
    pub background_selected: ColorConfig,
    #[serde(serialize_with = "serialize_color", deserialize_with = "deserialize_color")]
    pub search_border: ColorConfig,
    #[serde(serialize_with = "serialize_color", deserialize_with = "deserialize_color")]
    pub help_text: ColorConfig,
    #[serde(serialize_with = "serialize_color", deserialize_with = "deserialize_color")]
    pub delete_dialog_border: ColorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    #[serde(default = "default_color", skip_serializing_if = "is_reset")]
    pub fg: String,
    #[serde(default = "default_color", skip_serializing_if = "is_reset")]
    pub bg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Behavior {
    pub default_notes_file: String,
    pub auto_save: bool,
    pub search_case_sensitive: bool,
    pub confirm_delete: bool,
    pub max_events_per_frame: usize,
    pub ui_timeout_ms: u64,
    pub show_line_numbers: bool,
    pub highlighting_enabled: bool,
    pub encryption_enabled: bool,
    pub use_native_dialog: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            keybindings: KeyBindings::default(),
            colors: ColorTheme::default(),
            behavior: Behavior::default(),
        }
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        KeyBindings {
            quit: KeyBinding::new("q"),
            create_note: KeyBinding::new("n"),
            edit_note: KeyBinding::new("Enter"),
            view_note: KeyBinding::new("v"),
            delete_note: KeyBinding::new("Delete"),
            search_notes: KeyBinding::new("/"),
            move_up: KeyBinding::new("Up"),
            move_down: KeyBinding::new("Down"),
            save_and_exit: KeyBinding::new("Esc"),
            switch_field: KeyBinding::new("Tab"),
            title_to_content: KeyBinding::new("Enter"),
            edit_from_view: KeyBinding::new("e"),
            return_to_list: KeyBinding::new("Esc"),
            page_up: KeyBinding::new("PageUp"),
            page_down: KeyBinding::new("PageDown"),
            exit_search: KeyBinding::new("Esc"),
            search_select: KeyBinding::new("Enter"),
            search_view: KeyBinding::new("v"),
            confirm_delete: vec![KeyBinding::new("y"), KeyBinding::new("Y")],
            cancel_delete: vec![KeyBinding::new("n"), KeyBinding::new("N"), KeyBinding::new("Esc")],
            save_and_exit_unsaved: vec![KeyBinding::new("s"), KeyBinding::new("S")],
            discard_and_exit: vec![KeyBinding::new("d"), KeyBinding::new("D")],
            cancel_exit: vec![KeyBinding::new("c"), KeyBinding::new("C"), KeyBinding::new("Esc")],
            toggle_highlighting: KeyBinding { key: "h".to_string(), ctrl: true, alt: false, shift: false },
            toggle_pin: KeyBinding::new("p"),
            toggle_help: KeyBinding::new("F5"),
            manual_save: KeyBinding { key: "s".to_string(), ctrl: true, alt: false, shift: false },
            export_plaintext: KeyBinding { key: "e".to_string(), ctrl: true, alt: false, shift: false },
        }
    }
}

impl Default for ColorTheme {
    fn default() -> Self {
        ColorTheme {
            title_bar: ColorConfig { fg: "Cyan".to_string(), bg: "Reset".to_string() },
            border_active: ColorConfig { fg: "Yellow".to_string(), bg: "Reset".to_string() },
            border_inactive: ColorConfig { fg: "White".to_string(), bg: "Reset".to_string() },
            text: ColorConfig { fg: "White".to_string(), bg: "Reset".to_string() },
            text_secondary: ColorConfig { fg: "Gray".to_string(), bg: "Reset".to_string() },
            text_highlight: ColorConfig { fg: "White".to_string(), bg: "Reset".to_string() },
            background_selected: ColorConfig { fg: "Reset".to_string(), bg: "DarkGray".to_string() },
            search_border: ColorConfig { fg: "Cyan".to_string(), bg: "Reset".to_string() },
            help_text: ColorConfig { fg: "Yellow".to_string(), bg: "Reset".to_string() },
            delete_dialog_border: ColorConfig { fg: "Red".to_string(), bg: "DarkGray".to_string() },
        }
    }
}

impl Default for Behavior {
    fn default() -> Self {
        let default_notes_file = Config::config_dir()
            .map(|dir| dir.join("notes.json").to_string_lossy().to_string())
            .unwrap_or_else(|_| "notes.json".to_string());

        Behavior {
            default_notes_file,
            auto_save: true,
            search_case_sensitive: false,
            confirm_delete: true,
            max_events_per_frame: 50,
            ui_timeout_ms: 100,
            show_line_numbers: false,
            highlighting_enabled: true,
            encryption_enabled: false,
            use_native_dialog: true,
        }
    }
}

impl KeyBinding {
    pub fn new(key: &str) -> Self {
        KeyBinding {
            key: key.to_string(),
            ctrl: false,
            alt: false,
            shift: false,
        }
    }


    pub fn matches(&self, key_code: KeyCode, modifiers: KeyModifiers) -> bool {
        let expected_modifiers = KeyModifiers::from_bits_truncate(
            (if self.ctrl { KeyModifiers::CONTROL.bits() } else { 0 }) |
            (if self.alt { KeyModifiers::ALT.bits() } else { 0 }) |
            (if self.shift { KeyModifiers::SHIFT.bits() } else { 0 })
        );

        if modifiers != expected_modifiers {
            return false;
        }

        match self.key.as_str() {
            "Enter" => key_code == KeyCode::Enter,
            "Esc" => key_code == KeyCode::Esc,
            "Tab" => key_code == KeyCode::Tab,
            "Backspace" => key_code == KeyCode::Backspace,
            "Delete" => key_code == KeyCode::Delete,
            "Up" => key_code == KeyCode::Up,
            "Down" => key_code == KeyCode::Down,
            "Left" => key_code == KeyCode::Left,
            "Right" => key_code == KeyCode::Right,
            "PageUp" => key_code == KeyCode::PageUp,
            "PageDown" => key_code == KeyCode::PageDown,
            "Home" => key_code == KeyCode::Home,
            "End" => key_code == KeyCode::End,
            "F1" => key_code == KeyCode::F(1),
            "F2" => key_code == KeyCode::F(2),
            "F3" => key_code == KeyCode::F(3),
            "F4" => key_code == KeyCode::F(4),
            "F5" => key_code == KeyCode::F(5),
            "F6" => key_code == KeyCode::F(6),
            "F7" => key_code == KeyCode::F(7),
            "F8" => key_code == KeyCode::F(8),
            "F9" => key_code == KeyCode::F(9),
            "F10" => key_code == KeyCode::F(10),
            "F11" => key_code == KeyCode::F(11),
            "F12" => key_code == KeyCode::F(12),
            key if key.len() == 1 => {
                if let Some(c) = key.chars().next() {
                    key_code == KeyCode::Char(c)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}


fn default_color() -> String {
    "Reset".to_string()
}

fn is_false(b: &bool) -> bool {
    !b
}

fn is_reset(s: &str) -> bool {
    s == "Reset"
}


// helper function to set secure permissions on unix systems
#[cfg(unix)]
fn set_secure_permissions(path: &std::path::Path, is_directory: bool) -> io::Result<()> {
    let mode = if is_directory { 0o700 } else { 0o600 };
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(mode);
    fs::set_permissions(path, perms)?;
    Ok(())
}

impl Config {
    pub fn load() -> io::Result<Self> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let contents = fs::read_to_string(&config_path)?;
        let config: Config = match toml::from_str::<Config>(&contents) {
            Ok(config) => {
                config.save()?;
                config
            },
            Err(e) => {
                eprintln!("Warning: Config file has missing or invalid fields: {}", e);
                eprintln!("Creating updated config file with defaults for missing fields...");
                
                let default_config = Config::default();
                default_config.save()?;
                eprintln!("Config file has been updated. Your existing settings have been preserved where possible.");
                
                default_config
            }
        };

        Ok(config)
    }

    pub fn save(&self) -> io::Result<()> {
        let config_path = Self::config_path()?;
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
            // set secure permissions on the config directory
            set_secure_permissions(parent, true)?;
        }

        let contents = toml::to_string_pretty(self).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize config: {}", e),
            )
        })?;

        fs::write(&config_path, contents)?;
        // set secure permissions on the config file
        set_secure_permissions(&config_path, false)?;
        Ok(())
    }

    pub fn config_dir() -> io::Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .or_else(|| dirs::home_dir().map(|p| p.join(".config")))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Could not determine config directory",
                )
            })?;

        Ok(config_dir.join("tui-notes"))
    }

    fn config_path() -> io::Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }
}

pub fn key_matches_any(keybindings: &[KeyBinding], key_code: KeyCode, modifiers: KeyModifiers) -> bool {
    keybindings.iter().any(|kb| kb.matches(key_code, modifiers))
}

fn serialize_keybinding<S>(kb: &KeyBinding, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    if !kb.ctrl && !kb.alt && !kb.shift {
        serializer.serialize_str(&kb.key)
    } else {
        use serde::Serialize;
        kb.serialize(serializer)
    }
}

fn deserialize_keybinding<'de, D>(deserializer: D) -> Result<KeyBinding, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::{de::Error, Deserialize};
    use serde_json::Value;
    
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(KeyBinding::new(&s)),
        _ => KeyBinding::deserialize(value).map_err(D::Error::custom),
    }
}

fn serialize_keybinding_vec<S>(kbs: &[KeyBinding], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    
    if kbs.iter().all(|kb| !kb.ctrl && !kb.alt && !kb.shift) {
        let keys: Vec<&str> = kbs.iter().map(|kb| kb.key.as_str()).collect();
        keys.serialize(serializer)
    } else {
        kbs.serialize(serializer)
    }
}

fn deserialize_keybinding_vec<'de, D>(deserializer: D) -> Result<Vec<KeyBinding>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::{de::Error, Deserialize};
    use serde_json::Value;
    
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Array(arr) => {
            let mut result = Vec::new();
            for item in arr {
                match item {
                    Value::String(s) => result.push(KeyBinding::new(&s)),
                    _ => result.push(KeyBinding::deserialize(item).map_err(D::Error::custom)?),
                }
            }
            Ok(result)
        }
        _ => Err(D::Error::custom("expected array")),
    }
}

fn serialize_color<S>(color: &ColorConfig, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    
    if color.bg == "Reset" && color.fg != "Reset" {
        serializer.serialize_str(&color.fg)
    } else if color.fg == "Reset" && color.bg != "Reset" {
        let mut map = std::collections::HashMap::new();
        map.insert("bg", &color.bg);
        map.serialize(serializer)
    } else if color.fg != "Reset" && color.bg != "Reset" {
        let mut map = std::collections::HashMap::new();
        map.insert("fg", &color.fg);
        map.insert("bg", &color.bg);
        map.serialize(serializer)
    } else {
        serializer.serialize_str("Reset")
    }
}

fn deserialize_color<'de, D>(deserializer: D) -> Result<ColorConfig, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::{de::Error, Deserialize};
    use serde_json::Value;
    
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(ColorConfig {
            fg: s,
            bg: "Reset".to_string(),
        }),
        Value::Object(obj) => {
            let fg = obj.get("fg").and_then(|v| v.as_str()).unwrap_or("Reset").to_string();
            let bg = obj.get("bg").and_then(|v| v.as_str()).unwrap_or("Reset").to_string();
            Ok(ColorConfig { fg, bg })
        }
        _ => Err(D::Error::custom("expected string or object")),
    }
}

impl ColorConfig {
    pub fn to_color(&self) -> Color {
        parse_color(&self.fg)
    }
    
    pub fn to_bg_color(&self) -> Color {
        parse_color(&self.bg)
    }
}

fn parse_color(color_str: &str) -> Color {
    match color_str {
        "Reset" => Color::Reset,
        "Black" => Color::Black,
        "Red" => Color::Red,
        "Green" => Color::Green,
        "Yellow" => Color::Yellow,
        "Blue" => Color::Blue,
        "Magenta" => Color::Magenta,
        "Cyan" => Color::Cyan,
        "Gray" | "Grey" => Color::Gray,
        "DarkGray" | "DarkGrey" => Color::DarkGray,
        "LightRed" => Color::LightRed,
        "LightGreen" => Color::LightGreen,
        "LightYellow" => Color::LightYellow,
        "LightBlue" => Color::LightBlue,
        "LightMagenta" => Color::LightMagenta,
        "LightCyan" => Color::LightCyan,
        "White" => Color::White,
        _ if color_str.starts_with('#') && color_str.len() == 7 => {
            if let Ok(hex) = u32::from_str_radix(&color_str[1..], 16) {
                let r = ((hex >> 16) & 0xFF) as u8;
                let g = ((hex >> 8) & 0xFF) as u8;
                let b = (hex & 0xFF) as u8;
                Color::Rgb(r, g, b)
            } else {
                Color::White
            }
        },
        _ => {
            if let Ok(index) = color_str.parse::<u8>() {
                Color::Indexed(index)
            } else {
                Color::White
            }
        }
    }
}