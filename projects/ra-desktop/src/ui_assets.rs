//! 主菜单阶段的原版资源探测：证明进入对局前即可 MixVfs → SHP → RGBA。
//!
//! 本模块只做挂载与可读性探测；页面合成见 `ui_compose`，命中见 `ui_hit`。

use ra_adaptor::detect_edition;
use ra_assets::{IniDocument, Palette, ShpFile};
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
    /// 上述文件是否可读（仅存在性）。
    pub ui_ini_readable: bool,
    /// 已解析的 UI INI 文档（有字节且解析成功时）。
    pub ui_ini: Option<IniDocument>,
    /// 从 UI INI 抽出的 `.shp` 名（通常为空；零售菜单不在此文件）。
    pub ui_ini_shp_refs: Vec<String>,
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
                ui_ini: None,
                ui_ini_shp_refs: Vec::new(),
            };
        }
    };

    let ui_ini_name = Some(manifest.chain.ui_ini);
    let mut source = GameAssetSource::new(manifest.root.clone());
    let (mounted_root, _) = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let (mounted_nested, _) = source.mount_nested_plan(&manifest.composition.nested_mount_plan);

    let ui_ini_bytes = source.read(manifest.chain.ui_ini).ok();
    let ui_ini_readable = ui_ini_bytes.is_some();
    let ui_ini = ui_ini_bytes.as_ref().and_then(|b| match IniDocument::parse(b) {
        Ok(doc) => Some(doc),
        Err(e) => {
            tracing::warn!("UI INI 解析失败 {}: {e}", manifest.chain.ui_ini);
            None
        }
    });
    let ui_ini_shp_refs = ui_ini.as_ref().map(|d| d.collect_shp_refs()).unwrap_or_default();
    let mouse_frame = decode_named_shp_frame(&source, "unittem.pal", "mouse.shp");
    let clock_frame = decode_named_shp_frame(&source, "unittem.pal", "clock.shp");
    let ui_bit = match (&ui_ini, ui_ini_readable) {
        (Some(doc), _) => {
            format!("{} sections={} shp_refs={}", manifest.chain.ui_ini, doc.sections.len(), ui_ini_shp_refs.len())
        }
        (None, true) => format!("{} unparsed", manifest.chain.ui_ini),
        (None, false) => format!("{} missing", manifest.chain.ui_ini),
    };
    let note = match &mouse_frame {
        Some(img) => {
            format!("UI 探测 ok · mouse.shp {}×{} · {} · 根mix {} · 嵌套 {}", img.width(), img.height(), ui_bit, mounted_root, mounted_nested)
        }
        None => format!("UI 探测：未读到 mouse.shp · {} · 根mix {} · 嵌套 {}", ui_bit, mounted_root, mounted_nested),
    };
    if ui_ini_shp_refs.is_empty() && ui_ini.is_some() {
        tracing::info!("版本链 ui.ini 无 .shp 引用 · 主菜单素材需页面资源模型，不能指望该文件当目录");
    }
    MenuUiProbe { note, mouse_frame, clock_frame, source: Some(source), ui_ini_name, ui_ini_readable, ui_ini, ui_ini_shp_refs }
}

fn decode_named_shp_frame(source: &GameAssetSource, pal_name: &str, shp_name: &str) -> Option<RgbaImage> {
    let pal = Palette::parse(&source.read(pal_name).ok()?).ok()?;
    let bytes = source.read(shp_name).ok()?;
    let shp = ShpFile::parse(&bytes).ok()?;
    let frame = shp.frames.first()?;
    if frame.frame_width == 0 || frame.frame_height == 0 {
        return None;
    }
    RgbaImage::from_raw(u32::from(frame.frame_width), u32::from(frame.frame_height), frame.to_rgba(&pal))
}

/// 最近邻缩小到不超过 max_w×max_h（已更小则克隆）。
pub fn downscale_to_fit(img: &RgbaImage, max_w: u32, max_h: u32) -> Option<RgbaImage> {
    if img.width() == 0 || img.height() == 0 || max_w == 0 || max_h == 0 {
        return None;
    }
    let scale = (max_w as f32 / img.width() as f32).min(max_h as f32 / img.height() as f32).min(1.0);
    let nw = ((img.width() as f32) * scale).round().max(1.0) as u32;
    let nh = ((img.height() as f32) * scale).round().max(1.0) as u32;
    if nw == img.width() && nh == img.height() {
        return Some(img.clone());
    }
    let mut pixels = vec![0u8; (nw as usize) * (nh as usize) * 4];
    for dy in 0..nh {
        let sy = (dy as f32 * img.height() as f32 / nh as f32) as u32;
        for dx in 0..nw {
            let sx = (dx as f32 * img.width() as f32 / nw as f32) as u32;
            let si = ((sy * img.width() + sx) * 4) as usize;
            let di = ((dy * nw + dx) * 4) as usize;
            pixels[di..di + 4].copy_from_slice(&img.as_raw()[si..si + 4]);
        }
    }
    RgbaImage::from_raw(nw, nh, pixels)
}
