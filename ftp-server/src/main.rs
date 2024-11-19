mod prelude;
use std::{io::Read, net::TcpListener};
use prelude::prelude::{handshake, UniversalResult};
use rsa::pkcs1::EncodeRsaPrivateKey;
fn main() -> UniversalResult<()> {
    let socket = TcpListener::bind("0.0.0.0:13360")?;
    loop {
        let (mut client, address) = socket.accept()?;
        let (privatekey, publickey, rng) = handshake(&mut client)?;
        

        
        // if client_private_key == private_key.


    }
}
