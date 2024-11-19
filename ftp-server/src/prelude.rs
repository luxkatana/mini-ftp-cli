pub mod prelude {
    use core::str;
    use std::io::{Read, Write};
    use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::net::TcpStream;
    use rand::prelude::*;
    pub type UniversalResult<T> = Result<T, Box<dyn std::error::Error>>;
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



        Ok(generate_random_private_key_ip(client_time_parsed)?)

    }

    fn generate_random_private_key_ip(time: usize) -> Result<(RsaPrivateKey, RsaPublicKey, StdRng), Box<dyn std::error::Error>> {
        let mut rand_generator = StdRng::seed_from_u64(time as u64);
        let private_key = RsaPrivateKey::new(&mut rand_generator, 2048)?;
        let public_key = RsaPublicKey::from(&private_key);



        Ok((private_key, public_key, rand_generator))


    }
    pub fn encrypt_message(privatekey: &RsaPrivateKey, publickey: &RsaPublicKey, rng: &mut StdRng, data: String) -> UniversalResult<Vec<u8>> {
        let encoded = publickey.encrypt(rng, Pkcs1v15Encrypt, data.as_bytes())?;

        Ok(encoded)
    }

}