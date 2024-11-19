pub mod prelude {
    use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
    use rand::{prelude::StdRng, SeedableRng};
    pub type UniversalResult<T> = Result<T, Box<dyn std::error::Error>>;

    pub fn generate_keypair(seed: u64) -> UniversalResult<(RsaPrivateKey, RsaPublicKey, StdRng)> {
        let mut random_gen = StdRng::seed_from_u64(seed);
        let privatekey = RsaPrivateKey::new(&mut random_gen, 2048)?;
        let publickey = RsaPublicKey::from(&privatekey);
        Ok((privatekey, publickey, random_gen))
    }
    pub fn decrypt(privatekey: &RsaPrivateKey, publickey:  &RsaPublicKey, encrypted: Vec<u8>) -> UniversalResult<String> {
        let content = privatekey.decrypt(Pkcs1v15Encrypt, &encrypted)?;
        Ok(String::from_utf8(content).unwrap())

    }
}