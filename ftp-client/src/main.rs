#![allow(unused_variables)]
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    style::Stylize,
    widgets::Paragraph,
    DefaultTerminal,
};
use std::path::PathBuf;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};
use useful::client::{print_directory, unwrap_empty_string};
use useful::prelude::*;
const DESTINATION_ADDRESS: &str = "0.0.0.0:13360";
fn main() -> UniversalResult<()> {
    let mut terminal = ratatui::init();
    terminal.clear()?;
    match run(&mut terminal) {
        Ok(_) => (),
        Err(error) => {
            terminal.clear()?;
            loop {
                terminal.draw(|frame| {
                    let paragraph = Paragraph::new(format!("Error: {error} (press q to exit)"))
                        .blue()
                        .on_red();
                    frame.render_widget(paragraph, frame.area());
                })?;
                if let event::Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                        break;
                    }
                }
            }
        }
    };
    ratatui::restore();
    Ok(())
}
fn run(terminal: &mut DefaultTerminal) -> UniversalResult<()> {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64() as usize;
    terminal.draw(|frame| {
        frame.render_widget(
            Paragraph::new(format!("Connecting to {DESTINATION_ADDRESS}\n")).centered(),
            frame.area(),
        );
    })?;
    let mut client = TcpStream::connect(DESTINATION_ADDRESS)?;
    terminal.draw(|frame| {
        frame.render_widget(
            Paragraph::new(format!("Connected!\n")).centered().green(),
            frame.area(),
        );
    })?;

    let (private_key, public_key, random_generator) = generate_keypair(time)?;
    client.write(format!("{time}").as_bytes())?;
    let content_length: usize = {
        let mut current_char: [u8; 1] = [0];
        let mut buffer = String::new();
        while current_char[0] != 0x9 {
            client.read(&mut current_char)?;
            buffer.push(current_char[0] as char);
        }
        buffer.pop();
        buffer.parse().unwrap()
    };
    let mut data: Vec<u8> = vec![0; content_length];
    while data.contains(&0) {

    }
    
    client.read(&mut data)?;
    let decrypted = decrypt_packet(&private_key, data)?;
    let entries = unwrap_empty_string(decrypted, "\r");
    terminal.draw(|frame| {
        frame.render_widget(
            Paragraph::new(
                entries
                    .iter()
                    .map(|e| e.as_os_str().to_str().unwrap())
                    .collect::<Vec<&str>>()
                    .join("\n"),
            ),
            frame.area(),
        );
    })?;
    let mut currently_selected: usize = 0;
    let mut currently_path: PathBuf = PathBuf::from(".");

    loop {
        terminal.clear()?;
        print_directory(terminal, &entries, currently_selected)?;
        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        terminal.clear()?;
                        terminal.draw(|frame| frame.render_widget(Paragraph::new("Closing... Goodbye!").centered().bold(), frame.area()))?;
                        return Ok(());
                    },
                    KeyCode::Up => {
                        if currently_selected == 0 {
                            currently_selected = entries.len();
                        }
                        currently_selected -= 1;
                    },
                    KeyCode::Enter => {

                    }
                    KeyCode::Down => {
                        currently_selected =  (currently_selected + 1) % entries.len();
                    }
                    _ => (),
                }
            }
        }
    }
}
