//! MIX 索引解密：RSA 解出 Blowfish 密钥，再 ECB 解开文件表。
//!
//! 算法与公开工具（ccmixar / ccmix）同族：仅保护索引，正文不加密。

use blowfish::{
    Blowfish,
    cipher::{Block, BlockCipherDecrypt, KeyInit},
};
use byteorder::BE;
use num_bigint::BigUint;
use ra_types::{RaError, RaResult};

/// RSA 公钥指数。
const RSA_EXPONENT: u32 = 65537;

/// RSA 模数（40 字节，大端）。源自 Westwood `keys.ini` 的公开 Base64 钥。
const RSA_MODULUS_BE: [u8; 40] = [
    0x51, 0xBC, 0xDA, 0x08, 0x6D, 0x39, 0xFC, 0xE4, 0x56, 0x51, 0x60, 0xD6, 0x51, 0x71, 0x3F, 0xA2, 0xE8, 0xAA, 0x54, 0xFA, 0x66, 0x82, 0xB0,
    0x4A, 0xAB, 0xDD, 0x0E, 0x6A, 0xF8, 0xB0, 0xC1, 0xE6, 0xD1, 0xFB, 0x4F, 0x3D, 0xAA, 0x43, 0x7F, 0x15,
];

/// RSA 密钥块字节数（加密 MIX 索引前）。
pub(crate) const RSA_KEY_BLOCK_SIZE: usize = 80;
const RSA_HALF_SIZE: usize = 40;
const BLOWFISH_KEY_SIZE: usize = 56;
/// Blowfish 块大小（字节）。
pub(crate) const BLOWFISH_BLOCK_SIZE: usize = 8;
const RSA_COMBINE_SHIFT_BITS: u64 = 312;

/// 从 80 字节 RSA 块解出 56 字节 Blowfish 密钥。
pub(crate) fn extract_blowfish_key(encrypted_block: &[u8]) -> RaResult<[u8; BLOWFISH_KEY_SIZE]> {
    if encrypted_block.len() < RSA_KEY_BLOCK_SIZE {
        return Err(RaError::Parse(format!("mix RSA 块过小: {}（需要 {}）", encrypted_block.len(), RSA_KEY_BLOCK_SIZE)));
    }

    let mut reversed = encrypted_block[..RSA_KEY_BLOCK_SIZE].to_vec();
    reversed.reverse();

    let s0 = rsa_decrypt_be(&reversed[..RSA_HALF_SIZE]);
    let s1 = rsa_decrypt_be(&reversed[RSA_HALF_SIZE..RSA_KEY_BLOCK_SIZE]);
    let combined = (s0 << RSA_COMBINE_SHIFT_BITS) + s1;

    let be_bytes = combined.to_bytes_be();
    let mut key = [0u8; BLOWFISH_KEY_SIZE];
    if be_bytes.len() <= BLOWFISH_KEY_SIZE {
        let start = BLOWFISH_KEY_SIZE - be_bytes.len();
        key[start..].copy_from_slice(&be_bytes);
    }
    else {
        let skip = be_bytes.len() - BLOWFISH_KEY_SIZE;
        key.copy_from_slice(&be_bytes[skip..]);
    }
    key.reverse();
    Ok(key)
}

/// Blowfish ECB 原地解密；长度须为 8 的倍数。
pub fn blowfish_decrypt_ecb(key: &[u8], data: &mut [u8]) -> RaResult<()> {
    if !data.len().is_multiple_of(BLOWFISH_BLOCK_SIZE) {
        return Err(RaError::Parse(format!("Blowfish 数据长度 {} 不是 {} 的倍数", data.len(), BLOWFISH_BLOCK_SIZE)));
    }

    let cipher: Blowfish<BE> = Blowfish::new_from_slice(key).map_err(|e| RaError::Parse(format!("Blowfish 初始化失败: {e}")))?;

    for chunk in data.chunks_exact_mut(BLOWFISH_BLOCK_SIZE) {
        let block: &mut Block<Blowfish<BE>> = chunk.try_into().map_err(|_| RaError::Parse("Blowfish 块长度无效".into()))?;
        cipher.decrypt_block(block);
    }
    Ok(())
}

fn rsa_decrypt_be(ciphertext_be: &[u8]) -> BigUint {
    let ciphertext = BigUint::from_bytes_be(ciphertext_be);
    let exponent = BigUint::from(RSA_EXPONENT);
    let modulus = BigUint::from_bytes_be(&RSA_MODULUS_BE);
    ciphertext.modpow(&exponent, &modulus)
}
