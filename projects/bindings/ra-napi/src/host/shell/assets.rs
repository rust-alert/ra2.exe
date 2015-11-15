//! UI / CSF / FNT 资源装载与解析备注。

use std::time::Instant;

use ra_assets::{CsfFile, FntFile, IniDocument};
use ra_renderer::RgbaImage;
use ra_types::AssetSource;
use ra_widgets::fs_source::GameAssetSource;
use ra_widgets::original_screen::OriginalScreen;
use ra_widgets::skin::assets::load_menu_ui_assets;
use ra_widgets::skin::decode;
use ra_widgets::chrome::movie::MenuMoviePlayer;
use ra_widgets::screens::page::{page_resources_for_load_screen, page_resources_from_slots_with_edition};
use ra_widgets::skin::resolve;

use super::Shell;

impl Shell {
    /// 从挂载源解码 PCX → RGBA；品红 `(255,0,255)` 作色键透明（旗标索引未必为 0）。
    pub(super) fn load_pcx_rgba(source: &ra_widgets::fs_source::GameAssetSource, name: &str) -> Option<RgbaImage> {
        let bytes = source.read(name).ok()?;
        let pcx = ra_assets::parse_pcx(&bytes).ok()?;
        let mut rgba = pcx.rgba;
        for px in rgba.chunks_exact_mut(4) {
            if px[0] == 255 && px[1] == 0 && px[2] == 255 {
                px[3] = 0;
            }
        }
        RgbaImage::from_raw(pcx.width, pcx.height, rgba)
    }

    pub(super) fn ensure_menu_assets(&mut self) {
        if self.menu_assets.is_some() {
            return;
        }
        let assets = load_menu_ui_assets();
        tracing::info!(
            ui_ini = ?assets.ui_ini_name,
            ui_ini_ok = assets.ui_ini_readable,
            ui_sections = assets.ui_ini.as_ref().map(|d| d.sections.len()),
            ui_shp_refs = assets.ui_ini_shp_refs.len(),
            has_source = assets.source.is_some(),
            "{}",
            assets.note
        );
        self.banner = assets.note.clone();
        self.menu_assets = Some(assets);
        self.refresh_ui_resolve_note();
    }

    /// 对当前页已声明资源名做可读性检查，并尝试解码 chrome（不绘制）。
    pub(super) fn refresh_ui_resolve_note(&mut self) {
        let Some(assets) = self.menu_assets.as_ref()
        else {
            return;
        };
        let Some(source) = assets.source.as_ref()
        else {
            return;
        };
        let edition = assets.edition;
        let page = if self.screen == OriginalScreen::LoadScreen {
            page_resources_for_load_screen(&self.skirmish.side, self.window_width as u32, |name| source.resolve(name).is_some())
        }
        else {
            page_resources_from_slots_with_edition(self.screen, edition)
        };
        let Some(page) = page
        else {
            return;
        };
        let report = resolve::resolve_page(source, &page);
        tracing::info!(
            screen = self.screen.as_str(),
            named = report.named,
            readable = report.readable,
            missing = report.missing.len(),
            "{}",
            report.banner_note()
        );
        let mut banner = if report.named == 0 {
            if assets.note.contains("槽位未填") { assets.note.clone() } else { format!("{} · {}", assets.note, report.banner_note()) }
        }
        else {
            format!("{} · {}", assets.note, report.banner_note())
        };

        // 影片缺失不挡 chrome 解码；仅非 BIK 缺口才清空解码缓存。
        // 装载页：只要国家背景可读就解码（进度条 / 失败钮可缺）。
        let only_movie_gaps = report.missing.iter().all(|m| m.to_ascii_lowercase().ends_with(".bik"));
        let load_bg_name = page.background.as_ref().map(|b| b.name.to_ascii_lowercase());
        let load_bg_ok = self.screen == OriginalScreen::LoadScreen
            && load_bg_name.as_ref().is_some_and(|bg| !report.missing.iter().any(|m| m.eq_ignore_ascii_case(bg)));
        if report.named > 0 && (only_movie_gaps || load_bg_ok) {
            let decoded = decode::decode_page_chrome(source, &page);
            tracing::info!(
                screen = self.screen.as_str(),
                errors = decoded.errors.len(),
                chrome_ready = decoded.chrome_ready_for_enabled_buttons(&page),
                "{}",
                decoded.banner_note()
            );
            for err in &decoded.errors {
                tracing::warn!(screen = self.screen.as_str(), "UI 解码失败 · {err}");
            }
            banner = format!("{banner} · {}", decoded.banner_note());
            self.ui_decode_cache = Some(decoded);
        }
        else {
            self.ui_decode_cache = None;
        }

        if let Some(movie) = page.movie.as_ref() {
            match source.read(&movie.name) {
                Ok(bytes) => match MenuMoviePlayer::open(&movie.name, bytes) {
                    Ok(player) => {
                        tracing::info!(
                            name = %player.name(),
                            "主菜单影片播放器已就绪（自研 Bink）"
                        );
                        banner = format!("{banner} · {} 已解首帧", movie.name);
                        self.menu_movie = Some(player);
                        self.menu_movie_clock = Some(Instant::now());
                    }
                    Err(e) => {
                        tracing::warn!(name = %movie.name, "影片播放器启动失败 · {e}");
                        banner = format!("{banner} · {} 解码失败", movie.name);
                        self.menu_movie = None;
                        self.menu_movie_clock = None;
                    }
                },
                Err(_) => {
                    tracing::warn!(name = %movie.name, "影片不可读");
                    self.menu_movie = None;
                    self.menu_movie_clock = None;
                }
            }
        }
        else {
            self.menu_movie = None;
            self.menu_movie_clock = None;
        }

        self.banner = banner;
    }

    pub(super) fn ensure_menu_text_assets(&mut self) {
        if (self.menu_font.is_some() || self.menu_font_tried) && (self.menu_csf.is_some() || self.menu_csf_tried) {
            return;
        }
        let source = self.menu_assets.as_ref().and_then(|a| a.source.as_ref());

        if self.menu_font.is_none() && !self.menu_font_tried {
            self.menu_font_tried = true;
            match source.and_then(|s| s.read("game.fnt").ok()) {
                Some(bytes) => match FntFile::parse(&bytes) {
                    Ok(fnt) => {
                        tracing::info!(glyphs = fnt.glyph_count(), "菜单字体已解析 · game.fnt");
                        self.menu_font = Some(fnt);
                    }
                    Err(e) => tracing::warn!("game.fnt 解析失败 · {e}"),
                },
                None => tracing::warn!("game.fnt 不可读"),
            }
        }
        if self.menu_csf.is_none() && !self.menu_csf_tried {
            self.menu_csf_tried = true;
            let csf_bytes = source.and_then(|s| {
                // 资料片优先 `ra2md.csf`，再回退原版 `ra2.csf`。
                for name in ["ra2md.csf", "ra2.csf"] {
                    if let Ok(bytes) = s.read(name) {
                        return Some((name, bytes));
                    }
                }
                None
            });
            match csf_bytes {
                Some((name, bytes)) => match CsfFile::parse(&bytes) {
                    Ok(csf) => {
                        tracing::info!(entries = csf.len(), file = name, "菜单文案表已解析");
                        self.menu_csf = Some(csf);
                    }
                    Err(e) => tracing::warn!("{name} 解析失败 · {e}"),
                },
                None => tracing::warn!("未找到可读的 ra2.csf / ra2md.csf"),
            }
        }
    }

    /// 从挂载源读逻辑文件名。
    pub(super) fn read_asset_bytes(&self, name: &str) -> Option<Vec<u8>> {
        self.menu_assets.as_ref().and_then(|a| a.source.as_ref()).and_then(|s| s.read(name).ok())
    }

    /// 解析 INI；失败时仍可用 `soft_ini_get`。
    pub(super) fn read_ini_doc(&self, name: &str) -> Option<IniDocument> {
        let bytes = self.read_asset_bytes(name)?;
        match IniDocument::parse(&bytes) {
            Ok(doc) => Some(doc),
            Err(e) => {
                tracing::debug!(%name, error = %e, "INI 严格解析失败，改用宽松扫描");
                None
            }
        }
    }
}
