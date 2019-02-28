use hex;
use ring::aead::*;
use ring::rand::{SecureRandom, SystemRandom};
use utils::errors::*;

pub fn encrypt(plaintext: &str, encryption_key: &str) -> Result<String, DatabaseError> {
    let mut key = encryption_key.to_string().into_bytes();
    if key.len() > 32 {
        key.truncate(32);
    }
    //Pad the key
    for _ in key.len()..32 {
        key.push(0);
    }
    let sealing_key = SealingKey::new(&CHACHA20_POLY1305, &key)?;
    let mut nonce = vec![0; 12];
    let rng = SystemRandom::new();
    rng.fill(&mut nonce)?;

    let mut in_out = plaintext.to_string().into_bytes();
    //Add extra bytes for the tag
    for _ in 0..CHACHA20_POLY1305.tag_len() {
        in_out.push(0);
    }
    let _ = seal_in_place(
        &sealing_key,
        &nonce,
        &[],
        &mut in_out,
        CHACHA20_POLY1305.tag_len(),
    )?;

    let data = hex::encode(&in_out);
    let mut nonce_data = hex::encode(nonce);
    nonce_data.push_str(data.as_str());

    Ok(nonce_data)
}

pub fn decrypt(ciphertext: &str, encryption_key: &str) -> Result<String, DatabaseError> {
    let mut key = encryption_key.to_string().into_bytes();
    if key.len() > 32 {
        key.truncate(32);
    }
    for _ in key.len()..32 {
        key.push(0);
    }

    let opening_key = OpeningKey::new(&CHACHA20_POLY1305, &key)?;
    //check that the data is long enough to contain a nonce
    if ciphertext.len() < 24 {
        return Err(DatabaseError::new(
            ErrorCode::InternalError,
            Some("Cannot decrypt data".to_string()),
        ));
    }

    //split up nonce and data
    let (n, d) = ciphertext.split_at(24);
    let new_nonce = hex::decode(n);
    let new_data = hex::decode(d);

    if new_nonce.is_err() || new_data.is_err() {
        return Err(DatabaseError::new(
            ErrorCode::InternalError,
            Some("Cannot decrypt data".to_string()),
        ));
    }

    let mut in_out = new_data.unwrap();
    let decrypted_data =
        open_in_place(&opening_key, &new_nonce.unwrap(), &[], 0, &mut in_out).unwrap();

    let plaintext = String::from_utf8(decrypted_data.to_vec());
    //Doing this rather that implement a From for just this once instance
    if plaintext.is_err() {
        return Err(DatabaseError::new(
            ErrorCode::InternalError,
            Some("Cannot decrypt data".to_string()),
        ));
    }

    Ok(plaintext.unwrap())
}
