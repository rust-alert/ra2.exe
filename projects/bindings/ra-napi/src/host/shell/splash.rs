//! 启动闪屏生命周期。

use std::time::Instant;

use ra_widgets::original_screen::OriginalScreen;
use ra_widgets::startup_splash::{self, StartupSplashPresentation};
use ra_widgets::skin::decode;
use ra_widgets::screens::page::page_resources_from_slots_with_edition;
use ra_widgets::skin::resolve;

use super::Shell;

impl Shell {
    /// 用户请求跳过闪屏；预处理完成后才进主菜单。
    pub(super) fn request_splash_skip(&mut self) {
        if self.screen != OriginalScreen::Splash {
            return;
        }
        self.splash_skip = true;
        tracing::info!("闪屏跳过已请求");
    }

    /// 闪屏每帧：保证启动画面在屏、推进预处理；期限结束或跳过后切主菜单。
    pub(super) fn tick_splash(&mut self) {
        if self.screen != OriginalScreen::Splash {
            return;
        }
        // 先保证启动画面在屏，再做菜单资源预热（预热不得切换页面、不得清空 UI 页）。
        self.ensure_startup_splash_presented();
        if !self.splash_preload_done {
            self.ensure_menu_assets();
            self.ensure_menu_text_assets();
            self.ensure_menu_audio_assets();
            // 预热主菜单 chrome：不切入 MainMenu，避免合成/清屏打穿闪屏。
            self.warm_main_menu_chrome();
            self.splash_preload_done = true;
            if !self.banner.contains("预处理完成") {
                self.banner = format!("{} · 预处理完成", self.banner);
            }
            self.refresh_shell_title();
        }
        let now = Instant::now();
        let hold_active = self.startup_splash.as_ref().is_some_and(|splash| splash.is_active(now));
        let min_ok = !hold_active;
        if self.splash_preload_done && (min_ok || self.splash_skip) {
            tracing::info!(hold_active, min = self.splash_min_secs, skip = self.splash_skip, "启动闪屏结束 → 主菜单");
            self.startup_splash = None;
            self.set_screen(OriginalScreen::MainMenu);
            self.maybe_start_slide_in();
        }
    }

    /// 在闪屏状态下预热主菜单资源缓存（不改 `screen`、不上传菜单合成页）。
    pub(super) fn warm_main_menu_chrome(&mut self) {
        let Some(assets) = self.menu_assets.as_ref()
        else {
            return;
        };
        let Some(source) = assets.source.as_ref()
        else {
            return;
        };
        let edition = assets.edition;
        let Some(page) = page_resources_from_slots_with_edition(OriginalScreen::MainMenu, edition)
        else {
            return;
        };
        let report = resolve::resolve_page(source, &page);
        let only_movie_gaps = report.missing.iter().all(|m| m.to_ascii_lowercase().ends_with(".bik"));
        if report.named > 0 && only_movie_gaps {
            let decoded = decode::decode_page_chrome(source, &page);
            tracing::info!(
                errors = decoded.errors.len(),
                chrome_ready = decoded.chrome_ready_for_enabled_buttons(&page),
                "闪屏预热主菜单 chrome · {}",
                decoded.banner_note()
            );
            self.ui_decode_cache = Some(decoded);
        }
    }

    /// 构造或复用启动闪屏 presentation，并上传到 UI 页；首次成功上传时武装最短展示期限。
    pub(super) fn ensure_startup_splash_presented(&mut self) {
        if self.startup_splash.is_none() {
            self.ensure_menu_assets();
            self.ensure_menu_text_assets();
            let client_w = self.window_width.round().max(1.0) as u32;
            let client_h = self.window_height.round().max(1.0) as u32;
            let minimum = std::time::Duration::from_secs_f64(self.splash_min_secs.max(0.0));
            let prefer_md = self.menu_assets.as_ref().and_then(|a| a.edition).is_some_and(startup_splash::prefer_md_splash);
            let built = match self.menu_assets.as_ref().and_then(|a| a.source.as_ref()) {
                None => {
                    tracing::warn!("启动闪屏 · 安装资源源未挂载，使用黑底占位");
                    StartupSplashPresentation::placeholder(client_w, client_h, prefer_md, minimum).ok()
                }
                Some(source) => {
                    match StartupSplashPresentation::build(
                        source,
                        self.menu_csf.as_ref(),
                        self.menu_font.as_ref(),
                        client_w,
                        client_h,
                        prefer_md,
                        minimum,
                    ) {
                        Ok(splash) => Some(splash),
                        Err(e) => {
                            tracing::warn!("启动闪屏构造失败 · {e} · 回退黑底占位");
                            StartupSplashPresentation::placeholder(client_w, client_h, prefer_md, minimum).ok()
                        }
                    }
                }
            };
            if let Some(splash) = built {
                let shp = splash.shp_name();
                let w = splash.image().width();
                let h = splash.image().height();
                tracing::info!(shp, pal = splash.pal_name(), prefer_md, w, h, "启动闪屏已合成");
                if !self.banner.contains(shp) {
                    self.banner = format!("{} · {shp} {w}×{h}", self.banner);
                    self.refresh_shell_title();
                }
                self.startup_splash = Some(splash);
            }
            else if !self.banner.contains("闪屏缺图") {
                self.banner = format!("{} · 闪屏缺图", self.banner);
                self.refresh_shell_title();
            }
        }

        let Some(splash) = self.startup_splash.as_ref()
        else {
            return;
        };
        if !self.renderer.has_ui_page() {
            let page = splash.image().clone();
            self.renderer.clear_preview();
            self.upload_ui_page(page);
        }
        if let Some(splash) = self.startup_splash.as_mut() {
            splash.mark_presented(Instant::now());
        }
    }
}
