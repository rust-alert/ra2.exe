//! 集成测试：原 `src/image/pcx.rs` 内联测试迁出。

use ra_assets::parse_pcx;

fn solid_pcx(width: u16, height: u16, color_index: u8, rgb: [u8; 3]) -> Vec<u8> {
    let mut data = vec![0u8; 128];
    data[0] = 0x0A;
    data[1] = 5;
    data[2] = 1;
    data[3] = 8;
    data[4] = 0;
    data[5] = 0;
    data[6] = 0;
    data[7] = 0;
    let xmax = (width - 1).to_le_bytes();
    let ymax = (height - 1).to_le_bytes();
    data[8] = xmax[0];
    data[9] = xmax[1];
    data[10] = ymax[0];
    data[11] = ymax[1];
    data[65] = 1;
    let bpl = width.to_le_bytes();
    data[66] = bpl[0];
    data[67] = bpl[1];

    let total = width as usize * height as usize;
    let mut body = Vec::new();
    let mut left = total;
    while left > 0 {
        let run = left.min(63);
        body.push(0xC0 | run as u8);
        body.push(color_index);
        left -= run;
    }
    data.extend_from_slice(&body);
    data.push(0x0C);
    for i in 0..256 {
        if i == color_index as usize {
            data.extend_from_slice(&rgb);
        }
        else {
            data.extend_from_slice(&[0, 0, 0]);
        }
    }
    data
}

#[test]
fn parse_solid_8bit_pcx() {
    let raw = solid_pcx(4, 2, 7, [10, 20, 30]);
    let img = parse_pcx(&raw).expect("pcx");
    assert_eq!((img.width, img.height), (4, 2));
    assert_eq!(&img.rgba[0..4], &[10, 20, 30, 255]);
    assert_eq!(&img.rgba[4 * 7..4 * 8], &[10, 20, 30, 255]);
}
