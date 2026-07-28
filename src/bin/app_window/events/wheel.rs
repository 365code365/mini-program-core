//! 滚轮 / 触控板双指滑动。
//!
//! 与拖动手势用**同一套仲裁规则**（`app_window::gesture` + `pointer_input`）：
//! 主轴锁定 → 内层不可滚就交给外层 → 到边界继续推也交给外层 → 最后落到页面。
//!
//! 从前这里是「命中点上最内层的 scroll-view 无条件接管」，三个后果在触控板上尤其明显：
//!   - **主轴不锁**：光标停在横向卡片列表上时，纵向双指滑动被那条列表按 `delta_x` 吃掉，
//!     整页不动、列表还会横向抖一下；
//!   - **不可滚也接管**：uni-app 产物的结构是整页套一层 `<scroll-view class="page">`，
//!     它自己 `max_scroll == 0`（页面级滚动才是干活的那个）—— 于是双指滑动全被它吞掉，
//!     只在原地橡皮筋、60ms 后弹回，页面一动不动；
//!   - **到边界不传递**：内层滚到底之后继续推不会接着滚页面。
//!
//! 拖动路径早就有这三条（`🖐 手势归属` / `🖐 … 不可滚 → 交给页面`），滚轮路径漏了。

use mini_render::ui::interaction::InteractionManager;
use mini_render::ui::scroll_controller::ScrollController;
use winit::event::MouseScrollDelta;

/// 一次滚轮/触控板事件的位移（逻辑像素）与来源
struct WheelDelta {
    x: f32,
    y: f32,
    /// 触控板/精密鼠标：一次事件就是一次真实位移；滚轮是脉冲
    precise: bool,
}

impl WheelDelta {
    fn parse(delta: MouseScrollDelta, scale_factor: f64) -> Self {
        match delta {
            // 行滚动（普通滚轮）：一格约 20 逻辑像素
            MouseScrollDelta::LineDelta(x, y) => Self { x: -x * 20.0, y: -y * 20.0, precise: false },
            // winit 把 macOS 的 point 位移换算成物理像素，这里换回逻辑像素做 1:1 跟手
            MouseScrollDelta::PixelDelta(pos) => Self {
                x: -pos.x as f32 / scale_factor as f32,
                y: -pos.y as f32 / scale_factor as f32,
                precise: true,
            },
        }
    }

    /// 主轴上的位移：取两轴里绝对值更大的那个方向（本次事件的滚动轴）
    fn dominant(&self) -> (bool, f32) {
        if self.x.abs() > self.y.abs() {
            (true, self.x)
        } else {
            (false, self.y)
        }
    }
}

/// 这个滚动控制器能不能吃下这次位移（可滚，且没有顶在这个方向的边界上）。
///
/// 顶在边界上仍然接管的话，内层会在原地橡皮筋而外层/页面一动不动 ——
/// 浏览器与微信都是把剩下的位移交给外层。
fn can_take(c: &ScrollController, delta: f32) -> bool {
    if c.get_max_scroll() <= 0.5 {
        return false;
    }
    let pos = c.get_position();
    if delta > 0.0 {
        pos < c.get_max_scroll() - 0.5
    } else {
        pos > 0.5
    }
}

/// 处理一次滚轮/触控板事件。返回是否需要重绘。
///
/// `lock_page_scroll` 为真时（光标停在弹窗遮罩上）不落到页面滚动，
/// 但覆盖层内部自己的 scroll-view 照常可滚 —— 与微信一致。
pub fn handle(
    delta: MouseScrollDelta,
    mouse_pos: (f32, f32),
    interaction: &mut InteractionManager,
    page_scroll: &mut ScrollController,
    scale_factor: f64,
    lock_page_scroll: bool,
) -> bool {
    let d = WheelDelta::parse(delta, scale_factor);
    if d.x.abs() < 0.1 && d.y.abs() < 0.1 {
        return false;
    }
    let (horizontal, step) = d.dominant();
    if step.abs() < 0.1 {
        return false;
    }

    let (x, y) = mouse_pos;
    // 覆盖层用视口坐标，正常流要加上页面滚动量
    let actual_y = y + page_scroll.get_position();

    // 由内到外找第一个「轴匹配且吃得下」的滚动区：先覆盖层，再正常流
    let target = pick_target(interaction, x, y, true, horizontal, step)
        .or_else(|| pick_target(interaction, x, actual_y, false, horizontal, step));

    if let Some(id) = target {
        if let Some(c) = interaction.get_scroll_controller_mut(&id) {
            c.handle_scroll(step, d.precise);
            return true;
        }
    }

    // 没人接：横向位移不该驱动页面（页面只有纵向滚动）
    if horizontal || lock_page_scroll {
        return false;
    }
    let before = page_scroll.get_position();
    page_scroll.handle_scroll(step, d.precise);
    // 页面滚动同样要请求重绘。缺了这一句时只有「命中页面内 scroll-view」才会重绘，
    // 页面级滚动只改了滚动位置：上屏按新偏移取画布，而画布上还是上一帧那条带
    // —— 滑动过程一片空白，停下后被别的原因触发一次重绘内容才出现。
    //
    // 位置没变也算处理过：内容不足一屏时页面仍允许越界橡皮筋，那一下也要出帧。
    (page_scroll.get_position() - before).abs() > 0.001 || page_scroll.is_animating()
}

/// 在一个坐标系里由内到外挑接管者
fn pick_target(
    interaction: &InteractionManager,
    x: f32,
    y: f32,
    fixed: bool,
    horizontal: bool,
    step: f32,
) -> Option<String> {
    let log = std::env::var("MINI_SCROLL_LOG").is_ok();
    for area in interaction.scroll_areas_at(x, y, fixed) {
        if area.is_horizontal != horizontal {
            continue; // 轴不匹配：纵向滑动不该驱动横向列表，反之亦然
        }
        match interaction.get_scroll_controller(&area.id) {
            Some(c) if can_take(c, step) => return Some(area.id.clone()),
            other => {
                if log {
                    eprintln!(
                        "🖱 {} 吃不下这次滚动（max_scroll={:?} pos={:?}）→ 交给外层",
                        area.id,
                        other.map(|c| c.get_max_scroll()),
                        other.map(|c| c.get_position()),
                    );
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use mini_render::ui::interaction::{InteractionType, InteractiveElement};
    use mini_render::Rect;
    use winit::dpi::PhysicalPosition;

    /// 造一个滚动区（`content` > `viewport` 才可滚）
    fn area(id: &str, y: f32, h: f32, horizontal: bool, content: f32) -> InteractiveElement {
        InteractiveElement {
            interaction_type: InteractionType::ScrollArea,
            id: id.to_string(),
            bounds: Rect::new(0.0, y, 375.0, h),
            checked: false,
            value: String::new(),
            disabled: false,
            min: 0.0,
            max: 100.0,
            content_height: if horizontal { h } else { content },
            viewport_height: h,
            content_width: if horizontal { content } else { 375.0 },
            viewport_width: 375.0,
            is_horizontal: horizontal,
            is_fixed: false,
        }
    }

    /// 触控板：dy>0 表示内容向上走（滚动位置变大）
    fn wheel_down(px: f32) -> MouseScrollDelta {
        MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -(px as f64) * 2.0))
    }

    fn wheel_right(px: f32) -> MouseScrollDelta {
        MouseScrollDelta::PixelDelta(PhysicalPosition::new(-(px as f64) * 2.0, 0.0))
    }

    /// 整页套一层 `<scroll-view class="page">`（uni-app 产物的标准结构）时，
    /// 它自己不可滚，双指滑动必须落到**页面**上。
    ///
    /// 这是 tea-app 上「触控板双指滑动不顺手」的主因：从前最内层的 scroll-view
    /// 无条件接管，于是所有位移都被这层壳吃掉，只在原地橡皮筋、60ms 后弹回。
    #[test]
    fn unscrollable_wrapper_hands_off_to_page() {
        let mut im = InteractionManager::new();
        im.register_element(area("page_wrap", 0.0, 667.0, false, 667.0)); // content == viewport
        let mut page = ScrollController::new(1400.0, 667.0);

        let redraw = handle(wheel_down(60.0), (187.0, 300.0), &mut im, &mut page, 2.0, false);
        assert!(redraw);
        assert!(
            (page.get_position() - 60.0).abs() < 0.01,
            "页面应当滚动 60px，实际 {}",
            page.get_position()
        );
        assert_eq!(
            im.get_scroll_controller("page_wrap").map(|c| c.get_position()),
            Some(0.0),
            "不可滚的壳不该动"
        );
    }

    /// 光标停在横向卡片列表上时，纵向双指滑动要滚页面（主轴锁定）
    #[test]
    fn vertical_wheel_over_horizontal_list_scrolls_page() {
        let mut im = InteractionManager::new();
        im.register_element(area("rail", 200.0, 160.0, true, 1200.0));
        let mut page = ScrollController::new(1400.0, 667.0);

        handle(wheel_down(50.0), (187.0, 260.0), &mut im, &mut page, 2.0, false);
        assert!((page.get_position() - 50.0).abs() < 0.01, "页面应当滚动");
        assert_eq!(
            im.get_scroll_controller("rail").map(|c| c.get_position()),
            Some(0.0),
            "横向列表不该被纵向滑动带着走"
        );
    }

    /// 横向双指滑动要滚那条横向列表，且**不该**驱动页面
    #[test]
    fn horizontal_wheel_scrolls_the_rail_only() {
        let mut im = InteractionManager::new();
        im.register_element(area("rail", 200.0, 160.0, true, 1200.0));
        let mut page = ScrollController::new(1400.0, 667.0);

        handle(wheel_right(50.0), (187.0, 260.0), &mut im, &mut page, 2.0, false);
        assert!(
            im.get_scroll_controller("rail").map(|c| c.get_position()).unwrap_or(0.0) > 40.0,
            "横向列表应当滚动"
        );
        assert_eq!(page.get_position(), 0.0, "页面不该被横向滑动带着走");
    }

    /// 内层纵向列表滚到底之后，继续推要接着滚页面（嵌套传递）。
    ///
    /// 跨越边界的那一下内层会橡皮筋出去几像素（松手 `end_wheel_gesture` 再弹回，
    /// 与触控板一致），所以这里不较真内层的精确落点，只验证语义链：
    /// 内层还能滚时页面不动、内层到头后继续滚会落到页面。
    #[test]
    fn inner_list_at_end_chains_to_page() {
        let mut im = InteractionManager::new();
        im.register_element(area("list", 0.0, 400.0, false, 500.0)); // 可滚 100
        let mut page = ScrollController::new(1400.0, 667.0);

        // 内层还没到头时页面绝不能动
        handle(wheel_down(30.0), (187.0, 200.0), &mut im, &mut page, 2.0, false);
        assert!(im.get_scroll_controller("list").unwrap().get_position() > 0.0);
        assert_eq!(page.get_position(), 0.0, "内层还能滚时页面不该动");

        // 把内层推到头（含一次跨界的橡皮筋）
        for _ in 0..5 {
            handle(wheel_down(30.0), (187.0, 200.0), &mut im, &mut page, 2.0, false);
        }
        assert!(
            im.get_scroll_controller("list").unwrap().get_position() >= 99.0,
            "内层应当已到达其上限区"
        );

        // 到头后继续推 → 落到页面
        let before = page.get_position();
        handle(wheel_down(30.0), (187.0, 200.0), &mut im, &mut page, 2.0, false);
        handle(wheel_down(30.0), (187.0, 200.0), &mut im, &mut page, 2.0, false);
        assert!(
            page.get_position() > before + 1.0,
            "内层到头后应当把滚动交给页面，实际 {} → {}",
            before,
            page.get_position()
        );
    }

    /// 指针停在弹窗遮罩上时页面锁住（覆盖层自己的 scroll-view 仍可滚）
    #[test]
    fn page_scroll_locked_over_overlay() {
        let mut im = InteractionManager::new();
        let mut page = ScrollController::new(1400.0, 667.0);
        handle(wheel_down(60.0), (187.0, 300.0), &mut im, &mut page, 2.0, true);
        assert_eq!(page.get_position(), 0.0);

        let mut sheet = area("sheet_body", 100.0, 400.0, false, 900.0);
        sheet.is_fixed = true;
        im.register_element(sheet);
        handle(wheel_down(60.0), (187.0, 300.0), &mut im, &mut page, 2.0, true);
        assert!(
            im.get_scroll_controller("sheet_body").map(|c| c.get_position()).unwrap_or(0.0) > 50.0,
            "覆盖层内部的 scroll-view 仍应可滚"
        );
        assert_eq!(page.get_position(), 0.0, "页面仍然锁住");
    }
}
