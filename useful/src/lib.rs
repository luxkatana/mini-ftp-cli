pub mod prelude {
    pub type UniversalResult<T> = Result<T, Box<dyn std::error::Error>>;
    pub fn path_exists(path: &std::path::Path) -> bool {
        path.exists()
    }
    pub fn build_packet(data: String, seperator: char) -> Vec<u8> {
        let result = format!("{}{seperator}{data}", data.len());
        Vec::from(result)
    }
}
pub mod server {
    use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};

    use crate::prelude::UniversalResult;
    use std::{fs::read_dir, path::PathBuf};


    pub fn list_directory(directory: &PathBuf) -> UniversalResult<Vec<String>> {
        let mut result: Vec<String> = Vec::new();
        if directory.parent().is_some() {
            result.push("DIR_..".to_string());
        }
        let files: Vec<String> = read_dir(directory)?
            .map(|entry| {
                let mut entry = entry.unwrap().path();
                if entry.is_dir() {
                    return format!("DIR_{}", entry.as_mut_os_str().to_str().unwrap());
                }
                format!("FILE_{}", entry.as_mut_os_str().to_str().unwrap())
            })
            .collect();
        result.extend_from_slice(&files);

        Ok(result)
    }
    pub fn load_tls(
        cert_path: &str,
        pk_path: &str,
    ) -> UniversalResult<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        let certs: Vec<CertificateDer> = CertificateDer::pem_file_iter(cert_path)?
            .map(|cert| cert.unwrap())
            .collect();
        let privatekey = PrivateKeyDer::from_pem_file(pk_path)?;

        Ok((certs, privatekey))
    }
}

pub mod client {
    use std::{ffi::OsStr, path::PathBuf};

    use ratatui::{
        crossterm::event,
        layout::{Alignment, Constraint, Direction, Layout},
        style::{Color, Style},
        text::{Line, Span, Text},
        widgets::{Block, Paragraph},
        DefaultTerminal, 
    };
    use rustls::RootCertStore;
    use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::{SyntaxReference, SyntaxSet}, util::LinesWithEndings};
    use tokio::{io::AsyncReadExt, net::TcpStream};
    use tokio_rustls::client::TlsStream;

    use crate::prelude::UniversalResult;
    pub async fn calculate_packet_size(
        client: &mut TlsStream<TcpStream>,
    ) -> UniversalResult<usize> {
        Ok({
            let mut buffer: [u8; 1] = [0];
            let mut content_len = String::new();
            while buffer[0] != b'\r' {
                client.read_exact(&mut buffer).await?;
                content_len.push(buffer[0] as char);
            }
            content_len.pop();

            content_len.parse()?
        })
    }
    pub fn print_file(terminal: &mut DefaultTerminal, content: &str, filename: &std::path::Path) -> UniversalResult<()> {
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let syntax: &syntect::parsing::SyntaxReference = {
            if let Some(extension) = filename.extension() {
                let extension = extension.to_str().unwrap();
                if let Some(e) = ps.find_syntax_by_extension(extension) {
                    e
                }
                else {
                    ps.find_syntax_by_extension("txt").unwrap()
                }

            }
            else {
            ps.find_syntax_by_extension("txt").unwrap()
            }
        };
        let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
        let mut lines: Vec<Line>  = vec![];
        for line in LinesWithEndings::from(content) { // LinesWithEndings enables use of newlines mode
            let line_spans: Vec<Span> =
                h.highlight_line(line, &ps)
                .unwrap()
                .into_iter()
                .filter_map(|segment| syntect_tui::into_span(segment).ok())
         .collect();
        let line = ratatui::text::Line::from(line_spans);
        lines.push(line);
        }

        terminal.draw(|frame| {
            frame.render_widget(Text::from(lines), frame.area());
        })?;
        Ok(())
    }
    pub fn load_certificates(certificate_path: &str) -> UniversalResult<RootCertStore> {
        let mut root_cert_store = RootCertStore::empty();
        let mut certificate_path = std::io::BufReader::new(std::fs::File::open(certificate_path)?);
        for certificate in rustls_pemfile::certs(&mut certificate_path) {
            root_cert_store.add(certificate?)?;
        }
        // root_cert_store.add_parsable_certificates(rustls_pemfile::certs(&mut certificate_path).map(|cert| cert.unwrap()));

        Ok(root_cert_store)
    }

    pub fn unwrap_empty_string(data_decrypted: String, seperator: &str) -> Vec<PathBuf> {
        data_decrypted
            .split(seperator)
            .map(PathBuf::from)
            .collect()
    }
    pub fn print_directory(
        terminal: &mut DefaultTerminal,
        entries: &[PathBuf],
        currently_selected: usize,
    ) -> UniversalResult<()> {
        let mut lines: Vec<Line> = vec![];
        for (index, entry) in entries.iter().enumerate() {
            let entry_str = entry.to_str().unwrap();
            let (prefix, mut style) = if entry_str.starts_with("DIR_") {
                ("DIR_", Style::default().fg(Color::White))
            } else {
                ("FILE_", Style::default().fg(Color::Yellow))
            };
            if index == currently_selected {
                style = style.bg(Color::LightGreen).fg(Color::White);
            }
            let line = Line::styled(entry_str.strip_prefix(prefix).unwrap(), style)
                .alignment(Alignment::Left);
            lines.push(line);
        }
        terminal.draw(|frame| {
            frame.render_widget(Paragraph::new(Text::from(lines)), frame.area());
            // frame.render_widget(Paragraph::from(), frame.area());
        })?;
        Ok(())
    }

    pub fn draw_input_field(terminal: &mut DefaultTerminal, _title: Option<String>, default_val: Option<String>) -> UniversalResult<String> {
        let mut content = {
            let mut x = String::new();
            if let Some(val) = default_val {
                x = val
            }
            x
        };
        let _title = if let Some(_title) = _title {_title} else {"Input field".to_string()};
        loop {
            let mut content_clone = content.clone();
            let _title = _title.clone();
            terminal.draw(|frame| {
                // Create a layout with the input bar at the bottom
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
                    .split(frame.area());

                // Create the input bar with the current input
                let input_bar = Paragraph::new(content_clone.clone())
                    .style(Style::default().bg(Color::Green).fg(Color::White))
                    .block(Block::default().style(Style::default().bg(Color::Green)));

                // Render the input bar in the bottom chunk
                frame.render_widget(input_bar, chunks[1]);
                frame.render_widget(
                    Paragraph::new(_title).alignment(Alignment::Center),
                    frame.area(),
                );
            })?;
            if let event::Event::Key(key) = event::read()? {
                match key.code {
                    event::KeyCode::Char('q') => {
                        break;
                    },
                    event::KeyCode::Enter => {
                        break
                    }
                    event::KeyCode::Backspace => {
                        content_clone.pop();
                    },
                    event::KeyCode::Char(character) => {
                        content_clone.push(character);
                    },
                    _ => {}
                    
                }
            }
            content = content_clone;
        }
        Ok(content)
    }
    pub fn block_to_continue(text: Paragraph, terminal: &mut DefaultTerminal) -> UniversalResult<()> {
        terminal.clear()?;
        loop {
            let paragraph = text.clone();
            terminal.draw(|frame| {
                frame.render_widget(paragraph, frame.area());
            })?;
            if let event::Event::Key(_) = event::read()? {
                break
            }
        }

        Ok(())
    }
}
