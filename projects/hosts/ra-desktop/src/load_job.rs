//! 遭遇战装载任务：后台线程跑装载，主线程只轮询结果与阶段进度。

use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, TryRecvError},
    },
    thread,
};

use crate::{
    boot::{BootResult, boot_from_install_with_progress},
    skirmish_setup::SkirmishBootRequest,
};

/// 装载阶段可见进度（主线程只读最新快照）。
#[derive(Debug, Clone)]
pub struct LoadProgress {
    /// 0..1 阶段比例（按装载步骤推进，非时间估算）。
    pub ratio: f32,
    /// 人类可读阶段名。
    pub stage: String,
}

impl Default for LoadProgress {
    fn default() -> Self {
        Self { ratio: 0.05, stage: "排队".into() }
    }
}

/// 后台装载句柄。
pub struct LoadJob {
    rx: Receiver<BootResult>,
    progress: Arc<Mutex<LoadProgress>>,
}

impl LoadJob {
    /// 启动安装目录遭遇战装载（不阻塞调用方）。
    ///
    /// 请求携带大厅所选地图、阵营与难度。
    pub fn start_install_boot(request: SkirmishBootRequest) -> Self {
        let (tx, rx) = mpsc::channel();
        let progress = Arc::new(Mutex::new(LoadProgress::default()));
        let progress_worker = Arc::clone(&progress);
        thread::Builder::new()
            .name("ra2-skirmish-load".into())
            .spawn(move || {
                let report = |ratio: f32, stage: &str| {
                    if let Ok(mut slot) = progress_worker.lock() {
                        slot.ratio = ratio.clamp(0.0, 1.0);
                        slot.stage = stage.to_string();
                    }
                };
                let boot = boot_from_install_with_progress(request, report);
                if boot.is_ready() {
                    report(1.0, "完成");
                }
                else {
                    report(1.0, "装载失败");
                }
                let _ = tx.send(boot);
            })
            .expect("spawn load thread");
        Self { rx, progress }
    }

    /// 测试场景装载（仅 test-harness）。
    #[cfg(feature = "test-harness")]
    pub fn start_test_scene(scene: String) -> Self {
        let (tx, rx) = mpsc::channel();
        let progress = Arc::new(Mutex::new(LoadProgress { ratio: 0.2, stage: "测试场景".into() }));
        let progress_worker = Arc::clone(&progress);
        thread::Builder::new()
            .name("ra2-test-load".into())
            .spawn(move || {
                if let Ok(mut slot) = progress_worker.lock() {
                    slot.ratio = 0.55;
                    slot.stage = "打开会话".into();
                }
                let boot = match crate::test_boot::boot_scene(&scene) {
                    Ok(t) => BootResult { note: t.note, engine: Some(t.engine), session: Some(t.session), preview: t.preview },
                    Err(e) => BootResult { note: format!("装载失败: {e}"), engine: None, session: None, preview: None },
                };
                if let Ok(mut slot) = progress_worker.lock() {
                    slot.ratio = 1.0;
                    slot.stage = if boot.is_ready() { "完成".into() } else { "装载失败".into() };
                }
                let _ = tx.send(boot);
            })
            .expect("spawn load thread");
        Self { rx, progress }
    }

    /// 非阻塞取结果。
    pub fn try_take(&self) -> Result<Option<BootResult>, ()> {
        match self.rx.try_recv() {
            Ok(boot) => Ok(Some(boot)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(()),
        }
    }

    /// 读取最新装载阶段进度。
    pub fn progress(&self) -> LoadProgress {
        self.progress.lock().map(|g| g.clone()).unwrap_or_default()
    }
}
