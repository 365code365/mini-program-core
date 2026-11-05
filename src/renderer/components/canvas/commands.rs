//! 画布指令的解释执行：一条 JSON 指令 → `Canvas2DContext` 上的一次调用。
//!
//! `canvas` 模块的一片。从 `manager.rs` 的 `execute_commands` 循环体**原样搬出来**（纯搬迁），
//! 好让旧接口（`wx.createCanvasContext` 的 `draw()`）与 `type="2d"` 的追加式日志共用一份。
use super::*;

/// 执行一条指令（不认识的类型忽略）。有裁剪区时绘制类指令画完按蒙版混合（clipdash.rs）
pub(super) fn run_command(ctx: &mut Canvas2DContext, cmd: &serde_json::Value) {
    let cmd_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let n = |k: &str| cmd.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    match cmd_type {
        "clip" => return ctx.clip_with_rule(even_odd(cmd)),
        "setLineDash" => {
            let segs: Vec<f32> = cmd
                .get("segments")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().map(|x| x.as_f64().unwrap_or(-1.0) as f32).collect())
                .unwrap_or_default();
            ctx.set_line_dash(&segs);
            if cmd.get("offset").is_some() {
                ctx.set_line_dash_offset(n("offset"));
            }
            return;
        }
        "setLineDashOffset" => return ctx.set_line_dash_offset(n("offset")),
        "setMiterLimit" => return ctx.set_miter_limit(n("limit")),
        _ => {}
    }
    if ctx.has_clip() && super::clipdash::DRAW_OPS.contains(&cmd_type) {
        let before = ctx.clip_snapshot();
        run_inner(ctx, cmd, cmd_type);
        if let Some(b) = before {
            ctx.clip_apply(b);
        }
    } else {
        run_inner(ctx, cmd, cmd_type);
    }
}

fn run_inner(ctx: &mut Canvas2DContext, cmd: &serde_json::Value, cmd_type: &str) {
    match cmd_type {
        "setFillStyle" => {
            if let Some(color) = cmd.get("color").and_then(|v| v.as_str()) {
                ctx.set_fill_style(color);
            }
        }
        "setStrokeStyle" => {
            if let Some(color) = cmd.get("color").and_then(|v| v.as_str()) {
                ctx.set_stroke_style(color);
            }
        }
        "setLineWidth" => {
            if let Some(width) = cmd.get("width").and_then(|v| v.as_f64()) {
                ctx.set_line_width(width as f32);
            }
        }
        "fillRect" => {
            let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            ctx.fill_rect(x, y, w, h);
        }
        "strokeRect" => {
            let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            ctx.stroke_rect(x, y, w, h);
        }
        "clearRect" => {
            let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            ctx.clear_rect(x, y, w, h);
        }
        "beginPath" => ctx.begin_path(),
        "closePath" => ctx.close_path(),
        "moveTo" => {
            let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            ctx.move_to(x, y);
        }
        "lineTo" => {
            let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            ctx.line_to(x, y);
        }
        "arc" => {
            let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let r = cmd.get("r").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let s = cmd.get("sAngle").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let e = cmd.get("eAngle").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let cc = cmd.get("counterclockwise").and_then(|v| v.as_bool()).unwrap_or(false);
            ctx.arc(x, y, r, s, e, cc);
        }
        "fill" => ctx.fill_with_rule(even_odd(cmd)),
        "stroke" => ctx.stroke(),
        "roundRect" => {
            let n = |k: &str| cmd.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let r = cmd.get("radii").and_then(|v| v.as_array());
            let ri = |i: usize| r.and_then(|a| a.get(i)).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            ctx.round_rect(n("x"), n("y"), n("width"), n("height"), [ri(0), ri(1), ri(2), ri(3)]);
        }
        "save" => ctx.save(),
        "restore" => ctx.restore(),
        "translate" => {
            let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            ctx.translate(x, y);
        }
        "rotate" => {
            let a = cmd.get("angle").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            ctx.rotate(a);
        }
        "scale" => {
            let sx = cmd.get("scaleX").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
            let sy = cmd.get("scaleY").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
            ctx.scale(sx, sy);
        }
        "setFontSize" => {
            if let Some(s) = cmd.get("size").and_then(|v| v.as_f64()) { ctx.font_size = s as f32; }
        }
        "setTextAlign" => {
            if let Some(a) = cmd.get("align").and_then(|v| v.as_str()) { ctx.set_text_align(a); }
        }
        "setTextBaseline" => {
            if let Some(b) = cmd.get("baseline").and_then(|v| v.as_str()) { ctx.set_text_baseline(b); }
        }
        "setGlobalAlpha" => {
            if let Some(a) = cmd.get("alpha").and_then(|v| v.as_f64()) { ctx.set_global_alpha(a as f32); }
        }
        "setLineCap" => {
            if let Some(c) = cmd.get("cap").and_then(|v| v.as_str()) { ctx.set_line_cap(c); }
        }
        "setLineJoin" => {
            if let Some(j) = cmd.get("join").and_then(|v| v.as_str()) { ctx.set_line_join(j); }
        }
        "rect" => {
            let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            ctx.rect(x, y, w, h);
        }
        "quadraticCurveTo" => {
            let cpx = cmd.get("cpx").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let cpy = cmd.get("cpy").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            ctx.quadratic_curve_to(cpx, cpy, x, y);
        }
        "bezierCurveTo" => {
            let a = |k: &str| cmd.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            ctx.bezier_curve_to(a("cp1x"), a("cp1y"), a("cp2x"), a("cp2y"), a("x"), a("y"));
        }
        "fillText" | "strokeText" => {
            let text = cmd.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            // maxWidth 没给 / 是 null：不压缩（JSON 里 undefined 的字段本来就没有）
            let mw = cmd.get("maxWidth").and_then(|v| v.as_f64()).map(|v| v as f32);
            if cmd_type == "fillText" { ctx.fill_text_max(text, x, y, mw) } else { ctx.stroke_text_max(text, x, y, mw) }
        }
        "drawImage" => super::image_src::draw_image_cmd(ctx, cmd),
        // ── 2D 新接口（`type="2d"`）的那几条，见 fx2d.rs / extra2d.rs ──
        "setFillGradient" | "setStrokeGradient" => {
            if let Some(src) = gradient_source(cmd) {
                if cmd_type == "setFillGradient" { ctx.fx.fill = Some(src) } else { ctx.fx.stroke = Some(src) }
            }
        }
        "setFillPattern" | "setStrokePattern" => {
            let src = pattern_source(cmd);
            if cmd_type == "setFillPattern" { ctx.fx.fill = src } else { ctx.fx.stroke = src }
        }
        // 旧接口 setShadow(offsetX, offsetY, blur, color) 与 2d 的四个 shadow 属性同一条
        "setShadow" | "setShadow2d" => {
            let color = cmd.get("color").and_then(|v| v.as_str()).and_then(parse_color_str).unwrap_or(Color::TRANSPARENT);
            let n = |k: &str| cmd.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            ctx.fx.shadow = fx2d::Shadow { blur: n("blur").max(0.0), color, ox: n("offsetX"), oy: n("offsetY") };
        }
        "setComposite" => {
            ctx.fx.composite = fx2d::Composite::parse(cmd.get("op").and_then(|v| v.as_str()).unwrap_or(""));
        }
        "setTransform" | "transform" => {
            let n = |k: &str, d: f64| cmd.get(k).and_then(|v| v.as_f64()).unwrap_or(d) as f32;
            let m = [n("a", 1.0), n("b", 0.0), n("c", 0.0), n("d", 1.0), n("e", 0.0), n("f", 0.0)];
            if cmd_type == "transform" { ctx.transform_by(m) } else { ctx.set_transform_pre(m, pre(cmd)) }
        }
        "ellipse" => {
            let n = |k: &str| cmd.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let ccw = cmd.get("counterclockwise").and_then(|v| v.as_bool()).unwrap_or(false);
            ctx.ellipse(n("x"), n("y"), n("rx"), n("ry"), n("rotation"), n("sAngle"), n("eAngle"), ccw);
        }
        "arcTo" => {
            let n = |k: &str| cmd.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            ctx.arc_to(n("x1"), n("y1"), n("x2"), n("y2"), n("r"));
        }
        "putImageData" => {
            let n = |k: &str| cmd.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            if let Some(bytes) = cmd.get("b64").and_then(|v| v.as_str()).and_then(crate::wxfs::codec::unb64) {
                ctx.put_image_data(&bytes, n("w") as u32, n("h") as u32, n("dx"), n("dy"), pre(cmd));
            }
        }
        _ => {}
    }
}

/// `fill('evenodd')` / `clip('evenodd')`：指令里带 `rule`，没带就是 nonzero
fn even_odd(cmd: &serde_json::Value) -> bool {
    cmd.get("rule").and_then(|v| v.as_str()) == Some("evenodd")
}

/// `pre`：JS 侧算好的「CSS 尺寸 / canvas 尺寸」（见 extra2d.rs 的坐标约定），没给按 1
pub(super) fn pre(cmd: &serde_json::Value) -> (f32, f32) {
    let a = cmd.get("pre").and_then(|v| v.as_array());
    let g = |i: usize| a.and_then(|a| a.get(i)).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    (g(0), g(1))
}

/// `{kind: linear|radial|conic, args: [...], stops: [[offset, color], ...]}`（用户坐标）
fn gradient_source(cmd: &serde_json::Value) -> Option<fx2d::Source> {
    let args: Vec<f32> = cmd.get("args")?.as_array()?.iter().map(|v| v.as_f64().unwrap_or(0.0) as f32).collect();
    let mut stops: Vec<(f32, Color)> = cmd
        .get("stops")?
        .as_array()?
        .iter()
        .filter_map(|s| {
            let s = s.as_array()?;
            Some((s.first()?.as_f64()? as f32, parse_color_str(s.get(1)?.as_str()?)?))
        })
        .collect();
    stops.sort_by(|a, b| a.0.total_cmp(&b.0));
    let a = |i: usize| args.get(i).copied().unwrap_or(0.0);
    Some(match cmd.get("kind").and_then(|v| v.as_str()).unwrap_or("linear") {
        "radial" => fx2d::Source::Radial { c0: (a(0), a(1)), r0: a(2), c1: (a(3), a(4)), r1: a(5), stops },
        "conic" => fx2d::Source::Conic { angle: a(0), c: (a(1), a(2)), stops },
        _ => fx2d::Source::Linear { p0: (a(0), a(1)), p1: (a(2), a(3)), stops },
    })
}

/// `{src, repetition}`：图片按包内路径 / `wxfile://` 解析（与 `<image>` 同一套）
fn pattern_source(cmd: &serde_json::Value) -> Option<fx2d::Source> {
    let src = cmd.get("src")?.as_str()?;
    // 与 drawImage 同一份图源（解码一次进缓存；SVG / data: URI 也认，见 image_src.rs）
    let img = super::image_src::source(src)?;
    let rep = cmd.get("repetition").and_then(|v| v.as_str()).unwrap_or("repeat");
    Some(fx2d::Source::Pattern {
        rgba: img.rgba.clone(),
        w: img.w,
        h: img.h,
        repeat_x: rep == "repeat" || rep == "repeat-x" || rep.is_empty(),
        repeat_y: rep == "repeat" || rep == "repeat-y" || rep.is_empty(),
    })
}
