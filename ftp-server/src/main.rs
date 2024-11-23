use useful::prelude::*;
use useful::server::*;
use std::io::Read;
use std::thread::current;
use std::{io::Write, net::TcpListener};
use std::thread::spawn;
fn main() -> UniversalResult<()> {
    let socket = TcpListener::bind("0.0.0.0:13360")?;

    loop {
        let (mut client, _) = socket.accept()?;
        spawn(move || {
        println!("new incoming");
        let (privatekey, publickey, mut rng) = match handshake(&mut client) {
            Ok(e) =>  e,
            Err(err) => {
                eprintln!("Error when handshake {err}");
                return;
            }
        };
        
        let mut current_path = std::path::PathBuf::from(".").canonicalize().unwrap();

        let entries = list_directory(&current_path).unwrap().join("\t"); 
        println!("{}", entries.len());
        // for entry in entries {
        //     let fileasstr = entry.to_str().unwrap().to_string() + "\r"; 
        //     packet.push_str(&fileasstr)
        // }
        let packet = build_packet(&publickey, &mut rng, entries).expect("Could not build backet");
        client.write(&packet).unwrap();
        loop {
            let mut current_char: [u8;1] = [0];
            let mut content_length: u32 = 0;
            while current_char[0] != 0x9 {
                client.read(&mut current_char).unwrap();
                content_length = (content_length * 10) + (current_char[0] - 30) as u32;
            }
            println!("content len: {content_length}");

        }

        });
    }
}
