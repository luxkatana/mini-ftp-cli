use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind}, style::{Color, Stylize}, widgets::Paragraph, DefaultTerminal
};
use rustls::pki_types::ServerName;
use std::{
    env::current_dir, io::Write, path::{Path, PathBuf}, sync::Arc
};
use tar::Archive;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::rustls::ClientConfig;
use tokio_rustls::TlsConnector;
use useful::{
    client::{block_to_continue, load_certificates, print_directory, unwrap_empty_string},
    server::path_exists,
};
use useful::{
    client::{calculate_packet_size, draw_input_field, print_file},
    prelude::UniversalResult,
    server::build_packet,
};
const DESTINATION_ADDRESS: &str = "0.0.0.0:13360";
const CERTIFICATE_PATH: &str = "../certificates/rootCA.crt";
#[tokio::main]
async fn main() -> UniversalResult<()> {
    let mut terminal = ratatui::init();
    terminal.clear()?;
    match run(&mut terminal).await {
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
            .connect(ServerName::try_from("0.0.0.0")?, client)
            .await?
    };

    terminal.draw(|frame| {
        frame.render_widget(
            Paragraph::new(format!("Connected!\n")).centered().green(),
            frame.area(),
        );
    })?;

    let mut data: Vec<u8> = vec![0; calculate_packet_size(&mut client).await?];
    client.read(&mut data).await?;
    let mut entries = unwrap_empty_string(String::from_utf8(data).unwrap(), "\r");
    let mut folder_history: Vec<String> = vec![];

    let mut currently_selected: usize = 0;

    loop {
        let current_entry = entries.get(currently_selected).unwrap().to_str().unwrap();
        terminal.clear()?;
        // block_to_continue(Paragraph::new(format!("{:?}", entries)), terminal)?;
        print_directory(terminal, &entries, currently_selected)?;
        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
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
                        client.read(&mut got).await?;

                        std::fs::write(path, got)?;
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(());
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if currently_selected == 0 {
                            currently_selected = entries.len();
                        }
                        currently_selected -= 1;
                    }
                    KeyCode::Enter | KeyCode::Right => {
                        let packet = build_packet(current_entry.to_string(), '\r');
                        client.write(&packet).await?;
                        if current_entry.starts_with("FILE_") {
                            let current_entry = current_entry.strip_prefix("FILE_").unwrap();
                            let mut filecontent =
                                vec![0; calculate_packet_size(&mut client).await?];
                            client.read_exact(&mut filecontent).await?;
                            let filecontent_as_str =
                                String::from_utf8_lossy(&filecontent).to_string();
                            
                            'inside_file: loop {
                                print_file(terminal, &filecontent_as_str)?;
                                if let event::Event::Key(key) = event::read()? {
                                    if key.kind == KeyEventKind::Press {
                                        match key.code {
                                            KeyCode::Char('q') => break,
                                            KeyCode::Char('s') => {
                                                let filename = PathBuf::from(current_entry);
                                                let filename =
                                                    filename.file_name().unwrap().to_str().unwrap();
                                                terminal.clear()?;
                                                let path = loop {
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

                                                    break path;
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
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        } else {
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
                    }
                    KeyCode::Left => {
                        if let Some(last) = folder_history.pop() {
                            let packet = build_packet(last, '\r');
                            client.write(&packet).await?;
                            let mut content = vec![0u8; calculate_packet_size(&mut client).await?];
                            client.read_exact(&mut content).await?;
                            entries = unwrap_empty_string(String::from_utf8(content)?, "\r");


                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        currently_selected = (currently_selected + 1) % entries.len();
                    }
                    _ => (),
                }
            }
        }
    }
}
