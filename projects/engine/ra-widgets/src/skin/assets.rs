//! 主菜单阶段的安装资源挂载：MixVfs → 供 `compose` / [`crate::input::hit`] / 音频读取。
//!
//! 本模块只负责探测版本、按计划挂载与读 `ui.ini`；页面合成见 `compose`，命中见 [`crate::input::hit`]。

use ra_adaptor::detect_edition;
use ra_assets::{IniDocument, collect_shp_refs};
use ra_renderer::RgbaImage;
use ra_types::{AssetSource, GameEdition};

use crate::fs_source::GameAssetSource;
use ra_config::DesktopSettings;

/// 主菜单已挂载的安装资源（惰性装载一次，后续 UI 帧复用）。
pub struct MenuUiAssets {
    /// 人类可读备注（标题栏 / 日志）。
    pub note: String,
    /// 探测到的游戏版本（挂载失败时为 `None`）。
    pub edition: Option<GameEdition>,
    /// 已挂载的安装资源（挂载失败时为 `None`）。
    pub source: Option<GameAssetSource>,
    /// 版本链上的 UI 配置文件名（如 `ui.ini` / `uimd.ini`）。
    pub ui_ini_name: Option<&'static str>,
    /// 版本链上的音效表文件名（如 `sound.ini` / `soundmd.ini`）。
    pub sound_ini_name: Option<&'static str>,
    /// 版本链上的 EVA 播报表文件名（如 `eva.ini` / `evamd.ini`）。
    pub eva_ini_name: Option<&'static str>,
    /// 上述 UI 文件是否可读。
    pub ui_ini_readable: bool,
    /// 已解析的 UI INI 文档（有字节且解析成功时）。
    pub ui_ini: Option<IniDocument>,
    /// 从 UI INI 抽出的 `.shp` 名（通常为空；零售菜单不在此文件）。
    pub ui_ini_shp_refs: Vec<String>,
}

/// 按桌面配置探测版本并挂载菜单用 MIX；同时尝试读取版本链上的 `ui.ini`。
pub fn load_menu_ui_assets() -> MenuUiAssets {
    let (cfg, _) = DesktopSettings::load_or_default();
    let explicit = match cfg.edition.as_deref() {
        Some(s) => GameEdition::parse(s).ok(),
        None => None,
    };
    let manifest = match detect_edition(&cfg.ra2_dir, explicit) {
        Ok(m) => m,
        Err(e) => {
            return MenuUiAssets {
                note: format!("菜单资源挂载失败: {e}"),
                edition: None,
                source: None,
                ui_ini_name: None,
                sound_ini_name: None,
                eva_ini_name: None,
                ui_ini_readable: false,
                ui_ini: None,
                ui_ini_shp_refs: Vec::new(),
            };
        }
    };

    let ui_ini_name = Some(manifest.chain.ui_ini);
    let sound_ini_name = Some(manifest.chain.sound_ini);
    let eva_ini_name = Some(manifest.chain.eva_ini);
    let mut source = GameAssetSource::new(manifest.root.clone());
    let (mounted_root, _) = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let (mounted_nested, _) = source.mount_nested_plan(&manifest.composition.nested_mount_plan);
    // 遭遇战滑条饰条 `trofl`/`trofm`/`trofr` 在安装根 `Wdt.mix`（可缺）。
    let (mounted_wdt, _) =
        source.mount_root_plan(&[ra_adaptor::MountSpec { name: "Wdt.mix".to_string(), priority: 0, layer_id: "skirmish-chrome".to_string() }]);
    let mounted_root = mounted_root + mounted_wdt;

    let ui_ini_bytes = source.read(manifest.chain.ui_ini).ok();
    let ui_ini_readable = ui_ini_bytes.is_some();
    let ui_ini = ui_ini_bytes.as_ref().and_then(|b| match IniDocument::parse(b) {
        Ok(doc) => Some(doc),
        Err(e) => {
            tracing::warn!("UI INI 解析失败 {}: {e}", manifest.chain.ui_ini);
            None
        }
    });
    let ui_ini_shp_refs = ui_ini.as_ref().map(collect_shp_refs).unwrap_or_default();
    let ui_bit = match (&ui_ini, ui_ini_readable) {
        (Some(doc), _) => {
            format!("{} sections={} shp_refs={}", manifest.chain.ui_ini, doc.sections.len(), ui_ini_shp_refs.len())
        }
        (None, true) => format!("{} unparsed", manifest.chain.ui_ini),
        (None, false) => format!("{} missing", manifest.chain.ui_ini),
    };
    let note = format!("菜单资源已挂载 · {} · {ui_bit} · 根mix {mounted_root} · 嵌套 {mounted_nested}", manifest.chain.edition.as_str());
    if ui_ini_shp_refs.is_empty() && ui_ini.is_some() {
        tracing::info!("版本链 ui.ini 无 .shp 引用 · 主菜单素材需页面资源模型，不能指望该文件当目录");
    }
    MenuUiAssets {
        note,
        edition: Some(manifest.chain.edition),
        source: Some(source),
        ui_ini_name,
        sound_ini_name,
        eva_ini_name,
        ui_ini_readable,
        ui_ini,
        ui_ini_shp_refs,
    }
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
