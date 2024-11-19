mod prelude;
use prelude::prelude::*;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};
fn main() -> UniversalResult<()> {
    
    let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64() as u64;
    let mut client = TcpStream::connect("0.0.0.0:13360")?;
    let (private_key, public_key, random_generator) = generate_keypair(time)?;
    client.write(format!("{time}").as_bytes())?;
    let mut data: [u8; 256] = [0; 256];
    
    client.read(&mut data)?;
    let d = decrypt(&private_key, &public_key, data.to_vec())?;
    println!("{d}");
    Ok(())

}
