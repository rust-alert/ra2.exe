//! Blowfish ECB 往返（`blowfish` 0.10 API）。

use blowfish::{
    Blowfish,
    cipher::{Block, BlockCipherEncrypt, KeyInit},
};
use byteorder::BE;
use ra_assets::blowfish_decrypt_ecb;

#[test]
fn blowfish_roundtrip_aligned() {
    let key = b"test_key_for_blowfish_validation!!!!";
    let mut data = *b"01234567";
    let cipher: Blowfish<BE> = Blowfish::new_from_slice(key).unwrap();
    {
        let block: &mut Block<Blowfish<BE>> = (&mut data[..]).try_into().unwrap();
        cipher.encrypt_block(block);
    }
    blowfish_decrypt_ecb(key, &mut data).unwrap();
    assert_eq!(&data, b"01234567");
}
