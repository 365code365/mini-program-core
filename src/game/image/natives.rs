//! Image 的原生入口：`__game_img_new()`、`__game_img_src(id, src)`、`__game_img_free(id)`。
//! 结果由 GameEngine 在 pump 里交回 JS（`__gameImageDone(id, json)`，见 game/frame.rs 与 game-image.js）。

use super::ImageStore;
use crate::js::JsRuntime;
use rquickjs::{IntoJs, Value};
use std::cell::RefCell;
use std::rc::Rc;

pub fn register(rt: &JsRuntime, store: Rc<RefCell<ImageStore>>) -> Result<(), String> {
    let s = store.clone();
    rt.register_raw("__game_img_new", move |ctx, _| (s.borrow_mut().create() as f64).into_js(&ctx))?;
    let s = store.clone();
    rt.register_raw("__game_img_src", move |ctx, args| {
        let id = args.0.first().and_then(|v| v.as_number()).unwrap_or(0.0) as u32;
        let src = args.0.get(1).and_then(|v| v.as_string()).and_then(|v| v.to_string().ok()).unwrap_or_default();
        s.borrow_mut().set_src(id, &src);
        Ok(Value::new_undefined(ctx))
    })?;
    rt.register_raw("__game_img_free", move |ctx, args| {
        store.borrow_mut().free(args.0.first().and_then(|v| v.as_number()).unwrap_or(0.0) as u32);
        Ok(Value::new_undefined(ctx))
    })?;
    Ok(())
}
