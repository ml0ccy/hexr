use anyhow::Result;
use crossterm::{
    ExecutableCommand, cursor,
    event::{self, Event, KeyCode},
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{Write, stdout};

pub fn get_user_input(prompt: &str) -> Result<String> {
    let (_, height) = terminal::size()?;
    stdout().execute(cursor::MoveTo(0, height - 3))?;
    stdout().execute(terminal::Clear(ClearType::CurrentLine))?;
    stdout().execute(SetForegroundColor(Color::Cyan))?;
    print!("{}", prompt);
    stdout().execute(ResetColor)?;
    stdout().flush()?;

    let mut input = String::new();

    loop {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Enter => break,
                KeyCode::Esc => return Ok(String::new()),
                KeyCode::Backspace => {
                    if !input.is_empty() {
                        input.pop();
                        stdout().execute(cursor::MoveLeft(1))?;
                        stdout().execute(Print(" "))?;
                        stdout().execute(cursor::MoveLeft(1))?;
                        stdout().flush()?;
                    }
                }
                KeyCode::Char(c) => {
                    input.push(c);
                    print!("{}", c);
                    stdout().flush()?;
                }
                _ => {}
            }
        }
    }

    Ok(input)
}

pub fn hex_string_to_bytes(hex: &str) -> Result<Vec<u8>> {
    let hex = hex.replace(" ", "");
    let mut bytes = Vec::new();

    for chunk in hex.as_bytes().chunks(2) {
        if chunk.len() == 2 {
            let s = std::str::from_utf8(chunk)?;
            let byte = u8::from_str_radix(s, 16)?;
            bytes.push(byte);
        }
    }

    Ok(bytes)
}

pub fn bytes_to_hex_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}
