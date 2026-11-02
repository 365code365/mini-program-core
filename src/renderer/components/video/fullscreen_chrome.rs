//! 全屏那一层的控件排版：安全区、放大的那套尺寸（chrome.rs 的 `FULL`）、按住的键。
//! 从 fullscreen.rs 分出来（那边管进出全屏、方向、转画面、触摸分派）；画与点击命中都走这里的 [`layout`]，
//! 两边对得上。
//!
//! ## 安全区
//! 从前控件贴着画面四边排：iPhone 竖屏全屏时顶栏（返回 + 标题）整个压在状态栏 / 灵动岛里、
//! 底下的控制条压在底部横条上（iOS 真机截图）。现在顶栏排在上安全区下面、控制条排在下安全区上面，
//! 渐变一直铺到屏幕边上（状态栏的白字、横条压在暗处也看得见）。
//!
//! 宿主报的是**屏幕**（竖屏）方向的 insets：iOS 的界面锁竖屏，横屏只是引擎把这一层转过来画，
//! 系统一直按竖屏报。换到画面方向：90（左侧朝下）时画面的左边是屏幕上沿（灵动岛那侧）、
//! 右边是屏幕下沿（横条那侧）、上边是屏幕右沿；-90 反过来。横屏两侧取大的那个对称留 ——
//! 灵动岛转到哪边都不压控件，左右也不一边宽一边窄（与系统横屏时的安全区一样是对称的）。
//! 没报过安全区（桌面、测试）就是 0，排法与从前一样贴边。
use super::chrome::{self, Bar, Flags, Key};
use super::ui::{self, Status, VideoRef};
use super::*;
use std::cell::Cell;

thread_local! {
    /// 宿主报的安全区（逻辑像素，屏幕方向 [上, 右, 下, 左]）
    static SAFE: Cell<[f32; 4]> = const { Cell::new([0.0; 4]) };
}

/// 宿主的安全区（逻辑像素，屏幕方向 [上, 右, 下, 左]）。引擎每帧从创建选项 / `set_safe_area` 同步过来；
/// 全屏期间宿主收起状态栏报来的新值引擎先按住（engine_video_feed.rs），这里拿到的一直是进全屏前那份
pub fn set_safe_insets(insets: [f32; 4]) {
    SAFE.with(|s| s.set(insets.map(|v| if v.is_finite() { v.max(0.0) } else { 0.0 })));
}

/// 画面方向的安全区 [上, 右, 下, 左]（逻辑像素）
pub fn content_insets(dir: i32) -> [f32; 4] {
    let [t, r, b, l] = SAFE.with(|s| s.get());
    let side = t.max(b);
    match dir {
        90 => [r, side, l, side],
        -90 => [l, side, r, side],
        _ => [t, r, b, l],
    }
}

/// 全屏控件的位置（画面坐标，单位 = `u`）
pub struct Layout {
    /// 画面方向的安全区 [上, 右, 下, 左]
    pub insets: [f32; 4],
    /// 顶栏左上角与宽（安全区里面）
    pub top: (f32, f32, f32),
    /// 底下的控制条（安全区里面）
    pub bar: Bar,
}

/// `u`：一个逻辑像素等于几个单位（画的时候是像素比，点击命中是 1）
pub fn layout(cw: f32, ch: f32, flags: &Flags, u: f32, duration: f64, dir: i32) -> Layout {
    let [t, r, b, l] = content_insets(dir).map(|v| v * u);
    let w = (cw - l - r).max(0.0);
    Layout {
        insets: [t, r, b, l],
        top: (l, t, w),
        bar: chrome::bar_layout(l, 0.0, w, (ch - b).max(0.0), flags, u, true, duration),
    }
}

/// 落在哪个键上（画面坐标）。`shown`：控制条在不在；`paused` 时中间有播放键
#[allow(clippy::too_many_arguments)]
pub fn key_at(lay: &Layout, cw: f32, ch: f32, u: f32, flags: &Flags, shown: bool, paused: bool, x: f32, y: f32) -> Option<Key> {
    if shown {
        if chrome::back_hit(lay.top.0, lay.top.1, u, x, y) {
            return Some(Key::Back);
        }
        if lay.bar.contains(x, y) {
            return lay.bar.key_at(x, y);
        }
    }
    (paused && flags.center_btn && chrome::center_hit(0.0, 0.0, cw, ch, u, true, x, y)).then_some(Key::Center)
}

/// 画控件（画面、黑边已经画好）。`pressed`：按住的键（底下垫一圈半透明白）
#[allow(clippy::too_many_arguments)]
pub fn paint(canvas: &mut Canvas, text: Option<&TextRenderer>, video: &VideoRef, st: &Status, flags: &Flags, u: f32, dir: i32, pressed: Option<Key>) {
    let (cw, ch) = (canvas.width() as f32, canvas.height() as f32);
    let lay = layout(cw, ch, flags, u, st.duration, dir);
    let [t, _, b, _] = lay.insets;
    if ui::controls_shown(&video.key) {
        // 顶上：安全区那截与顶栏上半截压实，下半截淡出 —— 白字压在亮画面上也看得清
        let top_h = chrome::TOP_BAR_H * u;
        let solid = t + top_h * 0.5;
        chrome::gradient(canvas, 0.0, 0.0, cw, solid, 0.55, 0.55);
        chrome::gradient(canvas, 0.0, solid, cw, top_h * 0.5 + 20.0 * u, 0.55, 0.0);
        let title = video.attrs.get("title").map(|s| s.trim()).unwrap_or("");
        chrome::draw_top_bar(canvas, text, lay.top.0, lay.top.1, lay.top.2, u, title, pressed == Some(Key::Back));
        // 底下同理：从控制条中线往下压实，一直铺到屏幕底边（横条那截）
        let bar = &lay.bar;
        let mid = bar.y + bar.h * 0.5;
        let fade = bar.h * 0.5 + 20.0 * u;
        chrome::gradient(canvas, 0.0, mid - fade, cw, fade, 0.0, 0.55);
        chrome::gradient(canvas, 0.0, mid, cw, ch - mid, 0.55, 0.55);
        chrome::draw_bar(canvas, text, bar, st, ui::drag_ratio(&video.key), u, true, pressed);
    } else if st.playing && flags.bottom_progress {
        // 收起时那条细进度贴在下安全区上沿（竖屏不压在横条里）
        chrome::draw_bottom_progress(canvas, 0.0, 0.0, cw, (ch - b).max(0.0), u, st);
    }
    if !st.playing && flags.center_btn {
        chrome::draw_center(canvas, 0.0, 0.0, cw, ch, u, true, pressed == Some(Key::Center));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 安全区按方向换到画面上_横屏两侧对称() {
        // iPhone 竖屏：上 62（灵动岛）、下 34（横条）
        set_safe_insets([62.0, 0.0, 34.0, 0.0]);
        assert_eq!(content_insets(0), [62.0, 0.0, 34.0, 0.0]);
        // 左侧朝下：灵动岛在画面左边、横条在右边 → 两侧都留 62，上下不留
        assert_eq!(content_insets(90), [0.0, 62.0, 0.0, 62.0]);
        assert_eq!(content_insets(-90), [0.0, 62.0, 0.0, 62.0]);
        let flags = {
            let mut f = Flags::of(&Default::default(), 440.0);
            f.fullscreen_btn = true;
            f
        };
        let lay = layout(440.0, 956.0, &flags, 1.0, 10.0, 0);
        assert_eq!(lay.top, (0.0, 62.0, 440.0), "顶栏在灵动岛下面");
        assert_eq!(lay.bar.y + lay.bar.h, 956.0 - 34.0, "控制条在横条上面");
        assert_eq!(key_at(&lay, 440.0, 956.0, 1.0, &flags, true, false, 20.0, 80.0), Some(Key::Back));
        assert_eq!(key_at(&lay, 440.0, 956.0, 1.0, &flags, true, false, 20.0, 20.0), Some(Key::Back), "灵动岛那截也算返回");
        let y = lay.bar.y + lay.bar.h / 2.0;
        assert_eq!(key_at(&lay, 440.0, 956.0, 1.0, &flags, true, false, 30.0, y), Some(Key::Play));
        assert_eq!(key_at(&lay, 440.0, 956.0, 1.0, &flags, true, false, 410.0, y), Some(Key::Exit));
        assert_eq!(key_at(&lay, 440.0, 956.0, 1.0, &flags, true, true, 220.0, 478.0), Some(Key::Center));
        assert_eq!(key_at(&lay, 440.0, 956.0, 1.0, &flags, false, false, 20.0, 80.0), None, "控制条收起时没有返回键");
        // 横屏：画面 956×440，按钮离开两侧 62
        let lay = layout(956.0, 440.0, &flags, 1.0, 10.0, 90);
        assert_eq!(lay.bar.play.unwrap().x, 62.0 + 12.0);
        assert_eq!(lay.bar.fullscreen.unwrap().x + 48.0, 956.0 - 62.0 - 12.0);
        set_safe_insets([0.0; 4]);
        // 没报过安全区：与从前一样贴边排（e2e 按 bar_layout(0, 0, 宽, 高) 算进度条）
        let lay = layout(375.0, 667.0, &flags, 1.0, 1.0, 0);
        let old = chrome::bar_layout(0.0, 0.0, 375.0, 667.0, &flags, 1.0, true, 1.0);
        assert_eq!((lay.bar.y, lay.bar.track), (old.y, old.track));
    }
}
