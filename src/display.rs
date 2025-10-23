use crate::config::Config;
use crate::editor::{EditMode, HexEditor};
use anyhow::Result;
use crossterm::{
    cursor, execute, ExecutableCommand,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{Write, stdout};

pub struct Display {
    width: u16,
    height: u16,
    config: Config,
}

impl Display {
    pub fn new() -> Result<Self> {
        let (width, height) = terminal::size()?;
        Ok(Self {
            width,
            height,
            config: Config::default(),
        })
    }

    pub fn draw(&mut self, editor: &HexEditor) -> Result<()> {
        // Обновление размеров терминала
        let (width, height) = terminal::size()?;
        self.width = width;
        self.height = height;

        // Очистка экрана
        execute!(stdout(), Clear(ClearType::All))?;

        // Отрисовка компонентов
        self.draw_header(editor)?;
        self.draw_content(editor)?;
        self.draw_status_bar(editor)?;

        stdout().flush()?;
        Ok(())
    }

    fn draw_header(&self, editor: &HexEditor) -> Result<()> {
        stdout().execute(cursor::MoveTo(0, 0))?;
        stdout().execute(SetBackgroundColor(Color::DarkBlue))?;
        stdout().execute(SetForegroundColor(Color::White))?;

        let header = format!(
            " HEX EDITOR - {} {} {} {}",
            editor.get_file_path(),
            if editor.is_modified() {
                "[Modified]"
            } else {
                ""
            },
            if editor.is_readonly() {
                "[Read-Only]"
            } else {
                ""
            },
            if editor.is_new_file() {
                "[New File]"
            } else {
                ""
            }
        );

        print!("{:width$}", header, width = self.width as usize);
        stdout().execute(ResetColor)?;

        // Динамический заголовок колонок
        stdout().execute(cursor::MoveTo(0, 2))?;
        stdout().execute(SetForegroundColor(Color::DarkGrey))?;
        print!("  Offset  ");

        // Расчет динамического количества байтов на строку
        let available_width = self.width as usize;
        let offset_width = 10;
        let ascii_label_width = 8;
        let separator_width = 2;

        let bytes_per_line = ((available_width
            .saturating_sub(offset_width + separator_width + ascii_label_width))
            / 4)
        .max(8)
        .min(32);

        for i in 0..bytes_per_line {
            print!("{:02X} ", i);
        }
        print!("  ASCII");
        stdout().execute(ResetColor)?;

        Ok(())
    }

    fn draw_content(&self, editor: &HexEditor) -> Result<()> {
        let data = editor.get_data();
        let cursor_pos = editor.get_cursor_pos();
        let view_offset = editor.get_view_offset();
        let mode = editor.get_mode();

        // Динамический расчет bytes_per_line на основе ширины терминала
        let available_width = self.width as usize;
        let offset_width = 10;
        let ascii_label_width = 8;
        let separator_width = 2;

        let bytes_per_line = ((available_width
            .saturating_sub(offset_width + separator_width + ascii_label_width))
            / 4)
        .max(8)
        .min(32);

        let visible_lines = self.get_visible_lines();

        for line_idx in 0..visible_lines {
            let y = 3 + line_idx;
            stdout().execute(cursor::MoveTo(0, y as u16))?;

            let offset = view_offset + line_idx * bytes_per_line;
            if offset >= data.len() {
                break;
            }

            // Адрес
            stdout().execute(SetForegroundColor(Color::Yellow))?;
            print!("{:08X}  ", offset);
            stdout().execute(ResetColor)?;

            // Hex данные
            for byte_idx in 0..bytes_per_line {
                let pos = offset + byte_idx;

                if pos < data.len() {
                    // Подсветка курсора
                    if pos == cursor_pos && mode == EditMode::Hex {
                        stdout().execute(SetBackgroundColor(Color::DarkGreen))?;
                        stdout().execute(SetForegroundColor(Color::White))?;
                    }

                    print!("{:02X} ", data[pos]);
                    stdout().execute(ResetColor)?;
                } else {
                    print!("   ");
                }
            }

            print!(" ");

            // ASCII представление
            for byte_idx in 0..bytes_per_line {
                let pos = offset + byte_idx;

                if pos < data.len() {
                    let byte = data[pos];
                    let ch = if byte.is_ascii_graphic() || byte == b' ' {
                        byte as char
                    } else {
                        '.'
                    };

                    // Подсветка курсора
                    if pos == cursor_pos && mode == EditMode::Ascii {
                        stdout().execute(SetBackgroundColor(Color::DarkGreen))?;
                        stdout().execute(SetForegroundColor(Color::White))?;
                    }

                    print!("{}", ch);
                    stdout().execute(ResetColor)?;
                } else {
                    print!(" ");
                }
            }
        }

        Ok(())
    }

    fn draw_status_bar(&self, editor: &HexEditor) -> Result<()> {
        let y = self.height - 1;
        stdout().execute(cursor::MoveTo(0, y))?;
        stdout().execute(SetBackgroundColor(Color::DarkGrey))?;
        stdout().execute(SetForegroundColor(Color::White))?;

        let cursor_pos = editor.get_cursor_pos();
        let file_size = editor.get_data().len();
        let mode_str = match editor.get_mode() {
            EditMode::Hex => "HEX",
            EditMode::Ascii => "ASCII",
        };

        let status = format!(
            " Pos: 0x{:08X} ({}/{}) | Mode: {} | Ctrl+Q: Quit | Ctrl+S: Save | Ctrl+Z: Undo | Ctrl+Y: Redo | Ctrl+I: Insert Hex | Ctrl+V: Insert ASCII | Ins: Insert 0x00 ",
            cursor_pos, cursor_pos, file_size, mode_str
        );

        print!("{:width$}", status, width = self.width as usize);
        stdout().execute(ResetColor)?;

        Ok(())
    }

    pub fn get_visible_lines(&self) -> usize {
        // Высота минус: заголовок (1), пустая строка (1), заголовок колонок (1), статус бар (1)
        (self.height as usize).saturating_sub(4)
    }
}
