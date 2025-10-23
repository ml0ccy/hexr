mod config;
mod display;
mod editor;
mod undo_redo;
mod utils;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{self, ClearType, DisableLineWrap},
};
use std::io::stdout;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the file to edit
    file_path: Option<String>,

    /// Create a new file with specified size in bytes
    #[arg(short = 'n', long)]
    new: Option<usize>,

    /// Fill pattern for new file (hex value, default: 0x00)
    #[arg(short = 'f', long, default_value = "0")]
    fill: String,

    /// Open file in read-only mode
    #[arg(short, long)]
    readonly: bool,

    /// Number of bytes per line (default: 16)
    #[arg(short = 'w', long, default_value = "16")]
    bytes_per_line: usize,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Загрузка конфигурации
    let config = config::Config::load();

    // Запуск редактора
    run_editor(args, config)?;

    Ok(())
}

fn run_editor(args: Args, config: config::Config) -> Result<()> {
    // Инициализация терминала
    terminal::enable_raw_mode()?;
    stdout()
        .execute(terminal::EnterAlternateScreen)?
        .execute(terminal::Clear(ClearType::All))?
        .execute(DisableLineWrap)?;

    let result = (|| -> Result<()> {
        // Создание редактора
        let mut editor = if let Some(size) = args.new {
            // Создаем новый файл с указанным размером
            let file_path = args.file_path.unwrap_or_else(|| "untitled".to_string());
            let mut editor = editor::HexEditor::new_with_size(&args.fill, size, config.clone())?;
            editor.file_path = file_path; // Устанавливаем имя файла
            editor
        } else if let Some(file_path) = args.file_path {
            // Открываем существующий файл
            editor::HexEditor::open(&file_path, args.readonly, config.clone())?
        } else {
            // Создаем пустой файл
            editor::HexEditor::new(config.clone())?
        };

        // Создание display
        let mut display = display::Display::new()?;

        // Основной цикл
        loop {
            display.draw(&editor)?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    // КРИТИЧНО: обрабатываем только события нажатия клавиш
                    if key.kind == KeyEventKind::Press {
                        if !handle_input(&mut editor, key)? {
                            break;
                        }
                    }
                }
            }

            // Обработка auto-save
            if config.editor.auto_save {
                editor.check_auto_save()?;
            }
        }

        Ok(())
    })();

    // Восстановление терминала
    terminal::disable_raw_mode()?;
    stdout().execute(terminal::LeaveAlternateScreen)?;

    result
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
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } if c.is_ascii_hexdigit() => {
            editor.input_hex_char(c)?;
        }

        // Ввод ASCII символа (в ASCII режиме)
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } if editor.is_ascii_mode() && c.is_ascii_graphic() => {
            editor.input_ascii_char(c)?;
        }

        // Вставка hex строки (Ctrl+I)
        KeyEvent {
            code: KeyCode::Char('i'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => editor.insert_from_hex_input()?,

        // Вставка ASCII строки (Ctrl+V)
        KeyEvent {
            code: KeyCode::Char('v'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => editor.insert_from_ascii_input()?,

        // Вставка байта 0xFF (Ctrl+Insert)
        KeyEvent {
            code: KeyCode::Insert,
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            editor.insert_byte(0xFF)?;
        }

        // Вставка байта 0x00 (Insert key)
        KeyEvent {
            code: KeyCode::Insert,
            ..
        } => {
            editor.insert_byte(0x00)?;
        }

        _ => {}
    }

    Ok(true)
}
