mod display;
mod editor;
mod utils;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{self, ClearType},
};
use std::io::{Write, stdout};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File to edit
    file_path: String,

    /// Read-only mode
    #[arg(short, long)]
    readonly: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Инициализация терминала
    terminal::enable_raw_mode()?;
    stdout().execute(terminal::EnterAlternateScreen)?;
    stdout().execute(crossterm::cursor::Hide)?;

    let result = run_editor(&args);

    // Восстановление терминала
    stdout().execute(crossterm::cursor::Show)?;
    stdout().execute(terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    result
}

fn run_editor(args: &Args) -> Result<()> {
    let mut editor = editor::HexEditor::new(&args.file_path, args.readonly)?;

    loop {
        // Очистка экрана и отрисовка
        stdout().execute(terminal::Clear(ClearType::All))?;
        editor.draw()?;
        stdout().flush()?;

        // Обработка ввода
        if let Event::Key(key) = event::read()? {
            if !handle_input(&mut editor, key)? {
                break;
            }
        }
    }

    Ok(())
}

fn handle_input(editor: &mut editor::HexEditor, key: KeyEvent) -> Result<bool> {
    match key {
        // Выход
        KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => return Ok(false),

        // Сохранение
        KeyEvent {
            code: KeyCode::Char('s'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            editor.save()?;
        }

        // Навигация
        KeyEvent {
            code: KeyCode::Up, ..
        } => editor.move_cursor_up(),

        KeyEvent {
            code: KeyCode::Down,
            ..
        } => editor.move_cursor_down(),

        KeyEvent {
            code: KeyCode::Left,
            ..
        } => editor.move_cursor_left(),

        KeyEvent {
            code: KeyCode::Right,
            ..
        } => editor.move_cursor_right(),

        KeyEvent {
            code: KeyCode::PageUp,
            ..
        } => editor.page_up(),

        KeyEvent {
            code: KeyCode::PageDown,
            ..
        } => editor.page_down(),

        KeyEvent {
            code: KeyCode::Home,
            ..
        } => editor.move_to_line_start(),

        KeyEvent {
            code: KeyCode::End, ..
        } => editor.move_to_line_end(),

        // Переключение между hex и ASCII
        KeyEvent {
            code: KeyCode::Tab, ..
        } => editor.toggle_mode(),

        // Поиск
        KeyEvent {
            code: KeyCode::Char('f'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => editor.start_search()?,

        // Переход к адресу
        KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => editor.goto_address()?,

        // Ввод hex значения
        KeyEvent {
            code: KeyCode::Char(c),
            ..
        } if c.is_ascii_hexdigit() => {
            editor.input_hex_char(c)?;
        }

        // Ввод ASCII символа (в ASCII режиме)
        KeyEvent {
            code: KeyCode::Char(c),
            ..
        } if editor.is_ascii_mode() && c.is_ascii_graphic() => {
            editor.input_ascii_char(c)?;
        }

        _ => {}
    }

    Ok(true)
}
