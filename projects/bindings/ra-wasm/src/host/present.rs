//! wgpu 画布呈现（Wasm 宿主）。
//!
//! `attach_canvas` 成功后 `supports_present` 为真；首帧清屏验证交换链。
//! 表面附着与帧提交仅在 `wasm32` 编译；宿主测试目标只保留探测桩。

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use ra_renderer::Renderer;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

#[cfg(target_arch = "wasm32")]
struct PresentState {
    renderer: Renderer,
    ready: bool,
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static PRESENT: RefCell<Option<PresentState>> = const { RefCell::new(None) };
}

/// 是否已附着可用的 wgpu 表面。
pub fn is_ready() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        PRESENT.with(|slot| slot.borrow().as_ref().is_some_and(|s| s.ready))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// 当前后端标签（未附着则为空）。
pub fn backend_label() -> Option<&'static str> {
    #[cfg(target_arch = "wasm32")]
    {
        PRESENT.with(|slot| slot.borrow().as_ref().and_then(|s| s.renderer.backend_label()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// 将 HTML canvas 附着为 wgpu 表面并提交一帧清屏。
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = attachCanvas)]
pub async fn attach_canvas(canvas: HtmlCanvasElement) -> Result<(), JsValue> {
    if is_ready() {
        return Ok(());
    }
    let width = canvas.width().max(1);
    let height = canvas.height().max(1);
    let mut renderer = Renderer::new();
    renderer
        .attach_canvas(canvas)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    renderer.resize(width, height);
    renderer.draw_frame(None);
    PRESENT.with(|slot| {
        *slot.borrow_mut() = Some(PresentState { renderer, ready: true });
    });
    Ok(())
}

/// 调整已附着表面尺寸并重绘一帧。
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = resizePresent)]
pub fn resize_present(width: u32, height: u32) -> Result<(), JsValue> {
    PRESENT.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(state) = guard.as_mut()
        else {
            return Err(JsValue::from_str("present surface not attached"));
        };
        state.renderer.resize(width, height);
        state.renderer.draw_frame(None);
        Ok(())
    })
}

/// 提交一帧（当前为清屏 / 已上传纹理的合成；无对局环）。
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = presentFrame)]
pub fn present_frame() -> Result<(), JsValue> {
    PRESENT.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(state) = guard.as_mut()
        else {
            return Err(JsValue::from_str("present surface not attached"));
        };
        state.renderer.draw_frame(None);
        Ok(())
    })
}

/// 当前 wgpu 后端标签字符串。
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = presentBackend)]
pub fn present_backend() -> String {
    backend_label().unwrap_or("none").to_string()
}
