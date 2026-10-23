//! 桌面窗体的右上角胶囊：与 SDK 同一份几何与画法（`mini_render::host::capsule`），
//! 只在 `navigationStyle: custom` 的页面上画（普通页面桌面窗体没有导航栏，也就没有胶囊）。
//!
//! - 「•••」：查看关于 —— 弹一个 Modal 展示小程序信息（`mini_render::app_about`，演示用）；
//! - 「⊙」：返回宿主 —— 交互窗体关掉窗口；无头模式只打一行日志（脚本断言用）。
//!
//! 按下整串触摸归胶囊（不建手势、不发 touchstart），抬手还在按下的那一半里才算点了。
//! 画在 fixed 覆盖层的画布最上面（`frame_render.rs` 里那一段），所以状态变了要整帧重画一次。
//! `MINI_MENU_BUTTON=0` 关掉（对照「没有胶囊」的画面时用）。
use mini_render::host::capsule::{self, Half};
use mini_render::Rect;
use serde_json::Value;

pub struct DesktopCapsule {
    enabled: bool,
    /// app.json 的 `window`（与页面 json 合并后判断 navigationStyle / 前景色）
    app_window: Value,
    /// 关于信息（启动时读一次）
    about: Value,
    down: Option<Half>,
    inside: bool,
    /// `wx.setNavigationBarColor` 改过的前景色：（路由, 是否白色）
    front: Option<(String, bool)>,
    /// 点了「⊙」：交互窗体在事件循环里关窗口
    pub exit_requested: bool,
}

impl DesktopCapsule {
    pub fn new() -> Self {
        let app_json = serde_json::from_str::<Value>(&super::page_loader::load_app_json()).unwrap_or(Value::Null);
        let about = mini_render::app_about::about(&super::page_loader::get_app_path(), &app_json);
        Self {
            enabled: std::env::var("MINI_MENU_BUTTON").map(|v| v != "0").unwrap_or(true),
            app_window: app_json.get("window").cloned().unwrap_or(Value::Null),
            about,
            down: None,
            inside: false,
            front: None,
            exit_requested: false,
        }
    }
}

impl crate::MiniAppWindow {
    /// 当前页画不画胶囊、画在哪、是不是深色导航栏（白图标）
    pub(crate) fn capsule_spec(&self) -> Option<(Rect, bool)> {
        let c = &self.capsule;
        if !c.enabled {
            return None;
        }
        let route = &self.page_stack.last()?.path;
        let page = self.pages.get(route).map(|p| &p.window);
        let get = |k: &str| page.and_then(|w| w.get(k)).or_else(|| c.app_window.get(k)).and_then(|v| v.as_str());
        if get("navigationStyle") != Some("custom") {
            return None;
        }
        let dark = match &c.front {
            Some((r, white)) if r == route => *white,
            _ => get("navigationBarTextStyle") == Some("white"),
        };
        // 与 SDK 默认（不给安全区）同一份系统信息：状态栏 20、375 宽 → 281,24 87×32
        let info = mini_render::host::engine_options::system_info(
            super::LOGICAL_WIDTH,
            super::LOGICAL_HEIGHT,
            self.scale_factor as f32,
            &Default::default(),
        );
        Some((capsule::rect(&info), dark))
    }

    /// fixed 层重画时要画的：（位置, 深色, 按压态）
    pub(crate) fn capsule_paint(&self) -> Option<(Rect, bool, Option<Half>)> {
        let pressed = self.capsule.down.filter(|_| self.capsule.inside);
        self.capsule_spec().map(|(r, dark)| (r, dark, pressed))
    }

    /// 逻辑层发给宿主的事件：桌面关心导航栏前景色（胶囊换配色）与 hostCall（选图 / 选文件，
    /// 交给 host_calls.rs），其余的桌面本来就不消费
    pub(crate) fn capsule_sync_host_events(&mut self) {
        let route = self.page_stack.last().map(|p| p.path.clone()).unwrap_or_default();
        for raw in self.app.take_host_events() {
            let Ok(v) = serde_json::from_str::<Value>(&raw) else { continue };
            if v["type"] == "navigationBar" {
                if let Some(fc) = v.get("frontColor").and_then(|c| c.as_str()) {
                    self.capsule.front = Some((route.clone(), fc.eq_ignore_ascii_case("#ffffff")));
                    self.capsule_repaint();
                }
            } else if v["type"] == "hostCall" {
                self.queue_host_call(v);
            }
        }
    }

    /// 胶囊变了：fixed 层整帧重画（局部重绘帧会沿用旧的 fixed 画布）
    fn capsule_repaint(&mut self) {
        self.fixed_dirty = true;
        self.needs_redraw = true;
        self.force_full_redraw = true;
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// 按下：落在胶囊上返回 true（这串触摸归胶囊）
    pub(crate) fn capsule_press(&mut self, x: f32, y: f32) -> bool {
        let Some(half) = self.capsule_spec().and_then(|(r, _)| capsule::half_at(r, x, y)) else {
            return false;
        };
        self.capsule.down = Some(half);
        self.capsule.inside = true;
        self.capsule_repaint();
        true
    }

    /// 移动：这串触摸归胶囊时返回 true；移出按下的那一半褪掉按压态
    pub(crate) fn capsule_move(&mut self, x: f32, y: f32) -> bool {
        let Some(half) = self.capsule.down else { return false };
        let inside = self.capsule_spec().and_then(|(r, _)| capsule::half_at(r, x, y)) == Some(half);
        if inside != self.capsule.inside {
            self.capsule.inside = inside;
            self.capsule_repaint();
        }
        true
    }

    /// 抬起：这串触摸归胶囊时返回 true；还在按下的那一半里就执行它
    pub(crate) fn capsule_release(&mut self, x: f32, y: f32) -> bool {
        let Some(half) = self.capsule.down.take() else { return false };
        self.capsule.inside = false;
        self.capsule_repaint();
        if self.capsule_spec().and_then(|(r, _)| capsule::half_at(r, x, y)) == Some(half) {
            self.capsule_activate(half);
        }
        true
    }

    fn capsule_activate(&mut self, half: Half) {
        match half {
            Half::Menu => {
                println!("🔘 胶囊 menu：查看关于");
                let (title, content) = mini_render::app_about::dialog_text(&self.capsule.about);
                let mut m = super::ui_overlay::ModalState::new(&title, &content);
                m.show_cancel = false;
                self.modal = Some(m);
            }
            Half::Close => {
                println!("🔘 胶囊 close：返回宿主{}", if self.window.is_some() { "（关闭窗口）" } else { "（无头：只记日志）" });
                self.capsule.exit_requested = self.window.is_some();
            }
        }
    }
}
