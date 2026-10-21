//! `<picker-view>` 内嵌滚轮（G-8）：`value` 定初始位置、拖动跟手、松手吸附到整项、
//! `bindchange` 的 `detail.value` 是各列下标数组（number）、`bindpickstart` / `bindpickend`。
//!
//! 依据官方 component/picker-view.html。走 `MiniEngine`（SDK 链路的真指针 + 手势仲裁）。
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

const ITEMS: [&str; 5] = ["#ff0000", "#00ff00", "#0000ff", "#ffff00", "#ff00ff"];

fn fixture() -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("picker-view-e2e");
    std::fs::remove_dir_all(&root).ok();
    let items: String = ITEMS.iter().map(|c| format!(r#"<view style="height:40px;background:{c}"></view>"#)).collect();
    let wxml = format!(
        r#"<picker-view style="width:300px;height:200px" value="{{{{value}}}}" bindchange="onChange" bindpickstart="onStart" bindpickend="onEnd">
             <picker-view-column>{items}</picker-view-column>
             <picker-view-column>{items}</picker-view-column>
           </picker-view>"#
    );
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#.to_string()),
        ("app.js", "App({});".to_string()),
        ("app.wxss", String::new()),
        ("pages/index/index.json", "{}".to_string()),
        ("pages/index/index.wxml", wxml),
        ("pages/index/index.wxss", String::new()),
        (
            "pages/index/index.js",
            "Page({ data: { value: [1, 0], log: [] }, onChange(e) { this.data.log.push('change:' + JSON.stringify(e.detail.value) + ':' + typeof e.detail.value[0]); }, onStart() { this.data.log.push('start'); }, onEnd() { this.data.log.push('end'); } });".to_string(),
        ),
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
    fn frames(&mut self, n: u32) {
        for _ in 0..n {
            self.t += 16;
            self.e.pump(self.t);
        }
    }
    fn px(&mut self, x: usize, y: usize) -> (u8, u8, u8) {
        let (w, _) = self.e.pixel_size();
        let p = self.e.rgba();
        let i = (y * 2 * w as usize + x * 2) * 4;
        (p[i], p[i + 1], p[i + 2])
    }
    fn log(&self) -> String {
        self.e.eval("getCurrentPages()[0].data.log.join(' ')").unwrap_or_default().trim_matches('"').to_string()
    }
}

fn hex(c: &str) -> (u8, u8, u8) {
    let v = u32::from_str_radix(&c[1..], 16).unwrap();
    ((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

#[test]
fn picker_view_drag_snaps_and_reports_indices() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = fixture();
    let data = fx.0.join("_data");
    let mut e = E { e: MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).unwrap(), t: 1000 };
    e.e.set_animation_time(2.0);
    e.e.launch(Some("pages/index/index")).unwrap();
    e.frames(4);
    // value=[1,0]：第一列正中（y=100）是第 1 项（绿），第二列是第 0 项（红）。看指示条中间偏下一点（遮罩之外）
    assert_eq!(e.px(75, 100), hex(ITEMS[1]), "第一列选中第 1 项");
    assert_eq!(e.px(225, 100), hex(ITEMS[0]), "第二列选中第 0 项");
    // 第一列往上拖 80px（两项）：慢慢拖，松手速度近 0
    e.e.pointer_down(75.0, 150.0);
    e.frames(1);
    for i in 1..=16 {
        e.e.pointer_move(75.0, 150.0 - 5.0 * i as f32);
        e.frames(1);
    }
    e.frames(8); // 停住再松手
    e.e.pointer_up(75.0, 70.0);
    e.frames(30); // 吸附动画 260ms
    assert_eq!(e.px(75, 100), hex(ITEMS[3]), "吸附到第 3 项");
    assert_eq!(e.log(), "start change:[3,0]:number end", "pickstart → change（下标是 number）→ pickend");
    // 页面 setData 改 value：跳过去、不发 change
    e.e.eval("getCurrentPages()[0].setData({ value: [0, 4] })").unwrap();
    e.frames(3);
    assert_eq!(e.px(75, 100), hex(ITEMS[0]));
    assert_eq!(e.px(225, 100), hex(ITEMS[4]));
    assert_eq!(e.log(), "start change:[3,0]:number end", "受控改值不发事件");
}
