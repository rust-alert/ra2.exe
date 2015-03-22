//! Wasm 宿主：窗口 / 画布、输入、装载与副作用（与 `ra-napi::host` 同构职责）。
//!
//! 页面组合在 `ra-widgets`；本模块只做平台胶水。产品对标面在 `sites/playground`。

#![allow(missing_docs)]

pub mod audio;
pub mod boot;
pub mod install;
pub mod load_job;
pub mod present;
