//! 浏览器音频宿主（Web Audio）。
//!
//! 浏览器常要求用户手势后才能 `resume`；本模块只管设备就绪，不播具体曲目。

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{AudioContext, AudioContextState};

#[cfg(target_arch = "wasm32")]
thread_local! {
    static AUDIO: RefCell<Option<AudioContext>> = const { RefCell::new(None) };
}

/// 音频输出是否已运行（`AudioContext` 为 `running`）。
pub fn is_ready() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        AUDIO.with(|slot| slot.borrow().as_ref().is_some_and(|ctx| ctx.state() == AudioContextState::Running))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// 当前音频状态字符串（`running` / `suspended` / `closed` / `none`）。
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = audioState)]
pub fn audio_state() -> String {
    AUDIO.with(|slot| match slot.borrow().as_ref() {
        Some(ctx) => match ctx.state() {
            AudioContextState::Running => "running".into(),
            AudioContextState::Suspended => "suspended".into(),
            AudioContextState::Closed => "closed".into(),
            _ => "unknown".into(),
        },
        None => "none".into(),
    })
}

/// 创建（若需要）并尝试 `resume` AudioContext；须由用户手势触发。
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = resumeAudio)]
pub async fn resume_audio() -> Result<String, JsValue> {
    AUDIO.with(|slot| -> Result<(), JsValue> {
        if slot.borrow().is_none() {
            let created = AudioContext::new()?;
            *slot.borrow_mut() = Some(created);
        }
        Ok(())
    })?;

    let promise = AUDIO.with(|slot| {
        let guard = slot.borrow();
        let ctx = guard.as_ref().ok_or_else(|| JsValue::from_str("audio context missing"))?;
        ctx.resume()
    })?;
    JsFuture::from(promise).await?;
    Ok(audio_state())
}

/// 音频是否已就绪（供 JS 轮询）。
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = audioReady)]
pub fn audio_ready() -> bool {
    is_ready()
}
