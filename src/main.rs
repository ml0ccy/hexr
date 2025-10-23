mod config;
mod display;
mod editor;
mod undo_redo;
mod utils;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{self, ClearType, DisableLineWrap},
    execute,
};
use std::time::{Duration, Instant};
use std::io::{Write, stdout};
use std::sync::Mutex;

static LAST_KEY_EVENT: Mutex<Option<(KeyEvent, Instant)>> = Mutex::new(None);

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File to edit (use --new to create new file)
    file_path: String,

    /// Read-only mode
    #[arg(short, long)]
    readonly: bool,

    /// Create new file with specified size in bytes
    #[arg(short = 'n', long)]
    new: Option<usize>,

    /// Fill pattern for new file (hex value, default: 0x00)
    #[arg(short = 'p', long, default_value = "0")]
    pattern: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Инициализация терминала
    terminal::enable_raw_mode()?;
    stdout().execute(DisableLineWrap)?;
    execute!(stdout(), crossterm::terminal::Clear(ClearType::All))?;
    stdout().execute(terminal::EnterAlternateScreen)?;
    stdout().execute(crossterm::cursor::Hide)?;
    // Отключаем локальное эхо в терминале
    print!("\x1b[12l");
    stdout().flush()?;

    let result = run_editor(&args, args.new, &args.pattern);

    // Восстановление терминала
    stdout().execute(crossterm::cursor::Show)?;
    stdout().execute(terminal::LeaveAlternateScreen)?;
    // Включаем локальное эхо обратно
    print!("\x1b[12h");
    stdout().flush()?;
    terminal::disable_raw_mode()?;

    result
}

fn is_duplicate_key_event(key: &KeyEvent) -> bool {
    const DUPLICATE_THRESHOLD: Duration = Duration::from_millis(10);

    if let Ok(mut last_event) = LAST_KEY_EVENT.lock() {
        if let Some((last_key, last_time)) = *last_event {
            if last_key == *key && last_time.elapsed() < DUPLICATE_THRESHOLD {
                return true;
            }
        }
        *last_event = Some((key.clone(), Instant::now()));
        false
    } else {
        false
    }
}

fn run_editor(args: &Args, new_size: Option<usize>, pattern: &str) -> Result<()> {
    let mut editor = editor::HexEditor::new(&args.file_path, args.readonly, new_size, pattern)?;

    loop {
        // Очистка экрана и отрисовка
        stdout().execute(terminal::Clear(ClearType::FromCursorDown))?;
        editor.draw()?;
        stdout().flush()?;

        // Обработка ввода
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Проверяем на дублирование событий
                if !is_duplicate_key_event(&key) {
                    if !handle_input(&mut editor, key)? {
                        break;
                    }
                }
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

        // Undo
        KeyEvent {
            code: KeyCode::Char('z'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            let _ = editor.undo();
        }

        // Redo
        KeyEvent {
            code: KeyCode::Char('y'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            let _ = editor.redo();
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
