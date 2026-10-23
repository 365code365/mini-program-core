//! `wx.createAnimation()` 载荷（`animation="{{data}}"`）的逐步插值：从 animation.rs 拆出（那边已过 500 行）。
//! 合成（`resolve_animated_node`）仍在 animation.rs。

use super::*;

impl WxmlRenderer {
    /// 求 `animation="{{animData}}"`（`wx.createAnimation().export()`）此刻的样式覆盖值。
    ///
    /// 载荷形如 `{actions:[{animates:[{type,args}],option:{transition:{duration,delay,timingFunction}}}]}`，
    /// 每个 action 是一「步」：按 delay/duration 顺序播放，步与步之间对目标值做插值。
    /// 起始时刻按载荷指纹记忆 —— 同一份 export 在后续帧里继续播，换了新的 export 就重新开始。
    pub(super) fn js_animated_values(&mut self, node: &RenderNode) -> Option<JsAnimValues> {
        let raw = node.attrs.get("animation")?;
        if !raw.contains("actions") {
            return None;
        }
        // 属性里的对象被序列化成单引号 JSON（见 expr::render_value），这里还原
        let parsed: JsonValue = serde_json::from_str(&raw.replace('\'', "\"")).ok()?;
        let actions = parsed.get("actions")?.as_array()?;
        if actions.is_empty() {
            return None;
        }

        let fingerprint = {
            // FNV-1a：只用来判断「还是不是同一份 export」
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in raw.as_bytes() {
                h ^= *b as u64;
                h = h.wrapping_mul(0x1000_0000_01b3);
            }
            h
        };
        let now = self.animation_time();
        let start = match self.js_animations.get(&fingerprint) {
            Some(s) => *s,
            None => {
                if self.js_animations.len() > 64 {
                    self.js_animations.clear();
                }
                self.js_animations.insert(fingerprint, now);
                now
            }
        };
        let elapsed = now - start;

        let sf = self.scale_factor;
        let mut state = JsAnimValues {
            transform: crate::renderer::components::Transform::new(),
            has_transform: false,
            opacity: None,
            background_color: None,
        };
        let mut cursor = 0.0f32;
        let mut finished = true;
        for action in actions {
            let tr = action.get("option").and_then(|o| o.get("transition"));
            let ms = |k: &str, d: f32| {
                tr.and_then(|t| t.get(k)).and_then(|v| v.as_f64()).unwrap_or(d as f64) as f32 / 1000.0
            };
            let duration = ms("duration", 400.0).max(0.0);
            let delay = ms("delay", 0.0).max(0.0);
            let curve = tr
                .and_then(|t| t.get("timingFunction"))
                .and_then(|v| v.as_str())
                .map(crate::renderer::components::parse_timing_function)
                .unwrap_or((0.0, 0.0, 1.0, 1.0)); // createAnimation 默认 linear

            let target = Self::apply_js_animates(&state, action.get("animates"), sf);
            let step_start = cursor + delay;
            let step_end = step_start + duration;
            if elapsed >= step_end || duration <= 0.0 {
                state = target;
                cursor = step_end;
                continue;
            }
            finished = false;
            let p = if elapsed <= step_start {
                0.0
            } else {
                (elapsed - step_start) / duration
            };
            let eased = crate::renderer::anim::cubic_bezier(p, curve.0, curve.1, curve.2, curve.3);
            state = JsAnimValues {
                transform: crate::renderer::anim::lerp_transform(state.transform, target.transform, eased),
                has_transform: state.has_transform || target.has_transform,
                opacity: match (state.opacity, target.opacity) {
                    (Some(a), Some(b)) => Some(a + (b - a) * eased),
                    (None, Some(b)) => Some(1.0 + (b - 1.0) * eased),
                    (a, None) => a,
                },
                background_color: match (state.background_color, target.background_color) {
                    (Some(a), Some(b)) => Some(crate::renderer::anim::lerp_color(a, b, eased)),
                    (None, Some(b)) => Some(b),
                    (a, None) => a,
                },
            };
            break;
        }
        if !finished {
            self.mark_animating();
        }
        Some(state)
    }

    /// 把一步里的 animates 累积到状态上（每步给的是绝对目标值，未提及的属性沿用上一步）
    pub(super) fn apply_js_animates(
        base: &JsAnimValues,
        animates: Option<&JsonValue>,
        sf: f32,
    ) -> JsAnimValues {
        let mut out = base.clone();
        let Some(list) = animates.and_then(|a| a.as_array()) else { return out };
        for item in list {
            let ty = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let args = item.get("args").and_then(|v| v.as_array());
            let num = |i: usize, d: f32| {
                args.and_then(|a| a.get(i))
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(d)
            };
            match ty {
                // translate 的参数是逻辑 px，Transform 用物理 px
                "translate" => {
                    out.transform.translate_x = num(0, 0.0) * sf;
                    out.transform.translate_y = num(1, 0.0) * sf;
                    out.has_transform = true;
                }
                "translateX" => { out.transform.translate_x = num(0, 0.0) * sf; out.has_transform = true; }
                "translateY" => { out.transform.translate_y = num(0, 0.0) * sf; out.has_transform = true; }
                "rotate" => { out.transform.rotate = num(0, 0.0); out.has_transform = true; }
                "scale" => {
                    let sx = num(0, 1.0);
                    out.transform.scale_x = sx;
                    out.transform.scale_y = num(1, sx);
                    out.has_transform = true;
                }
                "scaleX" => { out.transform.scale_x = num(0, 1.0); out.has_transform = true; }
                "scaleY" => { out.transform.scale_y = num(0, 1.0); out.has_transform = true; }
                "skew" => {
                    out.transform.skew_x = num(0, 0.0);
                    out.transform.skew_y = num(1, 0.0);
                    out.has_transform = true;
                }
                "skewX" => { out.transform.skew_x = num(0, 0.0); out.has_transform = true; }
                "skewY" => { out.transform.skew_y = num(0, 0.0); out.has_transform = true; }
                // matrix(a, b, c, d, tx, ty)：分解成 rotate ∘ scale ∘ skewX（compose.rs 的合成顺序），平移是逻辑 px
                "matrix" => {
                    let (a, b, c, d) = (num(0, 1.0), num(1, 0.0), num(2, 0.0), num(3, 1.0));
                    let sx = (a * a + b * b).sqrt();
                    if sx > 1e-6 {
                        let th = b.atan2(a);
                        let (sin, cos) = th.sin_cos();
                        out.transform.rotate = th.to_degrees();
                        out.transform.scale_x = sx;
                        out.transform.scale_y = cos * d - sin * c;
                        out.transform.skew_x = ((cos * c + sin * d) / sx).atan().to_degrees();
                        out.transform.skew_y = 0.0;
                    }
                    out.transform.translate_x = num(4, 0.0) * sf;
                    out.transform.translate_y = num(5, 0.0) * sf;
                    out.has_transform = true;
                }
                // 3D 旋转：2D 仿射里按正交投影降级 —— 绕 X 轴转 θ 就是纵向压到 cos θ，绕 Y 轴是横向（写明的取舍）
                "rotateX" => { out.transform.scale_y = num(0, 0.0).to_radians().cos(); out.has_transform = true; }
                "rotateY" => { out.transform.scale_x = num(0, 0.0).to_radians().cos(); out.has_transform = true; }
                "rotate3d" => {
                    let (x, y, z, deg) = (num(0, 0.0), num(1, 0.0), num(2, 0.0), num(3, 0.0));
                    let len = (x * x + y * y + z * z).sqrt();
                    if len > 1e-6 {
                        let r = deg.to_radians();
                        out.transform.rotate = deg * z / len;
                        out.transform.scale_y = (r * x.abs() / len).cos();
                        out.transform.scale_x = (r * y.abs() / len).cos();
                    }
                    out.has_transform = true;
                }
                "opacity" => out.opacity = Some(num(0, 1.0).clamp(0.0, 1.0)),
                "backgroundColor" => {
                    if let Some(c) = args
                        .and_then(|a| a.first())
                        .and_then(|v| v.as_str())
                        .and_then(crate::renderer::components::parse_color_str)
                    {
                        out.background_color = Some(c);
                    }
                }
                // width/height 会引发重排，与 CSS 动画同样的取舍：不支持
                _ => {}
            }
        }
        out
    }
}
