//! 地图键值：`y * 1000 + x` 十进制打包格坐标。

/// 将打包整数拆成 `(x, y)`。
pub fn unpack_packed_cell(pos: u32) -> (u16, u16) {
    let y = (pos / 1000) as u16;
    let x = (pos % 1000) as u16;
    (x, y)
}

/// 解析十进制打包格文本；非法数字返回 `None`。
pub fn parse_packed_cell(raw: &str) -> Option<(u16, u16)> {
    let pos: u32 = raw.trim().parse().ok()?;
    Some(unpack_packed_cell(pos))
}
