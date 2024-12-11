#![allow(clippy::unused_io_amount)]
use rustls::ServerConfig;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tar::Builder;
use useful::prelude::UniversalResult;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use useful::server::*;
use std::{path::{Path, PathBuf}, sync::Arc};
const CERTIFICATE_FILE: &str = "../certificates/server_chain.pem";
const PK_FILE: &str = "../certificates/server.key";
const ADDR: &str = "0.0.0.0:13360";
#[tokio::main]
async fn main() -> UniversalResult<()> {
    let socket_config = {
        let (certificate, privatekey) = load_tls(CERTIFICATE_FILE, PK_FILE)?;
        ServerConfig::builder().with_no_client_auth().with_single_cert(certificate, privatekey)?
    };


    let acceptor = TlsAcceptor::from(Arc::new(socket_config));
    let socket = TcpListener::bind(ADDR).await?;
    println!("Daemon connected to {ADDR}");

    loop {
        let (client, addr) = socket.accept().await?;
        println!("Accepted {addr}");
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
        let mut client = match acceptor.accept(client).await {
            Ok(e) => e,
            Err(err) => {
                eprintln!("Handshake failed :( - {err}");
                return;
            }
        };
        let thread_result: UniversalResult<()> = async {
        
        let mut current_path = std::path::PathBuf::from(".").canonicalize()?;

        let entries = list_directory(&current_path)?.join("\r"); 
        let packet = build_packet(entries, '\r');
        client.write(&packet).await?;
        loop {
            let content_length: usize = {
                let mut current_char: [u8;1] = [0];
                let mut buffer = String::new();
                while current_char[0] != b'\r' {
                client.read_exact(&mut current_char).await?;
                buffer.push(current_char[0] as char);
                }
                buffer.pop();
                buffer.parse()?

            };
            let mut data: Vec<u8> = vec![0;content_length];
            client.read_exact(&mut data).await?;
            let data = String::from_utf8(data)?;
            println!("Received: {data}");
            if let Some(data) = data.strip_prefix("FILE_") {
                let content = std::fs::read_to_string(data)?;
                let packet =  build_packet(content, '\r');
                client.write(&packet).await?;

            }
            else if let Some(data) = data.strip_prefix("DIR_") {
                if data == ".." {
                    current_path.push("..");
                    current_path = current_path.canonicalize().unwrap();

                }
                else {
                    current_path = PathBuf::from(data);
                }

                let entries = list_directory(&PathBuf::from(&current_path))?.join("\r");
                let entries = build_packet(entries, '\r');
                client.write(&entries).await?;

            }
            else if let Some(data) = data.strip_prefix("SAVEDIR_") {
                println!("saving started");
                let path = Path::new(data);
                let mut buffer: Vec<u8> = vec![];
                {
                let mut builder = Builder::new(&mut buffer);
                builder.append_dir_all("", path)?;
                builder.finish()?;
                }
                let mut packet = Vec::from(format!("{}\r", buffer.len()));
                packet.extend_from_slice(&buffer);

                client.write(&packet).await?;
                println!("Saving sent");

            }
            else {
                eprintln!("Error when serving client {addr}: Invalid syntax");
                client.shutdown().await?;
                return Ok(());

            }
        };
        }.await;
        if let Err(error) = thread_result {
                eprintln!("Error when serving client {addr}: {error}");

        }
        });
    }
}
