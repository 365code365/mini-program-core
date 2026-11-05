//! `isPointInPath` / `isPointInStroke`（HTML 标准，Canvas 2D）。
//!
//! 点 (x, y) 是**画布坐标**（canvas.width × canvas.height 那一套，不受当前矩阵影响）；路径是当前路径
//! （或传进来的 Path2D）按建路径时的矩阵变换过的设备坐标 —— 与 `fill()` / `stroke()` 画出来的位置一致。
//! 做法：路径展平成折线（与虚线同一个容差 0.2 设备像素），
//! - in path：每条子路径按闭合算，nonzero（默认）或 evenodd 的环绕数；
//! - in stroke：点到每段折线的距离 ≤ 线宽 / 2（线帽按 round 近似、斜接不外伸 —— 与光栅化的描边在拐角外侧差不到一个线宽）。
//!
//! JS 侧是同步调用（`__native_canvas_hit`，js/bridge_io.rs）：先把这块画布排着的指令执行掉，再问。
use super::*;

impl Canvas2DContext {
    /// 画布坐标 → 设备像素：`pre` 是 JS 侧的「CSS 尺寸 / canvas 尺寸」（旧接口为 1），再乘 dpr
    fn canvas_to_device(&self, x: f32, y: f32, pre: (f32, f32)) -> (f32, f32) {
        (x * pre.0 * self.dpr, y * pre.1 * self.dpr)
    }

    /// 点在当前路径里（`even_odd` = `'evenodd'`）
    pub fn is_point_in_path(&self, x: f32, y: f32, pre: (f32, f32), even_odd: bool) -> bool {
        if !(x.is_finite() && y.is_finite()) {
            return false;
        }
        let (px, py) = self.canvas_to_device(x, y, pre);
        let mut winding = 0i32;
        for c in self.build_path().flatten(0.2) {
            if c.len() < 2 {
                continue;
            }
            for i in 0..c.len() {
                let (a, b) = (c[i], c[(i + 1) % c.len()]);
                // 半开区间：一个顶点只算一次
                if (a.y <= py) != (b.y <= py) {
                    let t = (py - a.y) / (b.y - a.y);
                    if a.x + t * (b.x - a.x) > px {
                        winding += if b.y > a.y { 1 } else { -1 };
                    }
                }
            }
        }
        if even_odd { winding % 2 != 0 } else { winding != 0 }
    }

    /// 点落在当前路径按当前线宽描出来的线上
    pub fn is_point_in_stroke(&self, x: f32, y: f32, pre: (f32, f32)) -> bool {
        if !(x.is_finite() && y.is_finite()) {
            return false;
        }
        let (px, py) = self.canvas_to_device(x, y, pre);
        let half = (self.line_width * self.avg_scale() / 2.0).max(0.0);
        for c in self.build_path().flatten(0.2) {
            if c.len() == 1 {
                continue;
            }
            for s in c.windows(2) {
                let (a, b) = (s[0], s[1]);
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let len2 = dx * dx + dy * dy;
                let t = if len2 > 0.0 { (((px - a.x) * dx + (py - a.y) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
                let (qx, qy) = (a.x + t * dx - px, a.y + t * dy - py);
                if qx * qx + qy * qy <= half * half {
                    return true;
                }
            }
        }
        false
    }
}

/// `__native_canvas_hit` 的一次问询：`{x, y, stroke, rule, path: [指令]|null, pre}`。
/// 给了 path（Path2D 录下的指令）就在上下文的一份拷贝上换成那条路径再问，不动画布的当前路径
pub fn hit_test_json(ctx: &Canvas2DContext, o: &serde_json::Value) -> bool {
    let n = |k: &str| o.get(k).and_then(|v| v.as_f64()).unwrap_or(f64::NAN) as f32;
    let pre = super::commands::pre(o);
    let even_odd = o.get("rule").and_then(|v| v.as_str()) == Some("evenodd");
    let stroke = o.get("stroke").and_then(|v| v.as_bool()).unwrap_or(false);
    let ask = |c: &Canvas2DContext| if stroke { c.is_point_in_stroke(n("x"), n("y"), pre) } else { c.is_point_in_path(n("x"), n("y"), pre, even_odd) };
    match o.get("path").and_then(|v| v.as_array()) {
        Some(cmds) => {
            let mut t = ctx.clone();
            t.begin_path();
            for cmd in cmds {
                // Path2D 里只有建路径的指令（moveTo / arc / rect …），不会画到画布上
                super::commands::run_command(&mut t, cmd);
            }
            ask(&t)
        }
        None => ask(ctx),
    }
}

/// 进程级画布表里问一次（键 `2d#<id>` 或旧接口的 canvas-id）
pub fn canvas_hit_test(key: &str, json: &str) -> bool {
    let o: serde_json::Value = serde_json::from_str(json).unwrap_or(serde_json::Value::Null);
    super::CANVAS_MANAGER.lock().ok().and_then(|m| m.get_existing_context(key).map(|c| hit_test_json(c, &o))).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 点在路径里_nonzero_与_evenodd() {
        // dpr 2：画布坐标 (x, y) 对应设备 (2x, 2y)
        let mut ctx = Canvas2DContext::new_with_dpr("hit", 100, 100, 2.0);
        ctx.begin_path();
        ctx.rect(10.0, 10.0, 60.0, 60.0);
        ctx.rect(20.0, 20.0, 20.0, 20.0); // 同向的第二个矩形：nonzero 里仍算里面，evenodd 里是洞
        assert!(ctx.is_point_in_path(15.0, 15.0, (1.0, 1.0), false));
        assert!(ctx.is_point_in_path(30.0, 30.0, (1.0, 1.0), false), "nonzero：环绕数 2 也在里面");
        assert!(!ctx.is_point_in_path(30.0, 30.0, (1.0, 1.0), true), "evenodd：环绕数 2 是洞");
        assert!(!ctx.is_point_in_path(80.0, 15.0, (1.0, 1.0), false));
        // 当前矩阵不影响点：translate 之后建的路径挪走了，原处不再命中
        ctx.begin_path();
        ctx.translate(50.0, 0.0);
        ctx.rect(0.0, 0.0, 10.0, 10.0);
        assert!(ctx.is_point_in_path(55.0, 5.0, (1.0, 1.0), false));
        assert!(!ctx.is_point_in_path(5.0, 5.0, (1.0, 1.0), false));
    }

    #[test]
    fn 点在描边上_按线宽() {
        let mut ctx = Canvas2DContext::new_with_dpr("hit", 100, 100, 1.0);
        ctx.set_line_width(10.0);
        ctx.begin_path();
        ctx.move_to(10.0, 50.0);
        ctx.line_to(90.0, 50.0);
        assert!(ctx.is_point_in_stroke(50.0, 54.0, (1.0, 1.0)));
        assert!(!ctx.is_point_in_stroke(50.0, 56.0, (1.0, 1.0)));
        assert!(!ctx.is_point_in_path(50.0, 50.0, (1.0, 1.0), false), "一条线围不出面积");
    }
}
