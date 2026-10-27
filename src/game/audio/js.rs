//! 音频的 JS 入口与读源（从 mod.rs 搬出来，内容未改）
use crate::audio_ctx::decode::{self, Pcm};
use super::GameAudio;

/// 读一个音频源：包内路径按包根（跑出包根拒绝）、`wxfile://`、http(s)
pub(super) fn load_src(src: &str, root: &std::path::Path) -> Result<Pcm, (i32, String)> {
    if src.starts_with("http://") || src.starts_with("https://") || src.starts_with(crate::wxfile::SCHEME) {
        return decode::load(src);
    }
    let path = crate::game::image::loader::package_path(src, root).ok_or((10003, format!("路径不能跑出包根：{src}")))?;
    let bytes = std::fs::read(&path).map_err(|e| (10003, format!("{e}: {src}")))?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    decode::decode_bytes(bytes, &ext)
}

/// JS 入口：`__game_audio(op, id, json)`；`__game_audio_session(json)`（setInnerAudioOption，返回生效后的三项）
pub fn register(rt: &crate::js::JsRuntime, audio: std::rc::Rc<std::cell::RefCell<GameAudio>>) -> Result<(), String> {
    use rquickjs::IntoJs;
    let a2 = audio.clone();
    rt.register_raw("__game_audio_session", move |ctx, a| {
        let text = a.0.first().and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        a2.borrow_mut().set_session_options(&v).to_string().into_js(&ctx)
    })?;
    rt.register_raw("__game_audio", move |ctx, a| {
        let text = |i: usize| a.0.get(i).and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
        let id = text(1).parse().unwrap_or(0);
        audio.borrow_mut().call(&text(0), id, &text(2)).into_js(&ctx)
    })
}
