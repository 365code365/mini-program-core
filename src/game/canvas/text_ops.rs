//! 画布上的文字：25 fillText、26 strokeText、27 measureText、34 设 font（game-doc/03 §2.2、05 GT-F2/F4）。
//! font 按 CSS 简写解析后存在 `Slot::font`；字族 / 字重 / 真实基线见 renderer/components/canvas/text_font.rs。

use super::Slot;
use crate::renderer::components::FontSpec;
use rquickjs::{Ctx, IntoJs, Object, Result as JsResult, Value};

fn num(v: Option<&Value>) -> f32 {
    v.and_then(|x| x.as_number()).unwrap_or(f64::NAN) as f32
}

fn text(v: Option<&Value>) -> String {
    v.and_then(|x| x.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default()
}

pub fn text_op<'js>(ctx: &Ctx<'js>, slot: &mut Slot, op: i32, a: &[Value<'js>]) -> JsResult<Value<'js>> {
    match op {
        // 设 font：解析成功返回读回值（规范化的串），失败返回 null（HTML：保留旧值）
        34 => match FontSpec::parse(&text(a.first())) {
            Some(f) => {
                // 老路径（裁剪、虚线等不涉及字体）与 measure 的回退都看字号：一并设上
                slot.ctx.set_font(&format!("{}px", f.size));
                let css = f.to_css();
                slot.font = f;
                css.into_js(ctx)
            }
            None => Ok(Value::new_null(ctx.clone())),
        },
        25 | 26 => {
            let t = text(a.first());
            let (x, y) = (num(a.get(1)), num(a.get(2)));
            // maxWidth 没给是 undefined；给了 NaN / 负数按 HTML 什么也不画（在 fill_text_font 里判）
            let mw = a.get(3).filter(|v| !v.is_undefined()).map(|v| v.as_number().unwrap_or(f64::NAN) as f32);
            if x.is_finite() && y.is_finite() && !t.is_empty() {
                let font = slot.font.clone();
                slot.ctx.draw_op(|g| g.fill_text_font(&font, &t, x, y, mw, op == 26));
            }
            Ok(Value::new_undefined(ctx.clone()))
        }
        27 => {
            let m = slot.ctx.measure_text_font(&slot.font, &text(a.first()));
            let o = Object::new(ctx.clone())?;
            o.set("width", m.width as f64)?;
            o.set("actualBoundingBoxLeft", m.left as f64)?;
            o.set("actualBoundingBoxRight", m.right as f64)?;
            o.set("actualBoundingBoxAscent", m.ascent as f64)?;
            o.set("actualBoundingBoxDescent", m.descent as f64)?;
            o.set("fontBoundingBoxAscent", m.font_ascent as f64)?;
            o.set("fontBoundingBoxDescent", m.font_descent as f64)?;
            Ok(o.into_value())
        }
        _ => Ok(Value::new_undefined(ctx.clone())),
    }
}
