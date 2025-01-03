#![allow(clippy::unused_io_amount, clippy::implicit_saturating_sub)]
use crossterm::{terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}, ExecutableCommand};
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind}, layout::{Constraint, Layout}, style::{Color, Stylize}, widgets::{Block, Borders, Paragraph}, DefaultTerminal
};
use rustls::pki_types::ServerName;
use std::{
    env::current_dir, ffi::OsStr, io::Write, path::{Path, PathBuf}, sync::Arc
};
use tar::Archive;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::rustls::ClientConfig;
use tokio_rustls::TlsConnector;
use useful::{client::*, prelude::*};
const DESTINATION_ADDRESS: &str = "0.0.0.0:13360";
const CERTIFICATE_PATH: &str = "../certificates/rootCA.crt";
#[tokio::main]
async fn main() -> UniversalResult<()> {
    if let Err(err) = color_eyre::install() {
        println!("Couldn't install color_eyre: {err}");
        println!("Will be using custom panic hook, but it is not so accurate!");

        std::panic::set_hook(Box::new(|panicinfo| {
            ratatui::restore();
            let location = panicinfo.location().unwrap();
            let payload = panicinfo.payload();
            eprintln!("Fatal error - program panicked\n\ton line: {}\n\tin file: {}\n\tpayload: {:?}", location.line(), location.file(), payload);
            std::process::exit(1);
        }));
    }
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let mut terminal = ratatui::Terminal::new(backend)?;
    // let mut terminal = ratatui::init();
    terminal.clear()?;
    if let Err(error) = run(&mut terminal).await {
        terminal.clear()?;
        block_to_continue(Paragraph::new(format!("Error: {error} (press q to exit)")).blue().on_red(), &mut terminal)?;
    };
    stdout.execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
async fn run(terminal: &mut DefaultTerminal) -> UniversalResult<()> {
    let certificates = load_certificates(CERTIFICATE_PATH)?;
    let client_configuration = ClientConfig::builder()
        .with_root_certificates(certificates)
        .with_no_client_auth();

    terminal.draw(|frame| {
        frame.render_widget(
            Paragraph::new(format!("Connecting to {DESTINATION_ADDRESS}\n")).centered(),
            frame.area(),
        );
    })?;
    let connector = TlsConnector::from(Arc::new(client_configuration));
    let mut client = {
        let client = TcpStream::connect(DESTINATION_ADDRESS).await?;
        connector
            .connect(ServerName::try_from("localhost")?, client)
            .await?
    };

    terminal.draw(|frame| {
        frame.render_widget(
            Paragraph::new("Connected!\n").centered().green(),
            frame.area(),
        );
    })?;

    let mut data: Vec<u8> = vec![0; calculate_packet_size(&mut client).await?];
    client.read_exact(&mut data).await?;
    let mut entries = unwrap_empty_string(String::from_utf8(data).unwrap(), "\r");
    let mut folder_history: Vec<String> = vec![];

    let mut currently_selected: usize = 0;
    let mut pointing_to_start: usize = 0; // '..' is always first

    loop {
        let current_entry = entries.get(currently_selected).unwrap().to_str().unwrap();
        terminal.clear()?;
        print_directory(terminal, &entries, currently_selected, pointing_to_start)?;
        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char(' ') => {
                        loop {
                            let (filetype, filesize, path) = {
                                let path = {
                                let path = current_entry.strip_prefix("DIR_").unwrap_or_else(|| current_entry.strip_prefix("FILE_").unwrap());
                                let packet = build_packet(format!("FILEINFO_{path}"), '\r');
                                client.write(&packet).await?;
                                path
                                };
                                let mut data = vec![0u8; calculate_packet_size(&mut client).await?];
                                client.read_exact(&mut data).await?;
                                let data_converted = String::from_utf8(data)?;
                                let data_converted = data_converted.split("\r").map(|d| d.parse::<u64>().unwrap()).collect::<Vec<u64>>();
                                (data_converted[0], data_converted[1], path)

                            };
                            let filetypeparagraph = Paragraph::new(if filetype == 1 {"Entrytype: File"} else {"Entrytype: Folder"}).centered();
                            let filesizeparagraph = Paragraph::new(format!("Entry size: {:.2} KB", filesize / 1024)).centered();

                            terminal.draw(|frame| {
                                let center = {
                                    let horizontal_mid = Layout::new(ratatui::layout::Direction::Horizontal, vec![
                                        Constraint::Percentage(30),
                                        Constraint::Percentage(40),
                                        Constraint::Percentage(30)
                                    ]).split(frame.area())[1];
                                    let vertical_mid = Layout::new(ratatui::layout::Direction::Vertical, vec![
                                        Constraint::Percentage(30),
                                        Constraint::Percentage(40),
                                        Constraint::Percentage(30)
                                    ]).split(horizontal_mid)[1];
                                    frame.render_widget(Block::new().borders(Borders::ALL), vertical_mid);
                                    /*
                                    Entry type: Folder/File
                                    Entry size:  ... KB
                                     */
                                    Layout::new(ratatui::layout::Direction::Vertical, vec![Constraint::Percentage(100 / 3), Constraint::Percentage(100 / 3), Constraint::Percentage(100/3)]).split(vertical_mid)
                                };
                                frame.render_widget(Paragraph::new(path), center[0]);
                                frame.render_widget(filetypeparagraph, center[1]);
                                frame.render_widget(filesizeparagraph, center[2]);
                                

                            })?;
                            if let event::Event::Key(e) = event::read()? {
                                if e.kind == KeyEventKind::Press {
                                    break
                                }
                            }
                        }
                    },
                    KeyCode::Char('s') => {
                        let path = {
                            if !current_entry.starts_with("FILE_") {
                                let current_entry = format!(
                                    "SAVEDIR_{}",
                                    current_entry.strip_prefix("DIR_").unwrap()
                                );
                                let default_val = {
                                    let mut current = current_dir()?;
                                    current.push(format!(
                                        "copied_{}",
                                        Path::new(&current_entry)
                                            .file_name()
                                            .unwrap()
                                            .to_str()
                                            .unwrap()
                                    ));
                                    current.to_str().unwrap().to_string()
                                };

                                let mut path_to_receive = PathBuf::from(draw_input_field(
                                    terminal,
                                    Some("Enter path to save folder ".to_string()),
                                    Some(default_val),
                                )?);
                                if path_to_receive.parent().is_none() {
                                    block_to_continue(Paragraph::new("Invalid path"), terminal)?;
                                    continue;
                                }
                                if path_exists(&path_to_receive) {
                                    loop {
                                        terminal.draw(|frame| {
                                            frame.render_widget(Paragraph::new("There is already a folder on that place, should I delete the old folder? (press 'y' for yes or 'n' for no)").centered(), frame.area());
                                        })?;
                                        if let event::Event::Key(e) = event::read()? {
                                            if e.kind == KeyEventKind::Press
                                                && e.code == KeyCode::Char('y')
                                            {
                                                std::fs::remove_dir_all(&path_to_receive)?;
                                                break;
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                    if path_exists(&path_to_receive) {
                                        continue;
                                    }
                                }
                                terminal.clear()?;
                                terminal.draw(|frame| {
                                    frame.render_widget(
                                        Paragraph::new("Pulling from server please wait...")
                                            .centered()
                                            .yellow(),
                                        frame.area(),
                                    );
                                })?;
                                let packet = build_packet(current_entry, '\r');
                                client.write(&packet).await?;
                                let mut tarbuffer =
                                    vec![0u8; calculate_packet_size(&mut client).await?];
                                client.read_exact(&mut tarbuffer).await?;
                                std::fs::create_dir(&path_to_receive)?;
                                path_to_receive.push("filetar.tar");
                                let mut tarfile = std::fs::File::create(&path_to_receive)?;
                                tarfile.write(&tarbuffer)?;
                                tarfile.flush()?;
                                let mut archive =
                                    Archive::new(std::fs::File::open(&path_to_receive).unwrap());
                                path_to_receive.pop();
                                archive.unpack(&path_to_receive)?;
                                block_to_continue(
                                    Paragraph::new(format!(
                                        "Unpacked at location {}",
                                        path_to_receive.to_string_lossy()
                                    ))
                                    .bold()
                                    .centered()
                                    .fg(Color::Green),
                                    terminal,
                                )?;
                                continue;
                            }
                            let mut default_val = current_dir().unwrap();
                            default_val.push(
                                Path::new(current_entry.strip_prefix("FILE_").unwrap())
                                    .file_name()
                                    .unwrap(),
                            );

                            draw_input_field(
                                terminal,
                                Some("Enter file path".to_string()),
                                Some(default_val.to_str().unwrap().to_string()),
                            )?
                        };
                        if PathBuf::from(&path).parent().is_none() {
                            block_to_continue(
                                Paragraph::new("Invalid path... (Press anything to escape)"),
                                terminal,
                            )?;
                            continue;
                        }
                        let packet = build_packet(current_entry.to_string(), '\r');
                        client.write(&packet).await?;
                        let mut got: Vec<u8> = vec![0; calculate_packet_size(&mut client).await?];
                        client.read_exact(&mut got).await?;

                        std::fs::write(path, got)?;
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        {
                            let packet = build_packet("SHUTDOWN".into(), '\r');
                            client.write(&packet).await?;
                        }

                        return Ok(());
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if currently_selected == 0 && get_screen_size().1 > entries.len() as u16 {
                            currently_selected = entries.len();
                        }


                        if currently_selected > 0 {
                        currently_selected -= 1;
                        }

                        if pointing_to_start > 0 {
                            pointing_to_start -= 1;
                        }

                    },
                    KeyCode::Enter | KeyCode::Right => {
                        let packet = build_packet(current_entry.to_string(), '\r');
                        client.write(&packet).await?;
                        if let Some(current_entry) = current_entry.strip_prefix("FILE_") {
                            let filelen: usize = {
                                let mut buffer = String::new();
                                let mut current_char: [u8;1] = [0];
                                while current_char[0] != b'\r' {
                                    client.read(&mut current_char).await?;
                                    buffer.push(current_char[0] as char);
                                }
                                if buffer == "fileisbinary\r" {
                                    block_to_continue(Paragraph::new(format!("I can't show {current_entry} because it is a binary file sorry :(")).centered().bg(Color::Red), terminal)?;
                                    continue
                                }
                                buffer.pop();
                                buffer.parse().unwrap()
                            };
                            let mut filecontent =
                                vec![0; filelen];
                            client.read_exact(&mut filecontent).await?;
                            let filecontent_as_str = String::from_utf8(filecontent)?;
                            let mut pointer_to_end: u16 = 0;
                            let mut pointer_to_start: u16 = 0;
                            let screen_max_y: u16 = get_screen_size().1;
                            let amount_of_lines_file: u16 = filecontent_as_str.lines().collect::<Vec<&str>>().len() as u16;
                            if amount_of_lines_file > screen_max_y {
                                pointer_to_end = screen_max_y - 1;
                            }

                            let mut jump_to_buffer: String = String::new();
                            'inside_file: loop {
                                let mut statustext = format!("Viewing {}", Path::new(current_entry).file_name().and_then(OsStr::to_str).unwrap());
                                if event::poll(std::time::Duration::from_millis(100))? {
                                    if let event::Event::Key(key) = event::read()? {
                                    if key.kind == KeyEventKind::Press {
                                        match key.code {
                                            KeyCode::Char('g') => {
                                                let total_linecount = filecontent_as_str.lines().count();
                                                let casted = jump_to_buffer.parse::<usize>();
                                                if !jump_to_buffer.is_empty() && casted.is_ok() && casted.as_ref().unwrap() < &total_linecount  {
                                                    statustext = format!("Jumped to line {jump_to_buffer}");
                                                    pointer_to_start = casted? as u16;
                                                    pointer_to_end = pointer_to_start + screen_max_y;

                                                }
                                                else {
                                                    statustext = "ERROR: JUMP BUFFER IS EMPTY OR INVALID".to_string();
                                                }
                                                jump_to_buffer.clear();

                                            }
                                            KeyCode::Char('q') => break,
                                            KeyCode::Char('k') | KeyCode::Up => {
                                                if amount_of_lines_file > screen_max_y && pointer_to_end >= screen_max_y{
                                                        pointer_to_end -= 1;
                                                        pointer_to_start -= 1;
                                                }

                                            },
                                            KeyCode::Char('j') | KeyCode::Down =>  {
                                                if amount_of_lines_file > screen_max_y && (amount_of_lines_file - 1) > (pointer_to_end - 1){
                                                    pointer_to_end += 1;
                                                    pointer_to_start += 1
                                            }

                                            },
                                            KeyCode::Char('s') => {
                                                let filename = PathBuf::from(current_entry);
                                                let filename =
                                                    filename.file_name().unwrap().to_str().unwrap();
                                                terminal.clear()?;
                                                let path = {
                                                    let mut default_val = current_dir().unwrap();
                                                    default_val.push(Path::new(&filename));
                                                    let path = PathBuf::from(
                                                        draw_input_field(
                                                            terminal,
                                                            Some("Path".to_string()),
                                                            Some(
                                                                default_val
                                                                    .to_str()
                                                                    .unwrap()
                                                                    .to_string(),
                                                            ),
                                                        )
                                                        .unwrap(),
                                                    );
                                                    if let Some(parent) = path.parent() {
                                                        if !parent.exists() {
                                                            block_to_continue(Paragraph::new("Invalid path (press anything to escpae)").red().bold(), terminal)?;
                                                            break 'inside_file;
                                                        }
                                                    }

                                                    path
                                                };
                                                std::fs::write(path, &filecontent_as_str)?;
                                                block_to_continue(
                                                    Paragraph::new(
                                                        "File created (press anything to escape)",
                                                    )
                                                    .green(),
                                                    terminal,
                                                )?;
                                            }
                                            KeyCode::Char(key) => {
                                                if key.is_digit(10) {
                                                    jump_to_buffer.push(key);
                                                    statustext = format!("{jump_to_buffer} (press g to jump)");
                                                }
                                            },
                                            _ => {}
                                        }
                                    }
                                }
                            }
                                print_file(terminal, &filecontent_as_str, Path::new(&current_entry), pointer_to_start, pointer_to_end, statustext)?;
                            }
                        } 
                        else {
                            if entries.len() > 1 {
                                if entries[1].starts_with("FILE_") {
                                    let modified = entries[1].parent().unwrap().to_str().unwrap().to_string();
                                    let modified = modified.strip_prefix("FILE_").unwrap();
                                    folder_history.push(format!("DIR_{modified}"));
                                }
                                else {
                                folder_history.push(entries[1].parent().unwrap().to_str().unwrap().to_string());
                                }
                            }
                            let mut directories =
                                vec![0u8; calculate_packet_size(&mut client).await?];
                            client.read_exact(&mut directories).await?;
                            entries = unwrap_empty_string(String::from_utf8(directories)?, "\r");
                            currently_selected = 0;
                        }
                    },
                    KeyCode::Left => {
                        if let Some(last) = folder_history.pop() {
                            let packet = build_packet(last, '\r');
                            client.write(&packet).await?;
                            let mut content = vec![0u8; calculate_packet_size(&mut client).await?];
                            client.read_exact(&mut content).await?;
                            entries = unwrap_empty_string(String::from_utf8(content)?, "\r");

                        }
                    },
                    KeyCode::Down | KeyCode::Char('j') => {
                        currently_selected = (currently_selected + 1) % entries.len();
                        if entries.len() > get_screen_size().1 as usize  && currently_selected + 1 > (get_screen_size().1 as usize) {
                            pointing_to_start += 1;
                        }

                    },
                    _ => (),
                }
            }
        }
    }
}