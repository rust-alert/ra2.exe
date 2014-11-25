//! 遭遇战大厅地图预览：后台合成缩略图，主线程轮询。

use std::{
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

use ra_renderer::RgbaImage;

use super::boot::preview_install_boot_map;
use ra_components::ui_assets::downscale_to_fit;

/// 一张地图预览任务的结果。
pub struct MapPreviewResult {
    /// 请求的地图文件名。
    pub map_name: String,
    /// 装载备注。
    pub note: String,
    /// 已缩小的缩略图（失败为 `None`）。
    pub image: Option<RgbaImage>,
}

/// 后台预览句柄。
pub struct PreviewJob {
    map_name: String,
    rx: Receiver<MapPreviewResult>,
}

impl PreviewJob {
    /// 启动指定地图的预览合成（不阻塞）。
    pub fn start(map_name: String) -> Self {
        let (tx, rx) = mpsc::channel();
        let name_for_thread = map_name.clone();
        thread::Builder::new()
            .name("ra2-map-preview".into())
            .spawn(move || {
                let result = match preview_install_boot_map(&name_for_thread) {
                    Some((note, image)) => {
                        let thumb = downscale_to_fit(&image, 320, 200).unwrap_or(image);
                        MapPreviewResult { map_name: name_for_thread, note, image: Some(thumb) }
                    }
                    None => MapPreviewResult { map_name: name_for_thread, note: "preview:无".into(), image: None },
                };
                let _ = tx.send(result);
            })
            .expect("spawn preview thread");
        Self { map_name, rx }
    }

    /// 当前任务对应的地图名。
    pub fn map_name(&self) -> &str {
        &self.map_name
    }

    /// 非阻塞取结果。
    pub fn try_take(&self) -> Result<Option<MapPreviewResult>, ()> {
        match self.rx.try_recv() {
            Ok(v) => Ok(Some(v)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(()),
        }
    }
}
