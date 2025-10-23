use crate::editor::{EditMode, HexEditor};
use anyhow::Result;
use crossterm::{
    ExecutableCommand, cursor,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal,
};
use std::io::stdout;

pub struct Display {
    width: u16,
    height: u16,
}

impl Display {
    pub fn new() -> Result<Self> {
        let (width, height) = terminal::size()?;
        Ok(Self { width, height })
    }

    pub fn draw(&self, editor: &HexEditor) -> Result<()> {
        self.draw_header(editor)?;
        self.draw_content(editor)?;
        if editor.get_config().display.show_status_bar {
            self.draw_status_bar(editor)?;
        }
        self.draw_help()?;
        Ok(())
    }

    fn draw_header(&self, editor: &HexEditor) -> Result<()> {
        // Курсор уже позиционирован в (0,0) в main.rs
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

        // Заголовок колонок
        stdout().execute(cursor::MoveTo(0, 2))?;
        stdout().execute(SetForegroundColor(Color::DarkGrey))?;
        print!("  Offset  ");

        for i in 0..16 {
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
        let bytes_per_line = editor.get_bytes_per_line();
        let mode = editor.get_mode();

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
        let y = self.height - 2;
        stdout().execute(cursor::MoveTo(0, y))?;
        stdout().execute(SetBackgroundColor(Color::DarkGrey))?;
        stdout().execute(SetForegroundColor(Color::White))?;

        let cursor_pos = editor.get_cursor_pos();
        let data_len = editor.get_data().len();
        let mode = match editor.get_mode() {
            EditMode::Hex => "HEX",
            EditMode::Ascii => "ASCII",
        };

        let undo_status = if editor.can_undo() { "✓" } else { "✗" };
        let redo_status = if editor.can_redo() { "✓" } else { "✗" };

        let status = format!(
            " Mode: {} | Pos: 0x{:08X}/{:08X} ({:.1}%) | Undo:{} Redo:{} ",
            mode,
            cursor_pos,
            data_len,
            (cursor_pos as f64 / data_len as f64) * 100.0,
            undo_status,
            redo_status
        );

        print!("{:width$}", status, width = self.width as usize);
        stdout().execute(ResetColor)?;

        Ok(())
    }

    fn draw_help(&self) -> Result<()> {
        let y = self.height - 1;
        stdout().execute(cursor::MoveTo(0, y))?;
        stdout().execute(SetForegroundColor(Color::DarkGrey))?;

        print!("^Q:Quit ^S:Save ^Z:Undo ^Y:Redo ^F:Find ^G:Goto Tab:Mode 0-9A-F:Edit Hex a-z:Edit ASCII ↑↓←→:Navigate PgUp/PgDn:Page");
        stdout().execute(ResetColor)?;

        Ok(())
    }

    pub fn get_visible_lines(&self) -> usize {
        (self.height - 5) as usize // Вычитаем заголовок, статус бар и помощь
    }
}
