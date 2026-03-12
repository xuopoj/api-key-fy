use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;

/// Encrypt plaintext using AES-256-GCM.
/// Returns a base64-encoded string: `nonce(12 bytes) || ciphertext`.
pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("encryption failed: {e}"))?;

    let mut combined = nonce_bytes.to_vec();
    combined.extend(ciphertext);

    Ok(B64.encode(combined))
}

/// Decrypt a base64-encoded `nonce || ciphertext` produced by `encrypt`.
pub fn decrypt(encoded: &str, key: &[u8; 32]) -> Result<String> {
    let data = B64.decode(encoded).context("invalid base64 in encrypted value")?;
    anyhow::ensure!(data.len() > 12, "encrypted value too short");

    let (nonce_bytes, ciphertext) = data.split_at(12);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("decryption failed: {e}"))?;

    String::from_utf8(plaintext).context("decrypted value is not valid UTF-8")
}

/// Parse a 32-byte key from a hex or base64 string in the config.
pub fn parse_key(raw: &str) -> Result<[u8; 32]> {
    let bytes = if raw.len() == 64 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
        hex_decode(raw)?
    } else {
        B64.decode(raw).context("ENCRYPTION_KEY must be 64-char hex or 32-byte base64")?
    };

    bytes.try_into().map_err(|_| anyhow::anyhow!("ENCRYPTION_KEY must be exactly 32 bytes"))
}

fn hex_decode(s: &str) -> Result<Vec<u8>> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).context("invalid hex"))
        .collect()
}
