pub mod prelude {
    use std::io::{Read, Write};
    pub type UniversalResult<T> = Result<T, Box<dyn std::error::Error>>;
    use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::net::TcpStream;
    use rand::prelude::*;
    pub fn handshake(client: &mut TcpStream) -> UniversalResult<(RsaPrivateKey, RsaPublicKey, StdRng)> {
        let now_local = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as usize;
        let client_time: String = {
            let mut client_time: [u8; 15] = [0;15];
            client.read(&mut client_time)?;
            String::from_utf8(client_time.to_vec())?
        }.chars().filter(|c| (*c as u8) >= 48 && (*c as u8) <= 57).collect::<String>();
        let client_time_parsed: usize = match client_time.parse() {
            Ok(t) => t,
            Err(_) => {
                client.write(&[1])?;
                panic!("CLient did not follow the protocol properly[1]")

            }

        };

        if (now_local - client_time_parsed) > 10 {
            todo!("latency")
        }



        Ok(generate_keypair(client_time_parsed)?)

    }

    pub fn generate_keypair(time: usize) -> Result<(RsaPrivateKey, RsaPublicKey, StdRng), Box<dyn std::error::Error>> {
        let mut rand_generator = StdRng::seed_from_u64(time as u64);
        let private_key = RsaPrivateKey::new(&mut rand_generator, 2048)?;
        let public_key = RsaPublicKey::from(&private_key);



        Ok((private_key, public_key, rand_generator))


    }
    pub fn build_packet(publickey: &RsaPublicKey, rng: &mut StdRng, data: String) -> UniversalResult<Vec<u8>> {
        let encoded = publickey.encrypt(rng, Pkcs1v15Encrypt, data.as_bytes())?;
        let mut packet = Vec::from(format!("{}\t", encoded.len()));
        packet.extend_from_slice(&encoded);
        Ok(packet)
    }
    pub fn decrypt_packet(privatekey: &RsaPrivateKey, encrypted: Vec<u8>) -> UniversalResult<String> {
        let content = privatekey.decrypt(Pkcs1v15Encrypt, &encrypted)?;
        Ok(String::from_utf8(content).unwrap())

    }

}
pub mod server {
    use crate::prelude::UniversalResult;
    use std::{fs::{read_dir, FileType}, path::PathBuf};

    pub fn list_directory(directory: &PathBuf) -> UniversalResult<Vec<String>> {
        let mut result: Vec<String> = Vec::new();
        if directory.parent().is_some() {
            result.push("..".to_string());
        }
        let files: Vec<String> = read_dir(directory)?
            .map(|entry| {
                let mut entry = entry.unwrap().path();
                if entry.is_dir() {
                    return format!("DIR_{}", entry.as_mut_os_str().to_str().unwrap());
                }
                format!("FILE_{}", entry.as_mut_os_str().to_str().unwrap())
                

            }).collect();
        result.extend_from_slice(&files);

        Ok(result)
    }
}
pub mod client {
    use std::path::PathBuf;

    use ratatui::{crossterm::style::Stylize, layout::Alignment, style::{Color, Modifier, Style}, text::{Line, Span, Text}, widgets::Paragraph, DefaultTerminal, Frame};

    use crate::prelude::UniversalResult;

    pub fn unwrap_empty_string(data_decrypted: String, seperator: &str) -> Vec<PathBuf> {
        data_decrypted.split(seperator).map(|entry| PathBuf::from(entry)).collect()
    }
    pub fn print_directory(terminal: &mut DefaultTerminal, entries: &Vec<PathBuf>, currently_selected: usize) -> UniversalResult<()> {
        let mut lines: Vec<Line> = vec![];
        for (index, entry) in entries.iter().enumerate() {
            let entry_str = entry.to_str().unwrap();
            let (prefix, mut style) = if entry_str.starts_with("DIR_") {
                ("DIR_", Style::default().fg(Color::White))
            } else {
                ("FILE_", Style::default().fg(Color::Yellow))
            };
            if index == currently_selected  {
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

}