//! 画布的原生入口（只注册在小游戏的运行时上）。
//!
//! 一个分派函数 `__game_2d(id, op, ...args)` + 几个画布管理函数。为什么不按方法各注册一个：
//! 每个原生函数都是 QuickJS 里的一个 C 函数对象，调用成本一样；一个分派函数让 JS 外壳（game-canvas.js）
//! 只认一个名字，op 编号与这里的 match 一一对应，加一个方法只改两处。
//! 热路径（fillRect / 路径 / 变换）直调 `Canvas2DContext` 的方法；渐变、阴影、合成模式这类不在热路径上的设置
//! 复用 JSON 解释器（commands.rs）那份解析，两条路不会各写一份而漂移。
//!
//! op 编号表（改了要同时改 game-canvas.js 的 OP）：
//! 1 fillRect 2 strokeRect 3 clearRect 4 beginPath 5 closePath 6 moveTo 7 lineTo 8 arc 9 rect
//! 10 quadraticCurveTo 11 bezierCurveTo 12 arcTo 13 ellipse 14 fill 15 stroke 16 clip 17 save 18 restore
//! 19 translate 20 rotate 21 scale 22 setTransform 23 transform 24 resetTransform
//! 25 fillText 26 strokeText 27 measureText
//! 30 fillStyle 31 strokeStyle 32 lineWidth 33 globalAlpha 34 font 35 textAlign 36 textBaseline
//! 37 lineCap 38 lineJoin 39 miterLimit 40 setLineDash 41 lineDashOffset 42 shadow 43 composite
//! 44 gradient/pattern（JSON 描述，fill 或 stroke） 45 getImageData 46 putImageData
//! 47 drawImage 48 图案（源是 Image / Canvas，见 image_ops.rs）

use super::GameCanvases;
use crate::js::binary::{array_buffer, require_bytes};
use crate::js::JsRuntime;
use rquickjs::{Ctx, Exception, IntoJs, Result as JsResult, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// 状态栈深度上限：恶意 / 写错的 save() 不配 restore() 时吃内存（game-doc/02 §5.2）
pub const MAX_SAVE_DEPTH: usize = 1024;

fn num(v: Option<&Value>) -> f32 {
    v.and_then(|x| x.as_number()).unwrap_or(f64::NAN) as f32
}

fn text(v: Option<&Value>) -> Option<String> {
    v.and_then(|x| x.as_string()).and_then(|s| s.to_string().ok())
}

/// HTML 语义：任一坐标 / 尺寸不是有限数的绘制调用整条忽略（不是画在 0 上）
fn finite(xs: &[f32]) -> bool {
    xs.iter().all(|x| x.is_finite())
}

pub fn register(rt: &JsRuntime, canvases: Rc<RefCell<GameCanvases>>, images: Rc<RefCell<crate::game::image::ImageStore>>) -> Result<(), String> {
    let c = canvases.clone();
    rt.register_raw("__game_canvas_new", move |ctx, _args| {
        let (id, w, h, main) = c.borrow_mut().create();
        vec![id as f64, w as f64, h as f64, if main { 1.0 } else { 0.0 }].into_js(&ctx)
    })?;
    let c = canvases.clone();
    rt.register_raw("__game_canvas_resize", move |ctx, args| {
        let id = num(args.0.first()) as u32;
        let (w, h) = (num(args.0.get(1)) as f64, num(args.0.get(2)) as f64);
        match c.borrow_mut().resize(id, w, h) {
            Some((w, h)) => vec![w as f64, h as f64].into_js(&ctx),
            None => Ok(Value::new_undefined(ctx)),
        }
    })?;
    let c = canvases.clone();
    rt.register_raw("__game_canvas_free", move |ctx, args| {
        c.borrow_mut().free(num(args.0.first()) as u32);
        Ok(Value::new_undefined(ctx))
    })?;
    // getContext(kind)：一个画布只能拿一种上下文（第一次拿到什么之后就只能是什么），返回是否给出
    let c = canvases.clone();
    rt.register_raw("__game_canvas_ctx", move |ctx, args| {
        let id = num(args.0.first()) as u32;
        let kind = match text(args.0.get(1)).as_deref() {
            Some("2d") => 1u8,
            // webgl / webgl2 在 G8；在那之前返回 null（与「设备不支持」同形）
            _ => 0,
        };
        let mut cs = c.borrow_mut();
        let ok = match cs.get(id) {
            Some(s) if kind != 0 && (s.kind == 0 || s.kind == kind) => {
                if s.kind == 0 {
                    // HTML 默认基线是 alphabetic（小程序老接口的默认是 middle，这里按 HTML Canvas 来）
                    s.ctx.set_text_baseline("alphabetic");
                }
                s.kind = kind;
                true
            }
            _ => false,
        };
        ok.into_js(&ctx)
    })?;
    // toDataURL(type, quality) → data URI（读后备缓冲，不含覆盖层）
    let c = canvases.clone();
    rt.register_raw("__game_canvas_encode", move |ctx, args| {
        let id = num(args.0.first()) as u32;
        let jpeg = text(args.0.get(1)).map(|t| t.contains("jpeg") || t.contains("jpg")).unwrap_or(false);
        let q = num(args.0.get(2));
        let mut cs = c.borrow_mut();
        let out = match cs.get(id) {
            Some(s) if s.w > 0 && s.h > 0 => match s.ctx.encode(jpeg, if q.is_finite() { q } else { 0.92 }, None, (0, 0), (1.0, 1.0)) {
                Ok(b) => format!("data:image/{};base64,{}", if jpeg { "jpeg" } else { "png" }, crate::wxfs::codec::b64(&b)),
                Err(_) => "data:,".into(),
            },
            // HTML：宽或高为 0 的画布 toDataURL 是 "data:,"
            _ => "data:,".into(),
        };
        out.into_js(&ctx)
    })?;
    // toTempFilePath(Sync)：{x,y,width,height,destWidth,destHeight,fileType,quality} → {tempFilePath} | {err}
    let c = canvases.clone();
    rt.register_raw("__game_canvas_save", move |ctx, args| {
        let id = num(args.0.first()) as u32;
        let o: serde_json::Value = text(args.0.get(1)).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        let f = |k: &str| o.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let jpeg = o.get("fileType").and_then(|v| v.as_str()) == Some("jpg");
        let region = (f("width") > 0.0 && f("height") > 0.0).then(|| (f("x") as f32, f("y") as f32, f("width") as u32, f("height") as u32));
        let mut cs = c.borrow_mut();
        let r = match cs.get(id) {
            Some(s) => s.ctx.encode(jpeg, o.get("quality").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32, region, (f("destWidth") as u32, f("destHeight") as u32), (1.0, 1.0)),
            None => Err("canvas not found".into()),
        };
        let out = match r {
            Ok(bytes) => {
                let (uri, real) = crate::wxfile::new_temp(if jpeg { "jpg" } else { "png" });
                if let Some(d) = real.parent() {
                    std::fs::create_dir_all(d).ok();
                }
                match std::fs::write(&real, bytes) {
                    Ok(()) => serde_json::json!({ "tempFilePath": uri }),
                    Err(e) => serde_json::json!({ "err": e.to_string() }),
                }
            }
            Err(e) => serde_json::json!({ "err": e }),
        };
        out.to_string().into_js(&ctx)
    })?;
    let c = canvases;
    rt.register_raw("__game_2d", move |ctx, args| {
        let id = num(args.0.first()) as u32;
        let op = num(args.0.get(1)) as i32;
        let a = &args.0[2.min(args.0.len())..];
        let mut cs = c.borrow_mut();
        let is_main = cs.is_main(id);
        if op == 47 || op == 48 {
            // 源（Image / 另一块画布）要在拿目标画布之前取
            let src = super::image_ops::source(&mut cs, &mut images.borrow_mut(), num(a.first()) as i32, num(a.get(1)) as u32, id);
            let Some(slot) = cs.get(id) else { return Ok(Value::new_undefined(ctx)) };
            let Some(src) = src else { return false.into_js(&ctx) };
            if op == 47 {
                super::image_ops::draw_image(&mut slot.ctx, &src, a);
                if is_main {
                    cs.main_dirty = true;
                }
                return Ok(Value::new_undefined(ctx));
            }
            let stroke = a.get(3).and_then(|v| v.as_bool()).unwrap_or(false);
            return super::image_ops::set_pattern(&mut slot.ctx, &src, &text(a.get(2)).unwrap_or_default(), stroke).into_js(&ctx);
        }
        let Some(slot) = cs.get(id) else { return Ok(Value::new_undefined(ctx)) };
        let r = op_2d(&ctx, &mut slot.ctx, op, a)?;
        // 只有真的会改像素的调用才把主画布标脏
        if is_main && matches!(op, 1..=3 | 14 | 15 | 25 | 26 | 46) {
            cs.main_dirty = true;
        }
        Ok(r)
    })?;
    Ok(())
}

fn op_2d<'js>(ctx: &Ctx<'js>, g: &mut crate::renderer::components::Canvas2DContext, op: i32, a: &[Value<'js>]) -> JsResult<Value<'js>> {
    let n = |i: usize| num(a.get(i));
    let undef = Value::new_undefined(ctx.clone());
    match op {
        1 | 2 | 3 => {
            let (x, y, w, h) = (n(0), n(1), n(2), n(3));
            if finite(&[x, y, w, h]) {
                g.draw_op(|g| match op {
                    1 => g.fill_rect(x, y, w, h),
                    2 => g.stroke_rect(x, y, w, h),
                    _ => g.clear_rect(x, y, w, h),
                });
            }
        }
        4 => g.begin_path(),
        5 => g.close_path(),
        6 | 7 if finite(&[n(0), n(1)]) => {
            if op == 6 { g.move_to(n(0), n(1)) } else { g.line_to(n(0), n(1)) }
        }
        8 if finite(&[n(0), n(1), n(2), n(3), n(4)]) => {
            if n(2) < 0.0 {
                return Err(Exception::throw_range(ctx, "arc: 半径不能为负（IndexSizeError）"));
            }
            g.arc(n(0), n(1), n(2), n(3), n(4), a.get(5).and_then(|v| v.as_bool()).unwrap_or(false));
        }
        9 if finite(&[n(0), n(1), n(2), n(3)]) => g.rect(n(0), n(1), n(2), n(3)),
        10 if finite(&[n(0), n(1), n(2), n(3)]) => g.quadratic_curve_to(n(0), n(1), n(2), n(3)),
        11 if finite(&[n(0), n(1), n(2), n(3), n(4), n(5)]) => g.bezier_curve_to(n(0), n(1), n(2), n(3), n(4), n(5)),
        12 if finite(&[n(0), n(1), n(2), n(3), n(4)]) => g.arc_to(n(0), n(1), n(2), n(3), n(4)),
        13 if finite(&[n(0), n(1), n(2), n(3), n(4), n(5), n(6)]) => {
            g.ellipse(n(0), n(1), n(2), n(3), n(4), n(5), n(6), a.get(7).and_then(|v| v.as_bool()).unwrap_or(false))
        }
        14 => g.draw_op(|g| g.fill()),
        15 => g.draw_op(|g| g.stroke()),
        16 => g.clip(),
        17 => {
            if g.save_depth() < MAX_SAVE_DEPTH {
                g.save();
            } else {
                return false.into_js(ctx); // JS 外壳据此只警告一次
            }
        }
        18 => g.restore(),
        19 if finite(&[n(0), n(1)]) => g.translate(n(0), n(1)),
        20 if n(0).is_finite() => g.rotate(n(0)),
        21 if finite(&[n(0), n(1)]) => g.scale(n(0), n(1)),
        22 | 23 => {
            let m = [n(0), n(1), n(2), n(3), n(4), n(5)];
            if finite(&m) {
                if op == 22 { g.set_transform_pre(m, (1.0, 1.0)) } else { g.transform_by(m) }
            }
        }
        24 => g.set_transform_pre([1.0, 0.0, 0.0, 1.0, 0.0, 0.0], (1.0, 1.0)),
        25 | 26 => {
            if let Some(t) = text(a.first()) {
                let (x, y) = (n(1), n(2));
                if finite(&[x, y]) {
                    g.draw_op(|g| if op == 25 { g.fill_text(&t, x, y) } else { g.stroke_text(&t, x, y) });
                }
            }
        }
        27 => {
            let t = text(a.first()).unwrap_or_default();
            return (crate::renderer::components::canvas_measure_text(g.font_size_px(), &t) as f64).into_js(ctx);
        }
        // 设颜色：解析不了就保持旧值并返回 false（HTML 语义），JS 外壳据此决定读回值
        30 | 31 => {
            let s = text(a.first()).unwrap_or_default();
            let ok = crate::renderer::components::parse_color_str(&s).is_some();
            if ok {
                if op == 30 { g.set_fill_style(&s) } else { g.set_stroke_style(&s) }
                // 换成纯色：清掉之前的渐变 / 图案
                g.run_json_command(&serde_json::json!({ "type": if op == 30 { "setFillPattern" } else { "setStrokePattern" } }));
            }
            return ok.into_js(ctx);
        }
        32 if n(0).is_finite() && n(0) > 0.0 => g.set_line_width(n(0)),
        33 if n(0).is_finite() && (0.0..=1.0).contains(&n(0)) => g.set_global_alpha(n(0)),
        34 => {
            if let Some(f) = text(a.first()) {
                g.set_font(&f)
            }
        }
        35 => g.set_text_align(&text(a.first()).unwrap_or_default()),
        36 => g.set_text_baseline(&text(a.first()).unwrap_or_default()),
        37 => g.set_line_cap(&text(a.first()).unwrap_or_default()),
        38 => g.set_line_join(&text(a.first()).unwrap_or_default()),
        39 if n(0).is_finite() && n(0) > 0.0 => g.set_miter_limit(n(0)),
        40 => {
            let segs: Vec<f32> = a
                .first()
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter::<f64>().filter_map(|x| x.ok()).map(|x| x as f32).collect())
                .unwrap_or_default();
            g.set_line_dash(&segs)
        }
        41 if n(0).is_finite() => g.set_line_dash_offset(n(0)),
        42 => {
            let color = text(a.get(1)).unwrap_or_else(|| "rgba(0,0,0,0)".into());
            g.run_json_command(&serde_json::json!({ "type": "setShadow2d", "blur": n(0), "color": color, "offsetX": n(2), "offsetY": n(3) }));
        }
        43 => g.run_json_command(&serde_json::json!({ "type": "setComposite", "op": text(a.first()).unwrap_or_default() })),
        44 => {
            // a[0] = JSON 描述（与解释器的 setFillGradient / setFillPattern 同格式），a[1] = 是否描边
            if let Some(desc) = text(a.first()).and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()) {
                let stroke = a.get(1).and_then(|v| v.as_bool()).unwrap_or(false);
                let mut d = desc;
                let kind = if d.get("src").is_some() { "Pattern" } else { "Gradient" };
                d["type"] = serde_json::json!(format!("set{}{}", if stroke { "Stroke" } else { "Fill" }, kind));
                g.run_json_command(&d);
            }
        }
        45 => {
            let (x, y, w, h) = (n(0), n(1), n(2), n(3));
            if !finite(&[x, y, w, h]) || w < 1.0 || h < 1.0 {
                return Err(Exception::throw_range(ctx, "getImageData: 宽高必须为正数（IndexSizeError）"));
            }
            let (w, h) = (w as u32, h as u32);
            if (w as u64) * (h as u64) * 4 > crate::js::binary::MAX_BRIDGE_BYTES as u64 {
                return Err(Exception::throw_range(ctx, "getImageData: 区域过大"));
            }
            return array_buffer(ctx, g.read_rgba(x, y, w, h, (1.0, 1.0)));
        }
        46 => {
            let (w, h, dx, dy) = (n(1) as u32, n(2) as u32, n(3), n(4));
            if finite(&[dx, dy]) {
                require_bytes(ctx, a.first(), "imageData.data", |b| {
                    if b.len() as u64 >= (w as u64) * (h as u64) * 4 {
                        g.put_image_data(b, w, h, dx, dy, (1.0, 1.0));
                    }
                })?;
            }
        }
        _ => {}
    }
    Ok(undef)
}
