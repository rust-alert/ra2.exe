//! 遭遇战装载任务：后台线程跑装载，主线程只轮询结果。

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::boot::{BootResult, boot_from_install_with_request};
use crate::skirmish_setup::SkirmishBootRequest;

/// 后台装载句柄。
pub struct LoadJob {
    rx: Receiver<BootResult>,
}

impl LoadJob {
    /// 启动安装目录遭遇战装载（不阻塞调用方）。
    ///
    /// 请求携带大厅所选地图、阵营与难度。
    pub fn start_install_boot(request: SkirmishBootRequest) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("ra2-skirmish-load".into())
            .spawn(move || {
                let boot = boot_from_install_with_request(request);
                let _ = tx.send(boot);
            })
            .expect("spawn load thread");
        Self { rx }
    }

    /// 测试场景装载（仅 test-harness）。
    #[cfg(feature = "test-harness")]
    pub fn start_test_scene(scene: String) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("ra2-test-load".into())
            .spawn(move || {
                let boot = match crate::test_boot::boot_scene(&scene) {
                    Ok(t) => BootResult {
                        note: t.note,
                        engine: Some(t.engine),
                        session: Some(t.session),
                        preview: t.preview,
                    },
                    Err(e) => BootResult {
                        note: format!("装载失败: {e}"),
                        engine: None,
                        session: None,
                        preview: None,
                    },
                };
                let _ = tx.send(boot);
            })
            .expect("spawn load thread");
        Self { rx }
    }

    /// 非阻塞取结果。
    pub fn try_take(&self) -> Result<Option<BootResult>, ()> {
        match self.rx.try_recv() {
            Ok(boot) => Ok(Some(boot)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(()),
        }
    }
}
