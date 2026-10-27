//! 小程序实例接受「每根指针一次」的触摸（`mr_app_touch`，game-doc/02 §7.1、04 G3.2）。
//!
//! 宿主可以统一只调这一个接口（小游戏的 `mr_game_touch` 同形），引擎在这里映射回小程序现有的单指状态机：
//! 第一根手指 → `pointer_down / pointer_move_at / pointer_up / pointer_cancel`，第二根 → `pointer2`（只有图片预览认它），
//! 第三根起丢掉。规则与 Android `MiniTouch.kt` 逐条相同（那份是 Kotlin 侧的同一张表）：
//! - 第一根先抬、第二根还按着：报第二根抬起，之后把它当第一根接着送 —— **不报第一根抬起**，否则页面会把它当一次点击；
//! - 系统取消：第二根先取消，再取消第一根。
//!
//! 老接口照旧保留；两条路的结果逐字节相同由 `engine_input_tests::mr_app_touch_*` 守着（GT-T9）。

use super::MiniEngine;

pub const PHASE_DOWN: i32 = 0;
pub const PHASE_MOVE: i32 = 1;
pub const PHASE_UP: i32 = 2;
pub const PHASE_CANCEL: i32 = 3;

#[derive(Default, Debug)]
pub struct TouchAdapter {
    primary: Option<i32>,
    /// 第二根手指的 id 与最后位置（第一根先抬时用它的位置报抬起）
    secondary: Option<(i32, f32, f32)>,
}

impl TouchAdapter {
    /// `t_ms`：事件时刻（宿主单调钟毫秒，与 `mr_app_pump` 同一个钟）；≤ 0 = 不知道，移动按到达处理
    pub fn apply(&mut self, e: &mut MiniEngine, phase: i32, id: i32, x: f32, y: f32, t_ms: f64) {
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        match phase {
            PHASE_DOWN => {
                if self.primary.is_none() {
                    self.primary = Some(id);
                    self.secondary = None;
                    e.pointer_down(x, y);
                } else if self.secondary.is_none() && self.primary != Some(id) {
                    self.secondary = Some((id, x, y));
                    e.pointer2(PHASE_DOWN, x, y);
                }
            }
            PHASE_MOVE => {
                if self.primary == Some(id) {
                    if t_ms > 0.0 {
                        e.pointer_move_at(x, y, t_ms);
                    } else {
                        e.pointer_move(x, y);
                    }
                } else if let Some(s) = self.secondary.as_mut().filter(|s| s.0 == id) {
                    (s.1, s.2) = (x, y);
                    e.pointer2(PHASE_MOVE, x, y);
                }
            }
            PHASE_UP => {
                if self.secondary.map(|s| s.0) == Some(id) {
                    self.secondary = None;
                    e.pointer2(PHASE_UP, x, y);
                } else if self.primary == Some(id) {
                    match self.secondary.take() {
                        // 第一根先抬：第二根接班（见文件头）
                        Some((sid, sx, sy)) => {
                            e.pointer2(PHASE_UP, sx, sy);
                            self.primary = Some(sid);
                        }
                        None => {
                            self.primary = None;
                            e.pointer_up(x, y);
                        }
                    }
                }
            }
            PHASE_CANCEL => {
                if self.primary.is_none() && self.secondary.is_none() {
                    return;
                }
                if self.secondary.take().is_some() {
                    e.pointer2(PHASE_CANCEL, 0.0, 0.0);
                }
                self.primary = None;
                e.pointer_cancel();
            }
            _ => {}
        }
    }
}
