use crate::config::Config;
use crate::editor::{EditMode, HexEditor};
use anyhow::Result;
use crossterm::{
    ExecutableCommand, cursor, execute,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{BufWriter, Stdout, Write, stdout};

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

        // Используем буферизированный вывод для уменьшения мерцания
        let mut stdout = BufWriter::new(stdout());

        // Перемещаемся в начало, но НЕ очищаем весь экран
        execute!(stdout, cursor::MoveTo(0, 0))?;

        // Отрисовка компонентов
        self.draw_header_buffered(&mut stdout, editor)?;
        self.draw_content_buffered(&mut stdout, editor)?;
        self.draw_status_bar_buffered(&mut stdout, editor)?;

        // Сбрасываем буфер один раз
        stdout.flush()?;
        Ok(())
    }

    fn draw_header_buffered(
        &self,
        stdout: &mut BufWriter<Stdout>,
        editor: &HexEditor,
    ) -> Result<()> {
        execute!(stdout, cursor::MoveTo(0, 0))?;
        execute!(stdout, SetBackgroundColor(Color::DarkBlue))?;
        execute!(stdout, SetForegroundColor(Color::White))?;

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

        write!(stdout, "{:width$}", header, width = self.width as usize)?;
        execute!(stdout, ResetColor)?;

        // Динамический заголовок колонок
        execute!(stdout, cursor::MoveTo(0, 2))?;
        execute!(stdout, SetForegroundColor(Color::DarkGrey))?;
        write!(stdout, "  Offset  ")?;

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
            write!(stdout, "{:02X} ", i)?;
        }
        write!(stdout, "  ASCII")?;
        execute!(stdout, ResetColor)?;

        Ok(())
    }

    fn draw_content_buffered(
        &self,
        stdout: &mut BufWriter<Stdout>,
        editor: &HexEditor,
    ) -> Result<()> {
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
            execute!(stdout, cursor::MoveTo(0, y as u16))?;

            let offset = view_offset + line_idx * bytes_per_line;
            if offset >= data.len() {
                // Очищаем оставшиеся строки
                execute!(stdout, Clear(ClearType::CurrentLine))?;
                continue;
            }

            // Адрес
            execute!(stdout, SetForegroundColor(Color::Yellow))?;
            write!(stdout, "{:08X}  ", offset)?;
            execute!(stdout, ResetColor)?;

            // Hex данные
            for byte_idx in 0..bytes_per_line {
                let pos = offset + byte_idx;

                if pos < data.len() {
                    // Подсветка курсора
                    if pos == cursor_pos && mode == EditMode::Hex {
                        execute!(stdout, SetBackgroundColor(Color::DarkGreen))?;
                        execute!(stdout, SetForegroundColor(Color::White))?;
                    }

                    write!(stdout, "{:02X} ", data[pos])?;
                    execute!(stdout, ResetColor)?;
                } else {
                    write!(stdout, "   ")?;
                }
            }

            write!(stdout, " ")?;

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
                        execute!(stdout, SetBackgroundColor(Color::DarkGreen))?;
                        execute!(stdout, SetForegroundColor(Color::White))?;
                    }

                    write!(stdout, "{}", ch)?;
                    execute!(stdout, ResetColor)?;
                } else {
                    write!(stdout, " ")?;
                }
            }

            // Очищаем остаток строки
            execute!(stdout, Clear(ClearType::UntilNewLine))?;
        }

        Ok(())
    }

    fn draw_status_bar_buffered(
        &self,
        stdout: &mut BufWriter<Stdout>,
        editor: &HexEditor,
    ) -> Result<()> {
        let y = self.height - 1;
        execute!(stdout, cursor::MoveTo(0, y))?;
        execute!(stdout, SetBackgroundColor(Color::DarkGrey))?;
        execute!(stdout, SetForegroundColor(Color::White))?;

        let cursor_pos = editor.get_cursor_pos();
        let file_size = editor.get_data().len();
        let mode_str = match editor.get_mode() {
            EditMode::Hex => "HEX",
            EditMode::Ascii => "ASCII",
        };

        let status = format!(
            " Pos: 0x{:08X} ({}/{}) | Mode: {} | Ctrl+Q: Quit | Ctrl+S: Save | Ctrl+Z: Undo | Ctrl+Y: Redo | Tab: Switch Mode ",
            cursor_pos, cursor_pos, file_size, mode_str
        );

        write!(stdout, "{:width$}", status, width = self.width as usize)?;
        execute!(stdout, ResetColor)?;

        Ok(())
    }

    pub fn get_visible_lines(&self) -> usize {
        // Высота минус: заголовок (1), пустая строка (1), заголовок колонок (1), статус бар (1)
        (self.height as usize).saturating_sub(4)
    }
}
