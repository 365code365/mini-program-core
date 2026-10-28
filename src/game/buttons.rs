//! 原生按钮（game-doc/03、04 G7.3，GT-U3）：UserInfoButton / OpenSettingButton / FeedbackButton / GameClubButton（与
//! GameRecorderShareButton 同一个外壳）。引擎画（与覆盖层同一次合成，层级：游戏 < 按钮 < Modal 等系统覆盖层），引擎命中，
//! 点了之后的动作在 JS 里交给宿主（game-buttons.js）。
//!
//! 样式字段按官方：left / top / width / height（逻辑像素）、backgroundColor、borderColor、borderWidth、borderRadius、
//! color、textAlign（left / center / right）、fontSize、lineHeight。type: 'text' 画底色 + 边框 + 一行文字，'image' 把图拉伸铺满
//! （圆角裁）。GameClubButton 的 `icon`（green / white / dark / light）微信用的是自己的游戏圈图标，这里没有那张图，
//! 画一个对应颜色的圆点（记在验收文档）。没设的颜色是透明、fontSize 16、textAlign left、lineHeight = 高度（垂直居中）。
//! 按下时整颗按钮 0.7 透明度（按压态；微信的按压反馈待取证）。

use super::GameEngine;
use crate::renderer::components::parse_color_str;
use crate::{Canvas, Color, Paint};
use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub struct Button {
    pub id: u32,
    kind: String,
    image_type: bool,
    text: String,
    image: Option<u32>,
    icon: String,
    style: Value,
    pub visible: bool,
}

#[derive(Default)]
pub struct Buttons {
    list: Vec<Button>,
    /// 按住的按钮与那根手指
    pressed: Option<(u32, i32)>,
    pub dirty: bool,
}

fn num(s: &Value, k: &str, d: f32) -> f32 {
    s.get(k).and_then(Value::as_f64).map(|v| v as f32).unwrap_or(d)
}

fn color(s: &Value, k: &str) -> Option<Color> {
    s.get(k).and_then(Value::as_str).and_then(parse_color_str).filter(|c| c.a > 0)
}

impl Button {
    fn rect(&self) -> (f32, f32, f32, f32) {
        let s = &self.style;
        (num(s, "left", 0.0), num(s, "top", 0.0), num(s, "width", 0.0), num(s, "height", 0.0))
    }
    fn contains(&self, x: f32, y: f32) -> bool {
        let (l, t, w, h) = self.rect();
        self.visible && x >= l && x < l + w && y >= t && y < t + h
    }
}

impl Buttons {
    pub fn any_visible(&self) -> bool {
        self.list.iter().any(|b| b.visible)
    }

    /// `__game_button(op, json)`：create / update（整份状态）/ destroy
    pub fn call(&mut self, op: &str, a: &Value) {
        let id = a.get("id").and_then(Value::as_u64).unwrap_or(0) as u32;
        match op {
            "destroy" => self.list.retain(|b| b.id != id),
            _ => {
                let b = Button {
                    id,
                    kind: a.get("kind").and_then(Value::as_str).unwrap_or("").into(),
                    image_type: a.get("type").and_then(Value::as_str) == Some("image"),
                    text: a.get("text").and_then(Value::as_str).unwrap_or("").into(),
                    image: a.get("image").and_then(Value::as_u64).map(|v| v as u32),
                    icon: a.get("icon").and_then(Value::as_str).unwrap_or("").into(),
                    style: a.get("style").cloned().unwrap_or(Value::Null),
                    visible: a.get("visible").and_then(Value::as_bool).unwrap_or(true),
                };
                match self.list.iter_mut().find(|x| x.id == id) {
                    Some(x) => *x = b,
                    None => self.list.push(b),
                }
            }
        }
        if self.pressed.is_some_and(|(p, _)| !self.list.iter().any(|b| b.id == p && b.visible)) {
            self.pressed = None;
        }
        self.dirty = true;
    }

    /// 最上面（后建的在上）那个包含该点的可见按钮
    fn at(&self, x: f32, y: f32) -> Option<u32> {
        self.list.iter().rev().find(|b| b.contains(x, y)).map(|b| b.id)
    }
}

/// 把一颗按钮画进 `c`（合成画布，`sf` = 合成像素 / 逻辑像素）
fn draw(b: &Button, c: &mut Canvas, sf: f32, img: Option<&super::image::loader::Decoded>, pressed: bool, tr: Option<&crate::text::TextRenderer>) {
    let s = &b.style;
    let (l, t, w, h) = b.rect();
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let (x, y, w, h) = (l * sf, t * sf, w * sf, h * sf);
    let r = (num(s, "borderRadius", 0.0) * sf).min(w / 2.0).min(h / 2.0);
    let bw = num(s, "borderWidth", 0.0) * sf;
    // 按压态：先画在一块小画布上，再按 0.7 透明度贴回
    let (cw, ch) = (w.ceil() as u32 + 1, h.ceil() as u32 + 1);
    let mut layer = Canvas::new(cw, ch);
    layer.clear(Color::TRANSPARENT);
    let lc = &mut layer;
    if let (Some(bc), true) = (color(s, "borderColor"), bw > 0.0) {
        lc.fill_round_rect(0.0, 0.0, w, h, r, bc);
    }
    let inner = (bw, bw, (w - 2.0 * bw).max(0.0), (h - 2.0 * bw).max(0.0), (r - bw).max(0.0));
    if b.image_type {
        if let Some(d) = img {
            lc.draw_image(&d.rgba, d.w, d.h, inner.0, inner.1, inner.2, inner.3, "scaleToFill", inner.4);
        }
    } else if let Some(bg) = color(s, "backgroundColor") {
        lc.fill_round_rect(inner.0, inner.1, inner.2, inner.3, inner.4, bg);
    }
    if b.kind == "gameClub" && !b.image_type {
        let c = match b.icon.as_str() { "white" => Color::new(255, 255, 255, 255), "dark" => Color::new(51, 51, 51, 255), "light" => Color::new(200, 200, 200, 255), _ => Color::new(7, 193, 96, 255) };
        let d = w.min(h) * 0.8;
        lc.fill_round_rect((w - d) / 2.0, (h - d) / 2.0, d, d, d / 2.0, c);
    }
    if !b.image_type && !b.text.is_empty() {
        if let Some(tr) = tr {
            let size = num(s, "fontSize", 16.0) * sf;
            let lh = num(s, "lineHeight", h / sf) * sf;
            let tw = tr.measure_text(&b.text, size);
            let tx = match s.get("textAlign").and_then(Value::as_str) {
                Some("center") => (w - tw) / 2.0,
                Some("right") => w - bw - tw,
                _ => bw,
            };
            // 行盒垂直居中：基线 ≈ 行盒中线 + 0.35 字号
            let baseline = lh / 2.0 + size * 0.35;
            let fg = color(s, "color").unwrap_or(Color::new(0, 0, 0, 255));
            tr.draw_text(lc, &b.text, tx, baseline, size, &Paint::new().with_color(fg));
        }
    }
    c.blend_pixels_opacity(x.round() as i32, y.round() as i32, layer.pixels(), cw as usize, if pressed { 0.7 } else { 1.0 });
}

impl GameEngine {
    /// 合成时：可见按钮画进 `c`
    pub(super) fn draw_buttons(&self, c: &mut Canvas, sf: f32) {
        let bs = self.buttons.borrow();
        let text = crate::text::shared_fonts();
        for b in bs.list.iter().filter(|b| b.visible) {
            let img = b.image.and_then(|id| self.images.borrow_mut().get(id));
            draw(b, c, sf, img.as_deref(), bs.pressed.is_some_and(|(p, _)| p == b.id), text.as_deref());
        }
    }

    pub(super) fn buttons_visible(&self) -> bool {
        self.buttons.borrow().any_visible()
    }

    /// 按钮的命中（在系统覆盖层之后、游戏之前，02 §7.4）：按下落在按钮上这串归按钮，抬起还在同一颗上就派发 tap
    pub(super) fn button_touch(&mut self, phase: i32, id: i32, x: f32, y: f32) -> bool {
        let mut bs = self.buttons.borrow_mut();
        if phase == 0 {
            if bs.pressed.is_some() {
                return false;
            }
            let Some(b) = bs.at(x, y) else { return false };
            bs.pressed = Some((b, id));
            bs.dirty = true;
            return true;
        }
        let Some((b, finger)) = bs.pressed.filter(|p| p.1 == id) else { return false };
        if phase == 1 {
            return true;
        }
        bs.pressed = None;
        bs.dirty = true;
        let hit = phase == 2 && bs.at(x, y) == Some(b);
        drop(bs);
        let _ = finger;
        if hit {
            self.guards.watchdog.arm();
            let r = self.app.eval(&format!("typeof __gameButtonTap === 'function' && __gameButtonTap({b})"));
            self.guards.watchdog.disarm();
            if let Err(e) = r {
                self.report_error(super::error_kind(&e, "button"), &e);
            }
            self.after_js();
        }
        true
    }
}

/// JS 入口：`__game_button(op, json)`
pub fn register(rt: &crate::js::JsRuntime, b: std::rc::Rc<std::cell::RefCell<Buttons>>) -> Result<(), String> {
    rt.register_raw("__game_button", move |ctx, a| {
        let s = |i: usize| a.0.get(i).and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
        let v: Value = serde_json::from_str(&s(1)).unwrap_or(Value::Null);
        b.borrow_mut().call(&s(0), &v);
        Ok(rquickjs::Value::new_undefined(ctx.clone()))
    })
}
