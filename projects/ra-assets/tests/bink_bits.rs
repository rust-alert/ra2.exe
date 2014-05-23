//! 集成测试：原 `src/image/bink_bits.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn read_single_bits_lsb_first() {
    let data = [0xA3u8];
    let mut r = BitReader::from_bytes(&data);
    assert_eq!(r.read_bit().unwrap(), true);
    assert_eq!(r.read_bit().unwrap(), true);
    assert_eq!(r.read_bit().unwrap(), false);
    assert_eq!(r.read_bit().unwrap(), false);
    assert_eq!(r.read_bit().unwrap(), false);
    assert_eq!(r.read_bit().unwrap(), true);
    assert_eq!(r.read_bit().unwrap(), false);
    assert_eq!(r.read_bit().unwrap(), true);
}

#[test]
fn read_bits_nibbles() {
    let data = [0xA3u8];
    let mut r = BitReader::from_bytes(&data);
    assert_eq!(r.read_bits(4).unwrap(), 0x3);
    assert_eq!(r.read_bits(4).unwrap(), 0xA);
}

#[test]
fn read_bits_across_bytes() {
    let data = [0x78u8, 0x56];
    let mut r = BitReader::from_bytes(&data);
    assert_eq!(r.read_bits(16).unwrap(), 0x5678);
}

#[test]
fn align_to_dword() {
    let data = [0xFFu8; 8];
    let mut r = BitReader::from_bytes(&data);
    r.skip(5);
    r.align_to_dword();
    assert_eq!(r.pos(), 32);
}

#[test]
fn eof_errors() {
    let data = [0u8; 1];
    let mut r = BitReader::from_bytes(&data);
    r.skip(8);
    assert!(r.read_bit().is_err());
}

#[test]
fn fixed_vlc_tables_build() {
    let tables = build_fixed_vlc_tables().unwrap();
    assert_eq!(tables[0].bits(), 4);
    // 全 4 位等长树：读 0b0000 → 符号 0
    let data = [0x00u8];
    let mut r = BitReader::from_bytes(&data);
    assert_eq!(tables[0].decode(&mut r).unwrap(), 0);
}
