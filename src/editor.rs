use crate::config::Config;
use crate::display::Display;
use crate::undo_redo::{EditOperation, UndoRedoStack};
use crate::utils;
use anyhow::{Result, bail};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditMode {
    Hex,
    Ascii,
}

pub struct HexEditor {
    pub file_path: String,
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
    undo_redo_stack: UndoRedoStack,
    config: Config,
    is_new_file: bool,
}

impl HexEditor {
    pub fn new(config: Config) -> Result<Self> {
        Self::new_with_size("0", 0, config)
    }

    pub fn new_with_size(fill_pattern: &str, size: usize, config: Config) -> Result<Self> {
        // Парсим fill pattern из hex строки
        let fill_byte = if fill_pattern.starts_with("0x") || fill_pattern.starts_with("0X") {
            u8::from_str_radix(&fill_pattern[2..], 16).unwrap_or(0)
        } else {
            u8::from_str_radix(fill_pattern, 16).unwrap_or(0)
        };

        // Создаем данные с указанным размером и заполнителем
        let data = vec![fill_byte; size];
        let original_data = data.clone();

        let display = Display::new()?;

        Ok(Self {
            file_path: "untitled".to_string(),
            data,
            original_data,
            cursor_pos: 0,
            view_offset: 0,
            mode: EditMode::Hex,
            readonly: false,
            modified: size > 0, // Если размер > 0, то файл считается измененным
            bytes_per_line: config.editor.bytes_per_line,
            half_byte: None,
            display,
            undo_redo_stack: UndoRedoStack::default(),
            config,
            is_new_file: true,
        })
    }

    pub fn open(file_path: &str, readonly: bool, config: Config) -> Result<Self> {
        // Открываем существующий файл
        let mut file = File::open(file_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let display = Display::new()?;

        Ok(Self {
            file_path: file_path.to_string(),
            data: data.clone(),
            original_data: data,
            cursor_pos: 0,
            view_offset: 0,
            mode: EditMode::Hex,
            readonly,
            modified: false,
            bytes_per_line: config.editor.bytes_per_line,
            half_byte: None,
            display,
            undo_redo_stack: UndoRedoStack::default(),
            config,
            is_new_file: false,
        })
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
            .create(true)
            .truncate(true)
            .open(&self.file_path)?;

        file.write_all(&self.data)?;
        file.flush()?;

        self.original_data = self.data.clone();
        self.modified = false;
        self.undo_redo_stack.clear(); // Очищаем историю после сохранения

        Ok(())
    }

    pub fn undo(&mut self) -> Result<()> {
        if self.readonly {
            bail!("Cannot undo in read-only mode");
        }

        if let Some(operation) = self.undo_redo_stack.undo() {
            operation.undo(&mut self.data);
            self.modified = true;
        }
        Ok(())
    }

    pub fn redo(&mut self) -> Result<()> {
        if self.readonly {
            bail!("Cannot redo in read-only mode");
        }

        if let Some(operation) = self.undo_redo_stack.redo() {
            operation.redo(&mut self.data);
            self.modified = true;
        }
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        self.undo_redo_stack.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.undo_redo_stack.can_redo()
    }

    pub fn get_config(&self) -> &Config {
        &self.config
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
        } else if self.cursor_pos < self.data.len() {
            // Перемещаемся к концу файла
            self.cursor_pos = self.data.len().saturating_sub(1);
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
        if self.cursor_pos < self.data.len().saturating_sub(1) {
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

        self.cursor_pos = (self.cursor_pos + jump).min(self.data.len().saturating_sub(1));
        self.adjust_view();
    }

    pub fn move_to_line_start(&mut self) {
        self.cursor_pos = (self.cursor_pos / self.bytes_per_line) * self.bytes_per_line;
    }

    pub fn move_to_line_end(&mut self) {
        let line_start = (self.cursor_pos / self.bytes_per_line) * self.bytes_per_line;
        let line_end = (line_start + self.bytes_per_line).saturating_sub(1).min(self.data.len().saturating_sub(1));
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
            if self.cursor_pos >= self.data.len() {
                self.half_byte = None;
                return Ok(());
            }

            let old_value = self.data[self.cursor_pos];
            let new_value = (high << 4) | value;
            self.data[self.cursor_pos] = new_value;
            self.modified = true;
            self.half_byte = None;

            // Сохраняем операцию для undo/redo
            self.undo_redo_stack.push(EditOperation::new_replace_byte(self.cursor_pos, old_value, new_value));

            if self.cursor_pos + 1 < self.data.len() {
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

        if self.cursor_pos >= self.data.len() {
            return Ok(());
        }

        let old_value = self.data[self.cursor_pos];
        let new_value = c as u8;
        self.data[self.cursor_pos] = new_value;
        self.modified = true;

        // Сохраняем операцию для undo/redo
        self.undo_redo_stack.push(EditOperation::new_replace_byte(self.cursor_pos, old_value, new_value));

        if self.cursor_pos + 1 < self.data.len() {
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

        if input.trim().is_empty() {
            return Ok(());
        }

        match usize::from_str_radix(&input, 16) {
            Ok(address) => {
                if address < self.data.len() {
                    self.cursor_pos = address;
                    self.adjust_view();
                } else {
                    // Адрес за пределами файла - переходим к концу
                    self.cursor_pos = self.data.len().saturating_sub(1);
                    self.adjust_view();
                }
            }
            Err(_) => {
                // Неверный формат адреса - игнорируем
            }
        }

        Ok(())
    }

    fn find_pattern(&self, pattern: &[u8], start: usize) -> Option<usize> {
        if pattern.is_empty() {
            return None;
        }

        let data_len = self.data.len();
        let pattern_len = pattern.len();

        if pattern_len > data_len {
            return None;
        }

        (start..=data_len.saturating_sub(pattern_len))
            .find(|&i| &self.data[i..i + pattern_len] == pattern)
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

    pub fn is_new_file(&self) -> bool {
        self.is_new_file
    }

    pub fn check_auto_save(&mut self) -> Result<()> {
        if self.config.editor.auto_save && self.modified && !self.readonly {
            self.save()?;
        }
        Ok(())
    }

    pub fn insert_byte(&mut self, value: u8) -> Result<()> {
        if self.readonly {
            bail!("Cannot insert in read-only mode");
        }

        let position = self.cursor_pos;

        // Вставляем байт в текущую позицию курсора
        self.data.insert(position, value);
        self.modified = true;

        // Сохраняем операцию для undo/redo
        self.undo_redo_stack.push(EditOperation::new_insert_byte(position, value));

        // Перемещаем курсор на следующую позицию
        if position < self.data.len() - 1 {
            self.cursor_pos += 1;
        }

        self.adjust_view();
        Ok(())
    }

    pub fn insert_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        if self.readonly {
            bail!("Cannot insert in read-only mode");
        }

        if bytes.is_empty() {
            return Ok(());
        }

        let position = self.cursor_pos;

        // Вставляем байты в текущую позицию курсора
        for (i, &byte) in bytes.iter().enumerate() {
            self.data.insert(position + i, byte);
        }
        self.modified = true;

        // Сохраняем операцию для undo/redo
        self.undo_redo_stack.push(EditOperation::new_insert_bytes(position, bytes.to_vec()));

        // Перемещаем курсор в конец вставленного блока
        self.cursor_pos = position + bytes.len();

        self.adjust_view();
        Ok(())
    }

    pub fn insert_from_hex_string(&mut self, hex_string: &str) -> Result<()> {
        let bytes = utils::hex_string_to_bytes(hex_string)?;
        self.insert_bytes(&bytes)
    }

    pub fn insert_from_ascii_string(&mut self, ascii_string: &str) -> Result<()> {
        let bytes: Vec<u8> = ascii_string.as_bytes().to_vec();
        self.insert_bytes(&bytes)
    }

    pub fn insert_from_hex_input(&mut self) -> Result<()> {
        let input = utils::get_user_input("Insert hex bytes: ")?;

        if input.trim().is_empty() {
            return Ok(());
        }

        self.insert_from_hex_string(&input)
    }

    pub fn insert_from_ascii_input(&mut self) -> Result<()> {
        let input = utils::get_user_input("Insert ASCII text: ")?;

        if input.trim().is_empty() {
            return Ok(());
        }

        self.insert_from_ascii_string(&input)
    }
}
