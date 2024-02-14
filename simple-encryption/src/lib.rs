// extern crate crypto;
extern crate rand;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce};
use curve25519_dalek::ristretto::CompressedRistretto;
use thiserror::Error;

use rand::RngCore;
use rand::{Rng, rngs::OsRng};
use x25519_dalek::x25519;
// use crypto::curve25519::{curve25519_base, curve25519};
// use crypto::chacha20poly1305::ChaCha20Poly1305;
// use crypto::aead::{AeadEncryptor, AeadDecryptor};
#[derive(Error,Debug)]
pub enum EncryptError {
    #[error("rng init failed")]
    RngInitializationFailed,
}
pub fn x25519_base(a: [u8;32]) -> [u8; 32]{
    return x25519(a, x25519_dalek::X25519_BASEPOINT_BYTES);
}
pub fn encrypt(public_key: &[u8; 32], message: &[u8]) -> Result<Vec<u8>, EncryptError> {
    let mut rng = OsRng::default();
    
    let mut ephemeral_secret_key = [0u8; 32];
    rng.fill_bytes(&mut ephemeral_secret_key[..]);
    
    let ephemeral_public_key: [u8; 32] = x25519_base(ephemeral_secret_key);
    let symmetric_key = x25519_dalek::x25519(ephemeral_secret_key, *public_key);
    
    let mut c = ChaCha20Poly1305::new_from_slice(&symmetric_key).unwrap();
    
    let mut output = vec![0; 32 + 16];
    let mut tag = [0u8; 16];
    output.append(&mut c.encrypt(&Nonce::default(), message).unwrap());
    // c.encrypt(message, , &mut tag[..]);
    
    for (dest, src) in (&mut output[0..32]).iter_mut().zip( ephemeral_public_key.iter() ) {
        *dest = *src;
    }
    
    for (dest, src) in (&mut output[32..48]).iter_mut().zip( tag.iter() ) {
        *dest = *src;
    }
    
    Ok(output)
}
#[derive(Error,Debug)]
pub enum DecryptError {
    #[error("malformed input")]
    Malformed,
    #[error("invalid input")]
    Invalid,
}

pub fn decrypt(secret_key: &[u8; 32], message: &[u8]) -> Result<Vec<u8>, DecryptError> {
    if message.len() < 48 {
        return Err(DecryptError::Malformed);
    }
    
    let ephemeral_public_key = (&message[0..32]).try_into().unwrap();
    let tag = &message[32..48];
    let ciphertext = &message[48..];
    
    let mut plaintext = vec![0; ciphertext.len()];
    let symmetric_key = x25519(*secret_key, ephemeral_public_key);
    
    let mut decrypter = ChaCha20Poly1305::new_from_slice(&symmetric_key).unwrap();
    // if !decrypter.decrypt(ciphertext, &mut plaintext[..], tag) {
    //     return Err(DecryptError::Invalid);
    // }
    
    // Ok(plaintext)
    return Ok(decrypter.decrypt(&Nonce::default(),ciphertext).unwrap());
}
