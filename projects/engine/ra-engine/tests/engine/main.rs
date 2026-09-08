//! `ra-engine` 集成测试根（单一 Cargo 测试二进制 `engine`）。
//!
//! 目录与 `src/` 七轴对齐：runtime / state / spatial / gameplay / lifecycle /
//! presentation / persistence。

mod common;
mod gameplay;
mod lifecycle;
mod persistence;
mod presentation;
mod runtime;
mod spatial;
mod state;
