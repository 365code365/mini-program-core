//! 示例小程序的场景测试共用的一套驱动（`#[path = "sample_harness/mod.rs"] mod harness;`）。
//!
//! 走 `MiniEngine`（SDK 链路）：真指针 → 触摸状态机 → 命中 → 事件派发，与真机同一条路；
//! 不用 `--click` 那种绕过指针层的捷径（见 doc/引擎测试说明.md「点击类回归只认 --touch」）。
//!
//! - 找元素靠逻辑层的 `wx.createSelectorQuery`（与页面量节点同一份布局），点它的中心；
//! - 逻辑层报错（`console.error`、事件处理函数 / 生命周期抛错打的 `[Error]` / `[Page.xxx]`）全部记下，
//!   [`App::assert_no_errors`] 一条都不许有 —— 示例页「点了没反应」多半是处理函数里抛了异常；
//! - 弹窗 / 操作菜单 / picker 面板按引擎画的几何点（与 tests/system_api_e2e.rs、picker_e2e.rs 同一套数）。
//!
//! 每个示例一个测试二进制（小程序根目录、组件模板是进程级的），同一个二进制里的用例用 [`SERIAL`] 串起来。
#![allow(dead_code)]
use mini_render::host::picker_sheet::{HEADER_H, ITEM_H, SHEET_H, VISIBLE_ROWS};
use mini_render::host::{EngineOptions, MiniEngine};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

pub static SERIAL: Mutex<()> = Mutex::new(());

pub const W: f32 = 375.0;
pub const H: f32 = 667.0;
pub const DPR: f32 = 2.0;

/// 拿 [`SERIAL`]（前一个用例 panic 了也接着跑）
pub fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|p| p.into_inner())
}

const HOOK: &str = r#"
globalThis.__errs = [];
(function () {
  var bad = /\[Error\]|\[Page\.|\[Component\.|\[property\.|\[observers|TypeError|ReferenceError|SyntaxError|RangeError|is not a function|Cannot read|is not defined|Timer error/;
  var p = globalThis.__native_print;
  if (typeof p === 'function') {
    globalThis.__native_print = function (s) { var t = String(s); if (bad.test(t)) __errs.push(t); return p.apply(this, arguments); };
  }
  var ce = console.error;
  console.error = function () {
    __errs.push(Array.prototype.slice.call(arguments).map(function (a) { return a && a.message ? a.message : String(a); }).join(' '));
    return ce.apply(console, arguments);
  };
})();
"#;

pub struct App {
    pub e: MiniEngine,
    pub t: u64,
    pub name: String,
}

impl App {
    /// 打开 `sample/<name>` 的 `route`（None = 首页）。存储目录每个用例单独一份、开跑前清空
    pub fn open(name: &str, route: Option<&str>, tag: &str) -> App {
        Self::open_with(name, route, tag, EngineOptions::default())
    }

    pub fn open_with(name: &str, route: Option<&str>, tag: &str, opts: EngineOptions) -> App {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample").join(name);
        assert!(root.join("app.json").exists(), "没有示例 {name}");
        let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sample-e2e").join(format!("{name}-{tag}"));
        std::fs::remove_dir_all(&data).ok();
        let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), W as u32, H as u32, DPR, Default::default(), opts)
            .expect("创建引擎");
        e.set_animation_clock(0.0);
        e.eval(HOOK).expect("装错误钩子");
        e.launch(route).unwrap_or_else(|err| panic!("打开 {name} {route:?} 失败：{err}"));
        let mut app = App { e, t: 1000, name: name.to_string() };
        app.settle();
        app
    }

    /// 推 `n` 帧（每帧 16ms 引擎时钟，墙上时钟也走一点：setTimeout 按墙上时钟）
    pub fn frames(&mut self, n: u32) {
        for _ in 0..n {
            self.t += 16;
            std::thread::sleep(Duration::from_millis(2));
            self.e.pump(self.t);
        }
    }

    /// 等页面安定：转场动画、setData 回调、定时器 0 延迟
    pub fn settle(&mut self) {
        self.frames(24);
    }

    /// 墙上时钟等 `ms`（定时器、倒计时、模拟网络延迟），期间照常出帧
    pub fn wait_ms(&mut self, ms: u64) {
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(ms) {
            self.t += 16;
            std::thread::sleep(Duration::from_millis(16));
            self.e.pump(self.t);
        }
        self.frames(2);
    }

    pub fn js(&self, code: &str) -> String {
        self.e.eval(code).unwrap_or_default()
    }

    /// 求一个 JS 表达式（JSON 化后解析）
    pub fn val(&self, expr: &str) -> Value {
        let s = self.js(&format!("JSON.stringify(({expr}))"));
        serde_json::from_str(s.trim()).unwrap_or(Value::Null)
    }

    /// 当前页 `data` 下的路径（`a.b[0]`）
    pub fn data(&self, path: &str) -> Value {
        self.val(&format!("getCurrentPages().slice(-1)[0].data.{path}"))
    }

    pub fn route(&self) -> String {
        self.val("getCurrentPages().slice(-1)[0].route").as_str().unwrap_or("").to_string()
    }

    pub fn depth(&self) -> usize {
        self.val("getCurrentPages().length").as_u64().unwrap_or(0) as usize
    }

    pub fn storage(&self, key: &str) -> Value {
        let k = serde_json::to_string(key).unwrap();
        self.val(&format!("wx.getStorageSync({k})"))
    }

    /// 选择器命中的所有节点（视口坐标，逻辑像素）：`[(x, y, w, h)]`
    pub fn rects(&mut self, sel: &str) -> Vec<(f32, f32, f32, f32)> {
        let s = serde_json::to_string(sel).unwrap();
        self.js(&format!(
            "globalThis.__q = null; wx.createSelectorQuery().selectAll({s}).boundingClientRect(function (r) {{ globalThis.__q = r; }}).exec();"
        ));
        for _ in 0..8 {
            self.frames(1);
            if !self.val("globalThis.__q").is_null() {
                break;
            }
        }
        self.val("globalThis.__q")
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|r| {
                        let f = |k: &str| r.get(k).and_then(|v| v.as_f64()).map(|v| v as f32);
                        Some((f("left")?, f("top")?, f("width")?, f("height")?))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn rect(&mut self, sel: &str) -> (f32, f32, f32, f32) {
        let r = self.rects(sel);
        *r.first().unwrap_or_else(|| panic!("[{}] 页面 {} 上没有 {sel}", self.name, self.route()))
    }

    pub fn exists(&mut self, sel: &str) -> bool {
        !self.rects(sel).is_empty()
    }

    pub fn count(&mut self, sel: &str) -> usize {
        self.rects(sel).len()
    }

    pub fn tap_xy(&mut self, x: f32, y: f32) {
        self.e.pointer_down(x, y);
        self.frames(1);
        self.e.pointer_up(x, y);
        self.frames(3);
    }

    /// 点选择器命中的第 `i` 个节点的中心（要在视口里：不在就先 [`Self::scroll_into`]）
    pub fn tap_nth(&mut self, sel: &str, i: usize) {
        let r = self.rects(sel);
        let &(x, y, w, h) = r.get(i).unwrap_or_else(|| panic!("[{}] {} 上 {sel} 只有 {} 个", self.name, self.route(), r.len()));
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        assert!(cy > 0.0 && cy < H && cx > 0.0 && cx < W, "[{}] {sel}#{i} 不在视口里：{:?}", self.name, (x, y, w, h));
        self.tap_xy(cx, cy);
    }

    pub fn tap(&mut self, sel: &str) {
        self.tap_nth(sel, 0);
    }

    /// 点完等页面安定（跳页、弹层入场）
    pub fn tap_settle(&mut self, sel: &str) {
        self.tap(sel);
        self.settle();
    }

    /// 长按（引擎默认 350ms 判长按，这里按住 600ms）
    pub fn long_press(&mut self, sel: &str) {
        let (x, y, w, h) = self.rect(sel);
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        self.e.pointer_down(cx, cy);
        for _ in 0..40 {
            self.t += 16;
            std::thread::sleep(Duration::from_millis(16));
            self.e.pump(self.t);
        }
        self.e.pointer_up(cx, cy);
        self.frames(4);
    }

    /// 手指从 (x, y) 拖 (dx, dy)，分 `steps` 步，每步一帧
    pub fn drag(&mut self, x: f32, y: f32, dx: f32, dy: f32, steps: u32) {
        self.e.pointer_down(x, y);
        self.frames(1);
        for i in 1..=steps {
            let k = i as f32 / steps as f32;
            self.e.pointer_move(x + dx * k, y + dy * k);
            self.frames(1);
        }
        self.e.pointer_up(x + dx, y + dy);
        self.frames(2);
    }

    /// 页面（或页面里的 scroll-view）往上滚 `dy`（手指往上拖），等惯性停下
    pub fn scroll_by(&mut self, x: f32, y: f32, dy: f32) {
        self.drag(x, y, 0.0, -dy, 12);
        self.wait_scroll_still();
    }

    /// 等页面与所有 scroll-view 的惯性 / 回弹停下（按墙上时钟走，机器忙时 900ms 不一定够）：
    /// 还在滑的时候点下去只会让它停住，不算点击
    pub fn wait_scroll_still(&mut self) {
        self.wait_ms(300);
        let snap = |e: &MiniEngine| (e.scroll_position(), e.scroll_view_offsets());
        let mut last = snap(&self.e);
        let mut still = 0;
        for _ in 0..200 {
            self.wait_ms(30);
            let now = snap(&self.e);
            if now == last {
                still += 1;
                if still >= 4 {
                    return;
                }
            } else {
                still = 0;
                last = now;
            }
        }
    }

    /// 点输入框再打字（`text` 整串上屏，等同输入法 commit）
    pub fn type_into(&mut self, sel: &str, text: &str) {
        self.tap(sel);
        self.frames(2);
        self.e.text_input(text);
        self.frames(3);
    }

    pub fn key(&mut self, name: &str) {
        self.e.key(name);
        self.frames(3);
    }

    /// 收起键盘（输入框失焦）
    pub fn blur(&mut self) {
        self.e.key("escape");
        self.frames(3);
    }

    pub fn back(&mut self) {
        self.e.back();
        self.settle();
    }

    // ───────── 系统弹层 ─────────

    pub fn modal_open(&self) -> bool {
        self.e.modal_open()
    }

    /// 点 `wx.showModal` 的确定（`true`）/ 取消：按钮在弹窗底部，高度随内容变，自下往上扫到弹窗关掉为止
    pub fn modal_tap(&mut self, confirm: bool) {
        assert!(self.e.modal_open(), "[{}] 没有弹窗", self.name);
        // 有取消键时确定在右半边；没有（showCancel:false）时整条都是确定
        let x = if confirm { W * 0.72 } else { W * 0.28 };
        let mut y = H * 0.5 + 160.0;
        while y > H * 0.5 - 40.0 {
            self.tap_xy(x, y);
            if !self.e.modal_open() {
                self.settle();
                return;
            }
            y -= 8.0;
        }
        panic!("[{}] 点不到弹窗的{}键", self.name, if confirm { "确定" } else { "取消" });
    }

    /// 点 `wx.showActionSheet` 的第 `i` 项（共 `n` 项；`i == n` 是取消）。面板每项 56、取消上面隔 8
    pub fn sheet_tap(&mut self, i: usize, n: usize) {
        // 入场动画按墙上时钟走，等它展开
        self.wait_ms(320);
        let top = H - (n as f32 * 56.0 + 8.0 + 56.0);
        let y = if i < n { top + i as f32 * 56.0 + 28.0 } else { top + n as f32 * 56.0 + 8.0 + 28.0 };
        self.tap_xy(W / 2.0, y);
        self.wait_ms(320);
    }

    pub fn picker_open(&self) -> bool {
        self.e.picker_sheet_open()
    }

    /// 点开 `<picker>`（选择器）并等面板入场
    pub fn picker(&mut self, sel: &str) {
        self.tap(sel);
        assert!(self.e.picker_sheet_open(), "[{}] 点 {sel} 没弹出 picker 面板", self.name);
        self.wait_ms(300);
    }

    /// 面板里第 `col` 列（共 `cols` 列）点相对中心第 `delta` 行
    pub fn picker_row(&mut self, col: usize, cols: usize, delta: i32) {
        let col_w = W / cols as f32;
        let wheel_top = H - SHEET_H + HEADER_H;
        let center = (VISIBLE_ROWS / 2) as i32;
        let y = wheel_top + (center + delta) as f32 * ITEM_H + ITEM_H / 2.0;
        self.tap_xy(col_w * col as f32 + col_w / 2.0, y);
        self.frames(4);
    }

    pub fn picker_confirm(&mut self) {
        self.tap_xy(W - 30.0, H - SHEET_H + HEADER_H / 2.0);
        self.wait_ms(300);
    }

    pub fn picker_cancel(&mut self) {
        self.tap_xy(30.0, H - SHEET_H + HEADER_H / 2.0);
        self.wait_ms(300);
    }

    // ───────── 判据 ─────────

    pub fn errors(&self) -> Vec<String> {
        self.val("globalThis.__errs || []")
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    }

    pub fn assert_no_errors(&self) {
        let errs = self.errors();
        assert!(errs.is_empty(), "[{}] {} 逻辑层报错：{errs:#?}", self.name, self.route());
    }

    /// 画面不是空白：视口里至少 `min` 种颜色（页面一片白 / 只剩骨架时只有一两种）
    pub fn assert_rendered(&mut self, min: usize) {
        self.frames(2);
        let px = self.e.rgba();
        let mut seen = std::collections::HashSet::new();
        for p in px.chunks_exact(4).step_by(7) {
            seen.insert([p[0] >> 3, p[1] >> 3, p[2] >> 3]);
            if seen.len() >= min {
                return;
            }
        }
        panic!("[{}] {} 画面几乎是空白：只有 {} 种颜色", self.name, self.route(), seen.len());
    }

    /// app.json 里的全部页面
    pub fn pages(&self) -> Vec<String> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample").join(&self.name);
        let j: Value = serde_json::from_str(&std::fs::read_to_string(root.join("app.json")).unwrap()).unwrap();
        j["pages"].as_array().unwrap().iter().filter_map(|p| p.as_str().map(str::to_string)).collect()
    }
}

/// 每一页都单独打开一次（带 `queries` 里给的参数）：不报错、画面不空白。
/// 详情页之类要参数的页面在 `queries` 里给（`("pages/detail/detail", "id=3")`）
pub fn smoke_all_pages(name: &str, queries: &[(&str, &str)]) {
    let probe = App::open(name, None, "smoke-home");
    probe.assert_no_errors();
    let pages = probe.pages();
    drop(probe);
    for (i, p) in pages.iter().enumerate() {
        let q = queries.iter().find(|(r, _)| r == p).map(|(_, q)| *q);
        let route = match q {
            Some(q) => format!("{p}?{q}"),
            None => p.clone(),
        };
        let mut app = App::open(name, Some(&route), &format!("smoke-{i}"));
        app.wait_ms(200);
        app.assert_no_errors();
        app.assert_rendered(6);
        assert_eq!(app.route(), *p, "[{name}] 打开 {route} 落在了别的页面");
    }
}
