//! `<picker>` 的 `bindcolumnchange` / `bindcancel` 与 multiSelector 多列联动（G-3 的 picker 部分）。
//!
//! 依据官方 component/picker.html：
//! - multiSelector 某一列的值改变时触发 `columnchange`，`e.detail = {column, value}`，两个都是**数字**；
//! - 页面在回调里 setData 改 `range`，弹着的面板立刻换上新选项；
//! - 「取消」与点遮罩收起都触发 `cancel`。
//!
//! 走 `MiniEngine`（SDK 链路：真指针 → 触摸状态机 → 面板命中），独立集成测试二进制的原因同
//! `component_e2e.rs`：小程序根目录是进程级的。
use mini_render::host::picker_sheet::{HEADER_H, ITEM_H, SHEET_H, VISIBLE_ROWS};
use mini_render::host::MiniEngine;
use std::path::PathBuf;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

const W: f32 = 375.0;
const H: f32 = 667.0;

const WXML: &str = r#"<picker id="pk" mode="multiSelector" range="{{range}}" value="{{value}}"
  bindchange="onChange" bindcolumnchange="onColumn" bindcancel="onCancel">
  <view style="height:120px">pick</view>
</picker>"#;

const JS: &str = r#"
const CITIES = [['a1', 'a2'], ['b1', 'b2', 'b3'], ['c1']];
Page({
  data: { range: [['A', 'B', 'C'], CITIES[0]], value: [0, 0], log: [], cancels: 0, picked: null },
  onColumn(e) {
    const d = e.detail;
    this.data.log.push(typeof d.column + ':' + typeof d.value + ':' + d.column + '=' + d.value);
    const patch = { log: this.data.log };
    // 微信文档里的典型写法：第 0 列换了，第 1 列整列换成对应的选项
    if (d.column === 0) patch['range[1]'] = CITIES[d.value];
    this.setData(patch);
  },
  onChange(e) { this.setData({ picked: e.detail.value }); },
  onCancel() { this.setData({ cancels: this.data.cancels + 1 }); },
});
"#;

fn fixture(name: &str) -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("picker-e2e").join(name);
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", WXML),
        ("pages/index/index.wxss", ""),
        ("pages/index/index.js", JS),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    Fixture(root)
}

struct E {
    e: MiniEngine,
    t: u64,
}

impl E {
    fn new(fx: &Fixture) -> Self {
        let data = fx.0.join("_data");
        let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), W as u32, H as u32, 2.0)
            .expect("创建引擎");
        e.set_animation_time(2.0);
        e.launch(Some("pages/index/index")).expect("打开页面");
        let mut me = E { e, t: 1000 };
        me.frames(3);
        me
    }
    fn frames(&mut self, n: u32) {
        for _ in 0..n {
            self.t += 16;
            self.e.pump(self.t);
        }
    }
    fn tap(&mut self, x: f32, y: f32) {
        self.e.pointer_down(x, y);
        self.frames(1);
        self.e.pointer_up(x, y);
        self.frames(2);
    }
    fn js(&self, code: &str) -> String {
        self.e.eval(code).unwrap_or_default().trim_matches('"').to_string()
    }
    /// 点开面板，等入场动画（按墙上时钟走 0.22s）走完 —— 没走完时面板还在半路，坐标会落空
    fn open(&mut self) {
        self.tap(100.0, 60.0);
        assert!(self.e.picker_sheet_open(), "点在 <picker> 上应弹出面板（坐标落空的话下面全是空转）");
        std::thread::sleep(std::time::Duration::from_millis(300));
        self.frames(2);
    }
    /// 点第 `col` 列（共 `cols` 列）相对中心第 `delta` 行
    fn tap_row(&mut self, col: usize, cols: usize, delta: i32) {
        let col_w = W / cols as f32;
        let wheel_top = H - SHEET_H + HEADER_H;
        let center = (VISIBLE_ROWS / 2) as i32;
        let y = wheel_top + (center + delta) as f32 * ITEM_H + ITEM_H / 2.0;
        self.tap(col_w * col as f32 + col_w / 2.0, y);
    }
    fn confirm(&mut self) {
        self.tap(W - 30.0, H - SHEET_H + HEADER_H / 2.0);
    }
    fn cancel(&mut self) {
        self.tap(30.0, H - SHEET_H + HEADER_H / 2.0);
    }
}

#[test]
fn columnchange_detail_is_numbers_and_range_relinks_open_sheet() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("relink");
    let mut e = E::new(&fx);
    e.open();

    // 第 0 列往下一行：A → B
    e.tap_row(0, 2, 1);
    assert_eq!(
        e.js("JSON.stringify(__currentPage.data.log)"),
        r#"["number:number:0=1"]"#,
        "columnchange 的 detail.column / detail.value 都应是 number"
    );
    // 页面把第 1 列换成 3 项；面板弹着也要跟上 —— 没跟上的话第 1 列只有 2 项，下面点到第 3 项会被夹回 1
    e.frames(2);
    e.tap_row(1, 2, 2);
    assert_eq!(e.js("JSON.stringify(__currentPage.data.log)"), r#"["number:number:0=1","number:number:1=2"]"#);
    assert!(e.e.picker_sheet_open(), "点选不关面板");

    e.confirm();
    assert_eq!(e.js("JSON.stringify(__currentPage.data.picked)"), "[1,2]", "确定后 bindchange 拿到联动后的下标");
    assert_eq!(e.js("__currentPage.data.cancels"), "0", "确定不算取消");
}

#[test]
fn unchanged_column_does_not_fire_columnchange() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("nochange");
    let mut e = E::new(&fx);
    e.open();
    // 点在中心行（已选中的那一项）上：值没变，不该发
    e.tap_row(0, 2, 0);
    // 第 0 列已在首项，往上点被夹住：值也没变
    e.tap_row(0, 2, -1);
    assert_eq!(e.js("JSON.stringify(__currentPage.data.log)"), "[]");
}

#[test]
fn cancel_button_and_mask_fire_bindcancel() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("cancel");
    let mut e = E::new(&fx);

    e.open();
    e.cancel();
    assert!(!e.e.picker_sheet_open(), "点取消应收起");
    assert_eq!(e.js("__currentPage.data.cancels"), "1");
    assert_eq!(e.js("JSON.stringify(__currentPage.data.picked)"), "null", "取消不发 change");

    // 等退场动画走完再开下一次
    std::thread::sleep(std::time::Duration::from_millis(300));
    e.frames(2);
    e.open();
    e.tap(W / 2.0, 100.0); // 遮罩（面板上方）
    assert!(!e.e.picker_sheet_open(), "点遮罩应收起");
    assert_eq!(e.js("__currentPage.data.cancels"), "2");
}
