//! 侧栏电表：`powerp.shp` 帧语义与满度/变色布局。
//!
//! 零售 `powerp.shp` 为 16×2 色带（5 帧）：空轨 / 绿 / 黄 / 红 / 灰。
//! 沿 cameo 左缘纵向平铺：空轨铺满 `cameo_band`；色带自底向上。
//! 条带总高随 `max(供电, 耗电)` 增长，填色高度为耗电占该标称的比例（盈余时上方留空轨）。

/// `powerp.shp` 空轨帧。
pub const POWERP_FRAME_TRACK: usize = 0;
/// 盈余（绿）。
pub const POWERP_FRAME_GREEN: usize = 1;
/// 紧张（黄，耗电接近供电）。
pub const POWERP_FRAME_YELLOW: usize = 2;
/// 低电 / 超载（红）。
pub const POWERP_FRAME_RED: usize = 3;
/// 无电闲置灰带（可选）。
pub const POWERP_FRAME_GRAY: usize = 4;
/// `powerp.shp` 帧数。
pub const POWERP_FRAME_COUNT: usize = 5;

/// 电表满格对应的标称供电或耗电（超出钳到满条）。
pub const POWER_METER_FULL_LEVEL: i32 = 2000;

/// 电表色带种类（对应 `powerp` 填色帧）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerMeterColor {
    /// 绿：供电充裕。
    Green,
    /// 黄：耗电 ≥ 供电的 80% 且未低电。
    Yellow,
    /// 红：低电或断电。
    Red,
    /// 灰：双侧均为 0 时的闲置提示（通常只画空轨，fill 为 0）。
    Gray,
}

impl PowerMeterColor {
    /// 对应 `powerp.shp` 帧下标。
    pub const fn frame_index(self) -> usize {
        match self {
            Self::Green => POWERP_FRAME_GREEN,
            Self::Yellow => POWERP_FRAME_YELLOW,
            Self::Red => POWERP_FRAME_RED,
            Self::Gray => POWERP_FRAME_GRAY,
        }
    }
}

/// 一帧电表绘制参数（相对 `cameo_band` 高度）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerMeterPaint {
    /// 自底向上的色带高度（像素）。`0` 表示只铺空轨。
    pub fill_h: i32,
    /// 色带帧。
    pub color: PowerMeterColor,
}

/// 由有效供电 / 耗电 / 低电态计算电表满度与颜色。
///
/// - 标称高度取 `max(output, drain)` 相对 [`POWER_METER_FULL_LEVEL`] 的比例。
/// - 填色高度 = 标称高度 × `drain / max(output, drain)`（盈余时上方留空）。
/// - 低电 → 红；否则耗电 ≥ 供电 80% → 黄；否则绿。
pub fn power_meter_paint(band_h: i32, power_output: i32, power_drain: i32, low_power: bool) -> PowerMeterPaint {
    let band_h = band_h.max(0);
    let output = power_output.max(0);
    let drain = power_drain.max(0);
    let level = output.max(drain);

    let color = if low_power {
        PowerMeterColor::Red
    }
    else if output == 0 && drain == 0 {
        PowerMeterColor::Gray
    }
    else if output == 0 || drain.saturating_mul(5) >= output.saturating_mul(4) {
        // 耗电 ≥ 供电的 80%。
        PowerMeterColor::Yellow
    }
    else {
        PowerMeterColor::Green
    };

    let fill_h = if level == 0 || band_h == 0 {
        0
    }
    else {
        let capped = level.min(POWER_METER_FULL_LEVEL);
        let meter_h = (i64::from(band_h) * i64::from(capped) / i64::from(POWER_METER_FULL_LEVEL)).max(1);
        let meter_h = meter_h.min(i64::from(band_h));
        // 耗电占标称的比例；低电时 drain≥output，填满该标称段。
        let filled = (meter_h * i64::from(drain) / i64::from(level)).max(if drain > 0 { 1 } else { 0 });
        filled.min(meter_h) as i32
    };

    PowerMeterPaint { fill_h, color }
}
