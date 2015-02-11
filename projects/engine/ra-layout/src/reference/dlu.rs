//! 对话框单位（DLU）换算。

/// 字体 base units（水平 / 垂直分子分母）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontBaseUnits {
    /// 水平分子。
    pub x_numer: i32,
    /// 水平分母。
    pub x_denom: i32,
    /// 垂直分子。
    pub y_numer: i32,
    /// 垂直分母。
    pub y_denom: i32,
}

/// MS Sans Serif 8pt：x×6/4、y×13/8。
pub const MS_SANS_SERIF_8PT: FontBaseUnits = FontBaseUnits {
    x_numer: 6,
    x_denom: 4,
    y_numer: 13,
    y_denom: 8,
};

/// 四舍五入的整数 MulDiv（兼容负值）。
pub fn mul_div_round(n: i32, numer: i32, denom: i32) -> i32 {
    debug_assert!(denom != 0);
    let value = i64::from(n) * i64::from(numer);
    let d = i64::from(denom);
    let rounded = if value >= 0 {
        (value + d / 2) / d
    } else {
        (value - d / 2) / d
    };
    rounded as i32
}

/// DLU 矩形（整数对话框单位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DluRect {
    /// 左。
    pub x: i32,
    /// 上。
    pub y: i32,
    /// 宽。
    pub width: i32,
    /// 高。
    pub height: i32,
}

impl DluRect {
    /// 构造。
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// 转为设计像素（整数，四舍五入）。
    pub fn to_design_px(self, units: FontBaseUnits) -> crate::geometry::Rect {
        let x = mul_div_round(self.x, units.x_numer, units.x_denom) as f32;
        let y = mul_div_round(self.y, units.y_numer, units.y_denom) as f32;
        let width = mul_div_round(self.width, units.x_numer, units.x_denom) as f32;
        let height = mul_div_round(self.height, units.y_numer, units.y_denom) as f32;
        crate::geometry::Rect::from_xywh(x, y, width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ms_sans_8pt_sample_matches_shell_helpers() {
        let r = DluRect::new(318, 122, 108, 23).to_design_px(MS_SANS_SERIF_8PT);
        assert_eq!(r.x, 477.0);
        assert_eq!(r.y, 198.0);
        assert_eq!(r.width, 162.0);
        assert_eq!(r.height, 37.0);
    }

    #[test]
    fn mul_div_round_negative() {
        assert_eq!(mul_div_round(-5, 6, 4), -8);
        assert_eq!(mul_div_round(-1, 13, 8), -2);
    }
}
