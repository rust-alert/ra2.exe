//! 装载任务：进度快照（浏览器侧对标 `ra-napi` load_job）。
//!
//! Wasm 单线程：`prepare` 仍同步执行，但通过本模块暴露阶段进度供 UI 轮询/展示。

use std::cell::{Cell, RefCell};

use wasm_bindgen::prelude::*;

#[derive(Debug, Clone)]
struct ProgressInner {
    ratio: f32,
    stage: String,
}

impl Default for ProgressInner {
    fn default() -> Self {
        Self { ratio: 0.0, stage: "idle".into() }
    }
}

thread_local! {
    static BUSY: Cell<bool> = const { Cell::new(false) };
    static PROGRESS: RefCell<ProgressInner> = RefCell::new(ProgressInner::default());
}

/// 是否有进行中的装载任务。
#[wasm_bindgen(js_name = loadJobBusy)]
pub fn is_busy() -> bool {
    BUSY.with(Cell::get)
}

/// 当前装载进度快照。
#[wasm_bindgen(js_name = LoadProgress)]
#[derive(Debug, Clone)]
pub struct LoadProgress {
    ratio: f32,
    stage: String,
}

#[wasm_bindgen]
impl LoadProgress {
    /// 0..1 阶段比例。
    #[wasm_bindgen(getter)]
    pub fn ratio(&self) -> f32 {
        self.ratio
    }

    /// 人类可读阶段名。
    #[wasm_bindgen(getter)]
    pub fn stage(&self) -> String {
        self.stage.clone()
    }
}

/// 读取最新进度快照。
#[wasm_bindgen(js_name = loadJobProgress)]
pub fn progress() -> LoadProgress {
    PROGRESS.with(|slot| {
        let p = slot.borrow();
        LoadProgress { ratio: p.ratio, stage: p.stage.clone() }
    })
}

pub(crate) fn begin(stage: &str) {
    BUSY.with(|b| b.set(true));
    report(0.05, stage);
}

pub(crate) fn report(ratio: f32, stage: &str) {
    PROGRESS.with(|slot| {
        let mut p = slot.borrow_mut();
        p.ratio = ratio.clamp(0.0, 1.0);
        p.stage = stage.to_string();
    });
}

pub(crate) fn finish_ok() {
    report(1.0, "完成");
    BUSY.with(|b| b.set(false));
}

pub(crate) fn finish_err() {
    report(1.0, "装载失败");
    BUSY.with(|b| b.set(false));
}
