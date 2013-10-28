//! 主菜单阶段的原版资源探测：证明进入对局前即可 MixVfs → SHP → RGBA。
//!
//! **不是 Pre-Alpha 原版 UI。** 只验证安装挂载与常见 UI SHP / `ui.ini` 可读；
//! 页面布局、字体、命中框仍由占位菜单负责。

use ra_adaptor::detect_edition;
use ra_assets::{Palette, ShpFile};
use ra_renderer::RgbaImage;
use ra_types::{AssetSource, GameEdition};

use crate::{config::load_desktop_config_with_diagnostics, fs_source::GameAssetSource};

/// 一次主菜单资源探测结果（保留挂载源供后续 UI 帧复用）。
pub struct MenuUiProbe {
    /// 人类可读备注（标题栏 / 日志）。
    pub note: String,
    /// `mouse.shp` 第一帧（若成功）。
    pub mouse_frame: Option<RgbaImage>,
    /// `clock.shp` 第一帧（若成功；证明挂载源可续读）。
    pub clock_frame: Option<RgbaImage>,
    /// 已挂载的安装资源（探测失败时为 `None`）。
    pub source: Option<GameAssetSource>,
    /// 版本链上的 UI 配置文件名（如 `ui.ini` / `uimd.ini`）。
    pub ui_ini_name: Option<&'static str>,
    /// 上述文件是否可读（仅存在性，尚未解析语义）。
    pub ui_ini_readable: bool,
}

impl MenuUiProbe {
    /// 用已挂载源再解一帧 SHP（调色板默认 `unittem.pal`）。
    #[allow(dead_code)]
    pub fn decode_shp_frame(&self, shp_name: &str) -> Option<RgbaImage> {
        let source = self.source.as_ref()?;
        decode_named_shp_frame(source, "unittem.pal", shp_name)
    }
}

/// 按桌面配置探测安装并尝试解码 `mouse.shp`、探测 `ui.ini`。
pub fn probe_menu_ui_assets() -> MenuUiProbe {
    let (cfg, _) = load_desktop_config_with_diagnostics();
    let explicit = match cfg.edition.as_deref() {
        Some(s) => GameEdition::parse(s).ok(),
        None => None,
    };
    let manifest = match detect_edition(&cfg.ra2_dir, explicit) {
        Ok(m) => m,
        Err(e) => {
            return MenuUiProbe {
                note: format!("UI 资源探测失败: {e}"),
                mouse_frame: None,
                clock_frame: None,
                source: None,
                ui_ini_name: None,
                ui_ini_readable: false,
            };
        }
    };

    let ui_ini_name = Some(manifest.chain.ui_ini);
    let mut source = GameAssetSource::new(manifest.root.clone());
    let (mounted_root, _) = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let mounted_nested = source.mount_nested_names(manifest.chain.nested_mix_files);

    let ui_ini_readable = source.read(manifest.chain.ui_ini).is_ok();
    let mouse_frame = decode_named_shp_frame(&source, "unittem.pal", "mouse.shp");
    let clock_frame = decode_named_shp_frame(&source, "unittem.pal", "clock.shp");
    let ui_bit = if ui_ini_readable {
        format!("{} ok", manifest.chain.ui_ini)
    }
    else {
        format!("{} missing", manifest.chain.ui_ini)
    };
    let note = match &mouse_frame {
        Some(img) => format!(
            "UI 探测 ok · mouse.shp {}×{} · {} · 根mix {} · 嵌套 {} · 占位菜单仍非 Pre-Alpha",
            img.width, img.height, ui_bit, mounted_root, mounted_nested
        ),
        None => format!(
            "UI 探测：未读到 mouse.shp · {} · 根mix {} · 嵌套 {} · 占位菜单仍非 Pre-Alpha",
            ui_bit, mounted_root, mounted_nested
        ),
    };
    MenuUiProbe {
        note,
        mouse_frame,
        clock_frame,
        source: Some(source),
        ui_ini_name,
        ui_ini_readable,
    }
}

fn decode_named_shp_frame(source: &GameAssetSource, pal_name: &str, shp_name: &str) -> Option<RgbaImage> {
    let pal = Palette::parse(&source.read(pal_name).ok()?).ok()?;
    let bytes = source.read(shp_name).ok()?;
    let shp = ShpFile::parse(&bytes).ok()?;
    let frame = shp.frames.first()?;
    if frame.frame_width == 0 || frame.frame_height == 0 {
        return None;
    }
    RgbaImage::new(u32::from(frame.frame_width), u32::from(frame.frame_height), frame.to_rgba(&pal))
}

/// 将 `src` 贴到 `dst` 右上角。源像素 alpha 过低或近黑视为透明。
pub fn stamp_top_right(dst: &mut RgbaImage, src: &RgbaImage, margin: u32) {
    if src.width == 0 || src.height == 0 || dst.width == 0 || dst.height == 0 {
        return;
    }
    let ox = dst.width.saturating_sub(src.width.saturating_add(margin));
    let oy = margin.min(dst.height.saturating_sub(1));
    stamp_at(dst, src, ox, oy);
}

/// 将 `src` 贴到 `dst` 左上角。
pub fn stamp_top_left(dst: &mut RgbaImage, src: &RgbaImage, margin: u32) {
    if src.width == 0 || src.height == 0 || dst.width == 0 || dst.height == 0 {
        return;
    }
    stamp_at(dst, src, margin, margin);
}

fn stamp_at(dst: &mut RgbaImage, src: &RgbaImage, ox: u32, oy: u32) {
    for sy in 0..src.height {
        let dy = oy.saturating_add(sy);
        if dy >= dst.height {
            break;
        }
        for sx in 0..src.width {
            let dx = ox.saturating_add(sx);
            if dx >= dst.width {
                break;
            }
            let si = ((sy * src.width + sx) * 4) as usize;
            let di = ((dy * dst.width + dx) * 4) as usize;
            let a = src.pixels[si + 3];
            let r = src.pixels[si];
            let g = src.pixels[si + 1];
            let b = src.pixels[si + 2];
            if a < 8 || (r | g | b) < 8 {
                continue;
            }
            dst.pixels[di..di + 4].copy_from_slice(&src.pixels[si..si + 4]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_skips_near_black() {
        let mut dst = RgbaImage::new(4, 4, vec![10u8; 4 * 4 * 4]).unwrap();
        let mut src_px = vec![0u8; 2 * 2 * 4];
        // 一像素亮色
        src_px[0..4].copy_from_slice(&[200, 100, 50, 255]);
        let src = RgbaImage::new(2, 2, src_px).unwrap();
        stamp_top_right(&mut dst, &src, 0);
        // 右上角 (2,0) 应对上 src (0,0)
        let di = ((0u32 * 4 + 2) * 4) as usize;
        assert_eq!(&dst.pixels[di..di + 3], &[200, 100, 50]);
    }
}
