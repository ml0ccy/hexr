use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub editor: EditorConfig,
    pub display: DisplayConfig,
    pub colors: ColorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub bytes_per_line: usize,
    pub tab_size: usize,
    pub auto_save: bool,
    pub auto_save_interval: u64, // в секундах
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub show_line_numbers: bool,
    pub show_ascii: bool,
    pub highlight_current_line: bool,
    pub show_status_bar: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    pub selection: String,
    pub header: String,
    pub status_bar: String,
    pub modified_indicator: String,
}


impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            bytes_per_line: 16,
            tab_size: 4,
            auto_save: false,
            auto_save_interval: 30,
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: true,
            show_ascii: true,
            highlight_current_line: true,
            show_status_bar: true,
        }
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            background: "black".to_string(),
            foreground: "white".to_string(),
            cursor: "green".to_string(),
            selection: "blue".to_string(),
            header: "blue".to_string(),
            status_bar: "grey".to_string(),
            modified_indicator: "red".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::get_config_path();

        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => {
                    match toml::from_str(&content) {
                        Ok(config) => return config,
                        Err(e) => {
                            eprintln!("Warning: Failed to parse config file: {}", e);
                            eprintln!("Using default configuration.");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to read config file: {}", e);
                    eprintln!("Using default configuration.");
                }
            }
        } else {
            // Создаем конфигурационный файл с настройками по умолчанию
            if let Err(e) = Self::create_default_config() {
                eprintln!("Warning: Failed to create default config file: {}", e);
            }
        }

        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::get_config_path();
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    fn get_config_path() -> PathBuf {
        let mut path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."));
        path.push("hexr");
        path.push("config.toml");
        path
    }

    fn create_default_config() -> anyhow::Result<()> {
        let config = Self::default();
        let config_path = Self::get_config_path();

        // Создаем директорию если её нет
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        config.save()?;
        println!("Created default config file at: {:?}", config_path);
        Ok(())
    }

}
