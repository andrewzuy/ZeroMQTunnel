use aes::Aes256;
use cipher::block_padding::Pkcs7;
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use cbc::{Decryptor, Encryptor};
use sha2::{Digest, Sha256};

type Aes256CbcEnc = Encryptor<Aes256>;
type Aes256CbcDec = Decryptor<Aes256>;

const BLOCK_SIZE: usize = 16;

#[derive(Clone)]
pub struct AesConfig {
    key: [u8; 32],
}

impl AesConfig {
    pub fn from_passphrase(passphrase: &str) -> Self {
        let key = derive_key(passphrase);
        Self { key }
    }

    pub fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        let mut iv = [0u8; 16];
        getrandom::getrandom(&mut iv).expect("failed to generate IV");
        let encryptor = Aes256CbcEnc::new(&self.key.into(), &iv.into());
        let mut buffer = vec![0u8; data.len() + BLOCK_SIZE];
        buffer[..data.len()].copy_from_slice(data);
        let ciphertext = encryptor
            .encrypt_padded_mut::<Pkcs7>(&mut buffer, data.len())
            .expect("encryption failed");
        let mut result = Vec::with_capacity(iv.len() + ciphertext.len());
        result.extend_from_slice(&iv);
        result.extend_from_slice(ciphertext);
        result
    }

    pub fn decrypt(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        if data.len() < 16 {
            anyhow::bail!("encrypted data too short");
        }
        let (iv_bytes, ciphertext) = data.split_at(16);
        let iv: [u8; 16] = iv_bytes.try_into().map_err(|_| anyhow::anyhow!("iv must be 16 bytes"))?;
        let decryptor = Aes256CbcDec::new(&self.key.into(), &iv.into());
        let mut buffer = ciphertext.to_vec();
        let plaintext = decryptor
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|e| anyhow::anyhow!("decryption failed: {}", e))?;
        Ok(plaintext.to_vec())
    }
}

fn derive_key(passphrase: &str) -> [u8; 32] {
    let hash = Sha256::digest(passphrase.as_bytes());
    hash.try_into().map_err(|_| anyhow::anyhow!("sha256 digest must be 32 bytes")).unwrap()
}
