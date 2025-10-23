use crate::display::Display;
use crate::utils;
use anyhow::{Result, bail};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditMode {
    Hex,
    Ascii,
}

pub struct HexEditor {
    file_path: String,
    data: Vec<u8>,
    original_data: Vec<u8>,
    cursor_pos: usize,
    view_offset: usize,
    mode: EditMode,
    readonly: bool,
    modified: bool,
    bytes_per_line: usize,
    half_byte: Option<u8>,
    display: Display,
}

impl HexEditor {
    pub fn new(file_path: &str, readonly: bool) -> Result<Self> {
        let mut file = File::open(file_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let original_data = data.clone();
        let display = Display::new()?;

        Ok(Self {
            file_path: file_path.to_string(),
            data,
            original_data,
            cursor_pos: 0,
            view_offset: 0,
            mode: EditMode::Hex,
            readonly,
            modified: false,
            bytes_per_line: 16,
            half_byte: None,
            display,
        })
    }

    pub fn draw(&self) -> Result<()> {
        self.display.draw(self)
    }

    pub fn save(&mut self) -> Result<()> {
        if self.readonly {
            bail!("File is opened in read-only mode");
        }

        if !self.modified {
            return Ok(());
        }

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.file_path)?;

        file.write_all(&self.data)?;
        file.flush()?;

        self.original_data = self.data.clone();
        self.modified = false;

        Ok(())
    }

    pub fn move_cursor_up(&mut self) {
        if self.cursor_pos >= self.bytes_per_line {
            self.cursor_pos -= self.bytes_per_line;
            self.adjust_view();
        }
    }

    pub fn move_cursor_down(&mut self) {
        if self.cursor_pos + self.bytes_per_line < self.data.len() {
            self.cursor_pos += self.bytes_per_line;
            self.adjust_view();
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.adjust_view();
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.data.len() - 1 {
            self.cursor_pos += 1;
            self.adjust_view();
        }
    }

    pub fn page_up(&mut self) {
        let lines_per_page = self.display.get_visible_lines();
        let jump = lines_per_page * self.bytes_per_line;

        if self.cursor_pos > jump {
            self.cursor_pos -= jump;
        } else {
            self.cursor_pos = 0;
        }
        self.adjust_view();
    }

    pub fn page_down(&mut self) {
        let lines_per_page = self.display.get_visible_lines();
        let jump = lines_per_page * self.bytes_per_line;

        self.cursor_pos = (self.cursor_pos + jump).min(self.data.len() - 1);
        self.adjust_view();
    }

    pub fn move_to_line_start(&mut self) {
        self.cursor_pos = (self.cursor_pos / self.bytes_per_line) * self.bytes_per_line;
    }

    pub fn move_to_line_end(&mut self) {
        let line_start = (self.cursor_pos / self.bytes_per_line) * self.bytes_per_line;
        let line_end = (line_start + self.bytes_per_line - 1).min(self.data.len() - 1);
        self.cursor_pos = line_end;
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            EditMode::Hex => EditMode::Ascii,
            EditMode::Ascii => EditMode::Hex,
        };
        self.half_byte = None;
    }

    pub fn input_hex_char(&mut self, c: char) -> Result<()> {
        if self.readonly || self.mode != EditMode::Hex {
            return Ok(());
        }

        let value = c.to_digit(16).unwrap() as u8;

        if let Some(high) = self.half_byte {
            // Второй полубайт
            self.data[self.cursor_pos] = (high << 4) | value;
            self.modified = true;
            self.half_byte = None;

            if self.cursor_pos < self.data.len() - 1 {
                self.cursor_pos += 1;
            }
        } else {
            // Первый полубайт
            self.half_byte = Some(value);
        }

        Ok(())
    }

    pub fn input_ascii_char(&mut self, c: char) -> Result<()> {
        if self.readonly || self.mode != EditMode::Ascii {
            return Ok(());
        }

        self.data[self.cursor_pos] = c as u8;
        self.modified = true;

        if self.cursor_pos < self.data.len() - 1 {
            self.cursor_pos += 1;
        }

        Ok(())
    }

    pub fn start_search(&mut self) -> Result<()> {
        // Упрощенная версия поиска
        let pattern = utils::get_user_input("Search (hex): ")?;
        let bytes = utils::hex_string_to_bytes(&pattern)?;

        if let Some(pos) = self.find_pattern(&bytes, self.cursor_pos + 1) {
            self.cursor_pos = pos;
            self.adjust_view();
        }

        Ok(())
    }

    pub fn goto_address(&mut self) -> Result<()> {
        let input = utils::get_user_input("Go to address (hex): ")?;
        let address = usize::from_str_radix(&input, 16)?;

        if address < self.data.len() {
            self.cursor_pos = address;
            self.adjust_view();
        }

        Ok(())
    }

    fn find_pattern(&self, pattern: &[u8], start: usize) -> Option<usize> {
        if pattern.is_empty() {
            return None;
        }

        for i in start..self.data.len() - pattern.len() + 1 {
            if &self.data[i..i + pattern.len()] == pattern {
                return Some(i);
            }
        }
        None
    }

    fn adjust_view(&mut self) {
        let visible_lines = self.display.get_visible_lines();
        let cursor_line = self.cursor_pos / self.bytes_per_line;
        let view_line = self.view_offset / self.bytes_per_line;

        if cursor_line < view_line {
            self.view_offset = cursor_line * self.bytes_per_line;
        } else if cursor_line >= view_line + visible_lines {
            self.view_offset = (cursor_line - visible_lines + 1) * self.bytes_per_line;
        }
    }

    // Getters для display
    pub fn get_data(&self) -> &[u8] {
        &self.data
    }
    pub fn get_cursor_pos(&self) -> usize {
        self.cursor_pos
    }
    pub fn get_view_offset(&self) -> usize {
        self.view_offset
    }
    pub fn get_mode(&self) -> EditMode {
        self.mode
    }
    pub fn is_modified(&self) -> bool {
        self.modified
    }
    pub fn get_bytes_per_line(&self) -> usize {
        self.bytes_per_line
    }
    pub fn is_ascii_mode(&self) -> bool {
        self.mode == EditMode::Ascii
    }
    pub fn get_file_path(&self) -> &str {
        &self.file_path
    }
    pub fn is_readonly(&self) -> bool {
        self.readonly
    }
}
