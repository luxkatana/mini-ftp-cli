mod prelude;
use std::{io::{Read, Write}, net::TcpListener};
use prelude::prelude::{encrypt_message, handshake, UniversalResult};
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
        let encrypted_end = encrypt_message(&privatekey, &publickey, &mut rng, "hi".to_string()).expect("Could not encrypt message");
        println!("{}", encrypted_end.len());
        dbg!(encrypted_end.clone());
        client.write(&encrypted_end).unwrap();
        println!("sent");
        });

        // if client_private_key == private_key.


    }
}
