use aesus::{
    encrypt_bytes,
    decrypt_bytes,
    generate_passphrase,
    passphrase_entropy
};

pub fn encrypt(data: &[u8], password: &str) -> Result<Vec<u8>, String> {

    encrypt_bytes(data, password)
        .map_err(|e| e.to_string())
}

pub fn decrypt(data: &[u8], password: &str) -> Result<Vec<u8>, String> {

    decrypt_bytes(data, password)
        .map_err(|e| e.to_string())
}


pub fn generate_password(words: usize) -> (String, f64) {

    let pass = generate_passphrase(words);

    let entropy = passphrase_entropy(words);

    (pass, entropy)
}