//! 读取世界状态，经现代 GPU（wgpu）绘制。
//!
//! 原生后端：DX12 / Vulkan / Metal。Wasm：WebGL2。
//! 本 crate **故意不**实现 DirectDraw。

use ra_types::GameEdition;
use ra_world::World;

#[derive(Debug, Default)]
pub struct Renderer {
    pub frames: u64,
}

impl Renderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// 占位：等壳里接上 wgpu 表面后再画真帧。
    pub fn draw_frame(&mut self, world: &World) {
        let _ = world.edition;
        self.frames = self.frames.wrapping_add(1);
    }

    pub fn backend_hint(edition: GameEdition) -> &'static str {
        let _ = edition;
        "wgpu"
    }
}
