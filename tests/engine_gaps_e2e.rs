//! 写 12 个新示例时撞到的引擎问题，每条一个最小复现（期望值按微信 / Skyline）。
//!
//! 走 `MiniEngine`（SDK 链路，真指针）。小程序根目录是进程级的：用例之间用 SERIAL 串起来。
use mini_render::host::MiniEngine;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

static SERIAL: Mutex<()> = Mutex::new(());
const W: f32 = 375.0;
const H: f32 = 667.0;

struct Fx(PathBuf);
impl Drop for Fx {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// 一个小程序：`pages` 是 (路由, wxml, js, wxss)，app.json 按顺序登记
fn fixture(name: &str, app_json_extra: &str, app_wxss: &str, pages: &[(&str, &str, &str, &str)]) -> Fx {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("engine-gaps").join(name);
    std::fs::remove_dir_all(&root).ok();
    let list: Vec<String> = pages.iter().map(|p| format!("\"{}\"", p.0)).collect();
    let app_json = format!(r#"{{"pages":[{}],"window":{{"navigationStyle":"custom"}}{app_json_extra}}}"#, list.join(","));
    let mut files: Vec<(String, String)> = vec![
        ("app.json".into(), app_json),
        ("app.js".into(), "App({});".into()),
        ("app.wxss".into(), app_wxss.into()),
    ];
    for (route, wxml, js, wxss) in pages {
        files.push((format!("{route}.wxml"), wxml.to_string()));
        files.push((format!("{route}.js"), js.to_string()));
        files.push((format!("{route}.wxss"), wxss.to_string()));
        files.push((format!("{route}.json"), "{}".into()));
    }
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    Fx(root)
}

struct E {
    e: MiniEngine,
    t: u64,
}

impl E {
    fn new(fx: &Fx) -> E {
        let data = fx.0.join("_data");
        let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), W as u32, H as u32, 2.0).expect("创建引擎");
        e.set_animation_clock(0.0);
        e.launch(None).expect("打开首页");
        let mut me = E { e, t: 1000 };
        me.frames(6);
        me
    }
    fn frames(&mut self, n: u32) {
        for _ in 0..n {
            self.t += 16;
            std::thread::sleep(Duration::from_millis(2));
            self.e.pump(self.t);
        }
    }
    fn wait(&mut self, ms: u64) {
        let s = std::time::Instant::now();
        while s.elapsed() < Duration::from_millis(ms) {
            self.frames(1);
            std::thread::sleep(Duration::from_millis(14));
        }
    }
    fn tap(&mut self, x: f32, y: f32) {
        self.e.pointer_down(x, y);
        self.frames(1);
        self.e.pointer_up(x, y);
        self.frames(4);
    }
    fn val(&self, expr: &str) -> Value {
        let s = self.e.eval(&format!("JSON.stringify(({expr}))")).unwrap_or_default();
        serde_json::from_str(s.trim()).unwrap_or(Value::Null)
    }
    fn data(&self, k: &str) -> Value {
        self.val(&format!("getCurrentPages().slice(-1)[0].data.{k}"))
    }
    /// 选择器第一个节点 (left, top, width, height)
    fn rect(&mut self, sel: &str) -> Option<(f32, f32, f32, f32)> {
        let s = serde_json::to_string(sel).unwrap();
        self.e
            .eval(&format!("globalThis.__q = undefined; wx.createSelectorQuery().select({s}).boundingClientRect(function (r) {{ globalThis.__q = r; }}).exec();"))
            .ok();
        self.frames(4);
        let r = self.val("globalThis.__q");
        let f = |k: &str| r.get(k).and_then(|v| v.as_f64()).map(|v| v as f32);
        Some((f("left")?, f("top")?, f("width")?, f("height")?))
    }
    fn px(&mut self, x: f32, y: f32) -> (u8, u8, u8) {
        let (w, _) = self.e.pixel_size();
        let p = self.e.rgba();
        let i = (((y * 2.0) as usize) * w as usize + (x * 2.0) as usize) * 4;
        (p[i], p[i + 1], p[i + 2])
    }
}

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|p| p.into_inner())
}

#[test]
fn class_里的_display_none_生效() {
    let _g = serial();
    let fx = fixture(
        "display-none",
        "",
        ".gone2 { display: none; }",
        &[(
            "pages/index/index",
            r#"<view class="gone" style="height:100px;background:#ff0000"></view><view class="gone2" style="height:100px;background:#00ff00"></view><view class="after" style="height:50px;background:#0000ff"></view>"#,
            "Page({})",
            ".gone { display: none; }",
        )],
    );
    let mut e = E::new(&fx);
    assert_eq!(e.px(100.0, 20.0), (0, 0, 255), "两个 display:none 的块都不占位，蓝块顶到最上面");
    let r = e.rect(".after").expect("量 .after");
    assert_eq!(r.1, 0.0);
}

#[test]
fn progress_的_active_color_不被继承的_color_盖掉() {
    let _g = serial();
    let fx = fixture(
        "progress-color",
        "",
        "page { color: #111111; }",
        &[("pages/index/index", r##"<progress percent="100" activeColor="#ff0000" stroke-width="20" />"##, "Page({})", "")],
    );
    let mut e = E::new(&fx);
    // 进度条在第一行，取它中间
    let mut red = 0;
    for y in 0..30 {
        for x in (10..360).step_by(10) {
            let p = e.px(x as f32, y as f32);
            if p.0 > 200 && p.1 < 60 && p.2 < 60 {
                red += 1;
            }
        }
    }
    assert!(red > 20, "进度条该是 activeColor 红色（红像素 {red}）");
}

#[test]
fn switch_的_change_不冒泡到外层() {
    let _g = serial();
    let fx = fixture(
        "switch-bubble",
        "",
        "",
        &[(
            "pages/index/index",
            r#"<view bindchange="outer"><switch class="sw" bindchange="inner" /></view>"#,
            "Page({ data: { log: [] }, inner(e) { this.data.log.push('inner:' + e.detail.value); this.setData({ log: this.data.log }); }, outer(e) { this.data.log.push('outer'); this.setData({ log: this.data.log }); } })",
            "",
        )],
    );
    let mut e = E::new(&fx);
    let (x, y, w, h) = e.rect(".sw").unwrap();
    e.tap(x + w / 2.0, y + h / 2.0);
    assert_eq!(e.data("log"), serde_json::json!(["inner:true"]), "change 是不冒泡的组件事件");
}

#[test]
fn switch_点过之后_set_data_改_checked_同步() {
    let _g = serial();
    let fx = fixture(
        "switch-sync",
        "",
        "",
        &[(
            "pages/index/index",
            r#"<switch class="sw" checked="{{on}}" bindchange="ch" />"#,
            "Page({ data: { on: false, got: [] }, ch(e) { this.data.got.push(e.detail.value); this.setData({ on: e.detail.value, got: this.data.got }); } })",
            "",
        )],
    );
    let mut e = E::new(&fx);
    let (x, y, w, h) = e.rect(".sw").unwrap();
    e.tap(x + w / 2.0, y + h / 2.0);
    e.e.eval("getCurrentPages()[0].setData({ on: false })").ok();
    e.frames(4);
    e.tap(x + w / 2.0, y + h / 2.0);
    assert_eq!(e.data("got"), serde_json::json!([true, true]), "页面把 checked 设回 false 之后再点，应该是打开");
}

#[test]
fn swiper_切页后_节点查询坐标跟着平移() {
    let _g = serial();
    let fx = fixture(
        "swiper-query",
        "",
        "",
        &[(
            "pages/index/index",
            r#"<swiper current="{{cur}}" style="height:200px"><swiper-item><view class="a" style="margin-left:12px;height:50px;background:#f00"></view></swiper-item><swiper-item><view class="b" style="margin-left:12px;height:50px;background:#0f0"></view></swiper-item><swiper-item><view class="c" style="margin-left:12px;height:50px;background:#00f"></view></swiper-item></swiper>"#,
            "Page({ data: { cur: 0 } })",
            "",
        )],
    );
    let mut e = E::new(&fx);
    e.e.eval("getCurrentPages()[0].setData({ cur: 2 })").ok();
    e.wait(1200);
    assert_eq!(e.px(100.0, 20.0), (0, 0, 255), "画面切到了第三页");
    let r = e.rect(".c").unwrap();
    assert_eq!(r.0, 12.0, "第三页的节点在视口里 left = 12");
    let r = e.rect(".a").unwrap();
    assert!(r.0 < 0.0, "第一页的节点在视口左边外面：{}", r.0);
}

#[test]
fn fixed_容器里的_radio_group_能选() {
    let _g = serial();
    let fx = fixture(
        "fixed-radio",
        "",
        "",
        &[(
            "pages/index/index",
            r#"<view style="height:2000px"></view><view style="position:fixed;bottom:0;left:0;right:0;height:80px;background:#eee"><radio-group bindchange="ch" style="display:flex;flex-direction:row"><label class="l1" style="width:150px;height:60px"><radio value="day" />日间</label><label class="l2" style="width:150px;height:60px"><radio value="night" />夜间</label></radio-group></view>"#,
            "Page({ data: { v: '' }, ch(e) { this.setData({ v: e.detail.value }); } })",
            "",
        )],
    );
    let mut e = E::new(&fx);
    let (x, y, _, h) = e.rect(".l1").unwrap();
    e.tap(x + 10.0, y + h / 2.0);
    assert_eq!(e.data("v"), "day", "点 fixed 底栏里的单选框本身");
    let (x, y, w, h) = e.rect(".l2").unwrap();
    e.tap(x + w / 2.0, y + h / 2.0);
    assert_eq!(e.data("v"), "night", "点 fixed 底栏里 label 的文字「夜间」");
}

#[test]
fn calc_混合_vh_与_px_rpx() {
    let _g = serial();
    let fx = fixture(
        "calc-mixed",
        "",
        "",
        &[(
            "pages/index/index",
            r#"<view class="a"></view><view class="b"></view><view class="c"></view>"#,
            "Page({})",
            ".a { height: calc(100vh - 52px); } .b { height: calc(100vh - 104rpx); } .c { height: calc(50% + 10px); width: calc(100vw - 30px); }",
        )],
    );
    let mut e = E::new(&fx);
    assert_eq!(e.rect(".a").unwrap().3, 615.0, "calc(100vh - 52px)");
    assert_eq!(e.rect(".b").unwrap().3, 615.0, "calc(100vh - 104rpx)（375 宽下 104rpx = 52px）");
    assert_eq!(e.rect(".c").unwrap().2, 345.0, "calc(100vw - 30px)");
}

#[test]
fn dataset_里的表达式值保留类型() {
    let _g = serial();
    let fx = fixture(
        "dataset-type",
        "",
        "",
        &[(
            "pages/index/index",
            r#"<view class="v" style="height:60px" data-code="{{code}}" data-n="{{n}}" data-lit="12" data-zero="007" bindtap="t"></view>"#,
            "Page({ data: { code: '600519', n: 7, got: null }, t(e) { const d = e.currentTarget.dataset; this.setData({ got: [typeof d.code, d.code, typeof d.n, d.n, typeof d.lit, typeof d.zero] }); } })",
            "",
        )],
    );
    let mut e = E::new(&fx);
    e.tap(100.0, 30.0);
    assert_eq!(
        e.data("got"),
        serde_json::json!(["string", "600519", "number", 7, "number", "string"]),
        "{{}} 绑定的值保留 JS 类型；字面量 data-lit=\"12\" 还原成 number（引擎既有约定）、007 保持字符串"
    );
}

#[test]
fn relaunch_带_query_栈底_redirect_替换() {
    let _g = serial();
    let fx = fixture(
        "nav-query",
        "",
        "",
        &[
            ("pages/a/a", "<view>a</view>", "Page({})", ""),
            ("pages/b/b", "<view>b</view>", "Page({ data: { q: null }, onLoad(q) { this.setData({ q }); } })", ""),
        ],
    );
    let mut e = E::new(&fx);
    e.e.eval("wx.reLaunch({ url: '/pages/b/b?id=7&x=y' })").ok();
    e.frames(8);
    assert_eq!(e.data("q"), serde_json::json!({ "id": "7", "x": "y" }), "reLaunch 的 query 交给 onLoad");
    assert_eq!(e.val("getCurrentPages().length"), 1);
    e.e.eval("wx.redirectTo({ url: '/pages/a/a' })").ok();
    e.frames(8);
    assert_eq!(e.val("getCurrentPages().map(p => p.route)"), serde_json::json!(["pages/a/a"]), "栈底那页 redirectTo：换掉，不是叠上去");
}

/// 主字体（macOS 冬青黑）的 Unicode 表里没有「›」(U+203A)：从前查到了它那张 Mac 平台 GB 编码表，
/// 取回「:」的字形（「持仓 ›」画成「持仓 :」）。现在只查 Unicode 子表，缺字回退到符号字体
#[test]
fn 单书名号不画成冒号() {
    let _g = serial();
    let fx = fixture(
        "glyph",
        "",
        "",
        &[("pages/index/index", r#"<view style="font-size:60px;height:80px">›</view><view style="font-size:60px;height:80px">:</view>"#, "Page({})", "")],
    );
    let mut e = E::new(&fx);
    let (w, _) = e.e.pixel_size();
    let px = e.e.rgba().to_vec();
    // 两行各 160 设备像素高：取每行里的墨迹（深色像素）位置
    let ink = |row: usize| -> Vec<(usize, usize)> {
        let mut v = Vec::new();
        for y in row * 160..(row + 1) * 160 {
            for x in 0..200usize {
                let i = (y * w as usize + x) * 4;
                if px[i] < 128 {
                    v.push((x, y - row * 160));
                }
            }
        }
        v
    };
    let (a, b) = (ink(0), ink(1));
    assert!(!a.is_empty() && !b.is_empty(), "两个字都画出来了");
    assert_ne!(a, b, "「›」与「:」画得一模一样");
}

/// `<text>` 里嵌套带底色 / 内边距的 `<text>`（标签）：整段一块底，字都在底里。
/// 从前逐字切开、每字一块圆角底，内边距把字推出自己的盒子（火车票确认订单「李明 本人」的「本」被盖掉一半）
#[test]
fn 嵌套的标签文字_一整块底色() {
    let _g = serial();
    let fx = fixture(
        "nested-tag",
        "",
        "page{color:#1d2129;font-size:28rpx}.tag{font-size:20rpx;padding:2rpx 10rpx;border-radius:6rpx;background:#eaf1ff;color:#2b6de8}",
        &[("pages/index/index", r#"<view style="padding:20rpx"><text style="font-size:28rpx">李明<text class="tag"> 本人 </text></text></view>"#, "Page({})", "")],
    );
    let mut e = E::new(&fx);
    let (w, _) = e.e.pixel_size();
    let px = e.e.rgba().to_vec();
    // 标签那一行（20rpx 内边距之下 4~22 逻辑像素）里，底色 #eaf1ff 的最长连续一段
    let mut best = 0;
    for y in (12..44).step_by(2) {
        let mut run = 0;
        for x in 0..w as usize {
            let i = (y * w as usize + x) * 4;
            let bg = (px[i] as i32 - 0xea).abs() < 6 && (px[i + 1] as i32 - 0xf1).abs() < 6 && px[i + 2] > 245;
            let ink = px[i + 2] > 180 && px[i] < 120; // 标签里的蓝字
            run = if bg || ink { run + 1 } else { 0 };
            best = best.max(run);
        }
    }
    // 两个 10px 的汉字 + 左右各 5px 内边距 = 30 逻辑像素 = 60 设备像素（逐字切开时每块只有 20 上下）
    assert!(best >= 54, "标签底色应连成一整块（最长 {best} 设备像素）");
}
