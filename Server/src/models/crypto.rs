extern crate aes;
extern crate block_modes;
extern crate block_padding;
extern crate hex;
extern crate generic_array;

use aes::Aes128;
use block_modes::{BlockMode, Cbc};
use block_padding::Pkcs7;
use hex::{encode};
use generic_array::GenericArray;
use crate::models::crypto::aes::NewBlockCipher;

const DEFAULT_IV: &str = "0000000000000000"; // Default IV, 16 bytes (you can adjust this)
const DEFAULT_KEY: &str = "00000000000000000000000000000000"; // Default 32-byte key (you can adjust this)

// Helper function to pad the input to the required length
fn pad_if_too_short(value: &str, length: usize, default: &str) -> String {
    if value.len() < length {
        let mut padded_value = value.to_string();
        padded_value.push_str(&default[(value.len()..length).start..]);
        padded_value
    } else {
        value.to_string()
    }
}

pub fn encrypt_data(iv_str: &str, secret_key: &str, data: &str) -> String {
    // Parse the IV and key with padding
    let iv = pad_if_too_short(iv_str, 16, DEFAULT_IV);
    let key = pad_if_too_short(secret_key, 32, DEFAULT_KEY);

    // Convert raw strings to bytes directly (not hex-encoded)
    let iv_bytes = iv.as_bytes();
    let key_bytes = key.as_bytes();

    // Convert Vec<u8> to GenericArray<u8, N>
    let iv_array = GenericArray::from_slice(&iv_bytes);  // Length of iv_array will be 16
    let key_array = GenericArray::from_slice(&key_bytes[..16]);  // Use first 16 bytes for AES-128

    // Ensure the input data is not empty
    if data.trim().is_empty() {
        return "".to_string();
    }

    // Initialize the AES-CBC cipher
    let cipher = Cbc::<Aes128, Pkcs7>::new(Aes128::new(&key_array), &iv_array);

    let ciphertext = cipher.encrypt_vec(data.as_bytes());

    // Return hex-encoded ciphertext
    encode(ciphertext)
}

pub fn decrypt_data(iv_str: &str, secret_key: &str, encrypted_data: &str) -> String {
    // Parse the IV and key with padding
    let iv = pad_if_too_short(iv_str, 16, DEFAULT_IV);
    let key = pad_if_too_short(secret_key, 32, DEFAULT_KEY);

    // Convert raw strings to bytes directly (not hex-encoded)
    let iv_bytes = iv.as_bytes();
    let key_bytes = key.as_bytes();

    // Convert Vec<u8> to GenericArray<u8, N>
    let iv_array = GenericArray::from_slice(&iv_bytes);  // Length of iv_array will be 16
    let key_array = GenericArray::from_slice(&key_bytes[..16]);  // Use first 16 bytes for AES-128

    // Ensure the input data is not empty
    if encrypted_data.trim().is_empty() {
        return "".to_string();
    }

    // Decode the hex-encoded ciphertext
    let ciphertext = hex::decode(encrypted_data).unwrap_or_default();
    
    // Initialize the AES-CBC cipher
    let cipher = Cbc::<Aes128, Pkcs7>::new(Aes128::new(&key_array), &iv_array);

    // Decrypt and handle potential errors
    match cipher.decrypt_vec(&ciphertext) {
        Ok(plaintext) => {
            String::from_utf8(plaintext).unwrap_or_default()
        },
        Err(_) => "".to_string()
    }
}