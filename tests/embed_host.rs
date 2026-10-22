//! 嵌入宿主（Flutter 插件 / 原生 View）那组接口的端到端用例：每个用例现场写一个
//! 最小小程序，用 `MiniEngine` 跑起来，按**可判定**的判据断言。
//!
//! ## 为什么是集成测试（独立进程），不放进 `src/tests`
//! 小程序根目录、数据目录、tabBar 高度、网络完成队列都是**进程级**的（一个进程一个
//! 引擎的约定，见 `data_dir.rs` 的「已知限制」）。单元测试全部用 `sample/sample-app`，
//! 根目录在并行用例之间是同一个值；这里每个用例一个夹具目录，放进同一进程里并行跑就会
//! 互相改掉对方的根目录。集成测试是独立的二进制，再用一把锁串行，就互不干扰。
use mini_render::compositor::HostEnv;
use mini_render::host::{EngineOptions, MiniEngine, SafeAreaInsets};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 取锁一律 `unwrap_or_else(into_inner)`：一个用例 panic 会把锁毒掉，`unwrap()` 的话后面每个用例
/// 都在取锁那一行失败 —— 一次 `css_无限动画逐帧出新画面` 的偶发失败（只在多线程跑时出现）报成 9 个失败
static SERIAL: Mutex<()> = Mutex::new(());

/// 写一个只有一页（`pages/p/p`）的小程序；`extra` 是额外的 (相对路径, 内容)
fn fixture(name: &str, wxml: &str, wxss: &str, js: &str, page_json: &str, extra: &[(&str, &str)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("embed-host").join(name);
    std::fs::remove_dir_all(&root).ok();
    let files: Vec<(&str, &str)> = vec![
        ("app.json", r##"{"pages":["pages/p/p","pages/q/q"],"window":{"navigationBarTitleText":"夹具","navigationBarBackgroundColor":"#ffffff"}}"##),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", wxml),
        ("pages/p/p.wxss", wxss),
        ("pages/p/p.js", js),
        ("pages/p/p.json", page_json),
        ("pages/q/q.wxml", "<view>第二页</view>"),
        ("pages/q/q.wxss", ""),
        ("pages/q/q.js", "Page({data:{}});"),
        ("pages/q/q.json", r#"{"navigationBarTitleText":"第二页"}"#),
    ];
    for (rel, text) in files.iter().chain(extra.iter()) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn engine(root: &PathBuf, w: u32, h: u32, dpr: f32, opts: EngineOptions) -> MiniEngine {
    let data = root.join("_data");
    let mut e = MiniEngine::new_with_options(
        &root.to_string_lossy(),
        Some(&data.to_string_lossy()),
        w,
        h,
        dpr,
        HostEnv::default(),
        opts,
    )
    .expect("创建引擎");
    e.launch(None).expect("打开首页");
    e
}

fn events(e: &mut MiniEngine) -> Vec<Value> {
    std::iter::from_fn(|| e.poll_event()).map(|s| serde_json::from_str(&s).expect("事件是合法 JSON")).collect()
}

fn js(e: &MiniEngine, code: &str) -> String {
    e.eval(code).unwrap_or_default()
}

const WANTS_MORE: u32 = 16;
const NEW: u32 = 1;

#[test]
fn 静止页面上_settimeout_不靠触摸也能按时触发() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "timer",
        "<view>{{n}}</view>",
        "",
        "Page({data:{n:0},onLoad(){setTimeout(()=>{this.setData({n:1})},500)}});",
        "{}",
        &[],
    );
    let mut e = engine(&root, 375, 667, 1.0, EngineOptions::default());
    let start = Instant::now();
    let mut t = 1u64;
    let mut fired_at = None;
    // 宿主按「WANTS_MORE 就继续挂 vsync」的规矩出帧：定时器在飞时这一位必须一直在
    while start.elapsed() < Duration::from_millis(900) {
        let f = e.pump_ex(t);
        if js(&e, "String(getCurrentPages()[0].data.n)") == "1" {
            fired_at = Some(start.elapsed());
            assert!(f & NEW != 0, "setData 那一帧要出新画面");
            break;
        }
        assert!(f & WANTS_MORE != 0, "定时器没到点时必须要求下一帧，否则宿主会停 vsync，回调永远不跑");
        std::thread::sleep(Duration::from_millis(16));
        t += 16;
    }
    let at = fired_at.expect("500ms 的定时器在 900ms 内没触发");
    // 定时器按墙上时钟走；开发机常年高负载（load 30+），给 80ms 的调度余量
    assert!(at <= Duration::from_millis(520 + 80), "触发晚了：{at:?}");
    // 定时器走完、页面静止：不再要求出帧（静止页面零耗电）
    for _ in 0..3 {
        t += 16;
        e.pump_ex(t);
    }
    assert_eq!(e.pump_ex(t + 16) & WANTS_MORE, 0, "静止页面不该再要帧");
}

#[test]
fn css_无限动画逐帧出新画面() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "anim",
        r#"<view class="box"></view>"#,
        "@keyframes slide { from { transform: translateX(0px); } to { transform: translateX(200px); } }
         .box { width: 50px; height: 50px; background-color: #ff0000; animation: slide 1s linear infinite; }",
        "Page({data:{}});",
        "{}",
        &[],
    );
    let mut e = engine(&root, 375, 667, 1.0, EngineOptions::default());
    let mut produced = 0;
    let mut frames = Vec::new();
    for i in 0..60u64 {
        let f = e.pump_ex(1 + i * 16);
        if f & NEW != 0 {
            produced += 1;
        }
        assert!(f & WANTS_MORE != 0, "动画在跑就要一直要帧（第 {i} 帧）");
        if i == 1 || i == 30 {
            frames.push(e.rgba().to_vec());
        }
        std::thread::sleep(Duration::from_millis(4));
    }
    assert!(produced >= 59, "60 次 pump 只出了 {produced} 帧（动画只画第一帧就停）");
    assert_ne!(frames[0], frames[1], "第 1 帧与第 30 帧应不同（动画在动）");
}

#[test]
fn 系统信息与安全区来自宿主() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "sysinfo",
        r#"<view class="bar"></view>"#,
        ".bar { position: fixed; left: 0; right: 0; bottom: 0; height: calc(100rpx + env(safe-area-inset-bottom)); background-color: #ff0000; }",
        "Page({data:{}});",
        "{}",
        &[],
    );
    let opts = EngineOptions::from_json(
        r#"{"safeArea":{"top":47,"right":0,"bottom":34,"left":0},"systemInfo":{"platform":"ios","model":"夹具机"}}"#,
    )
    .unwrap();
    let mut e = engine(&root, 375, 812, 1.0, opts);
    e.pump(1);
    let info: Value = serde_json::from_str(&js(&e, "JSON.stringify(wx.getSystemInfoSync())")).unwrap();
    assert_eq!(info["windowWidth"], 375.0);
    assert_eq!(info["windowHeight"], 812.0);
    assert_eq!(info["statusBarHeight"], 47.0);
    assert_eq!(info["safeArea"]["bottom"], 812.0 - 34.0);
    assert_eq!(info["platform"], "ios");
    assert_eq!(info["model"], "夹具机");

    // 底部吸底栏高 = 100rpx(=50px @375) + 34px 安全区 = 84px；按红色行数数
    let red_rows = |e: &mut MiniEngine| {
        let (w, h) = e.pixel_size();
        let px = e.rgba().to_vec();
        (0..h as usize)
            .filter(|y| {
                let i = (y * w as usize + w as usize / 2) * 4;
                px[i] > 200 && px[i + 1] < 60 && px[i + 2] < 60
            })
            .count()
    };
    assert_eq!(red_rows(&mut e), 84, "env(safe-area-inset-bottom) 没折进 calc");
    // 安全区变了（比如转屏后底部横条没了）：样式表重解析，吸底栏回到 50px
    assert!(e.set_safe_area(SafeAreaInsets { top: 20.0, ..Default::default() }));
    assert_eq!(red_rows(&mut e), 50);
    let info: Value = serde_json::from_str(&js(&e, "JSON.stringify(wx.getSystemInfoSync())")).unwrap();
    assert_eq!(info["statusBarHeight"], 20.0);
}

#[test]
fn 宿主实现的_api_往返() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "hostcall",
        r#"<view>{{n}}</view>"#,
        "",
        "Page({data:{n:-1, typ:''},
           onLoad(){
             this.setData({typ: typeof wx.chooseImage + '/' + typeof wx.getLocation});
             wx.getLocation({fail:(r)=>this.setData({loc: r.errMsg})});
             wx.chooseImage({count:2, success:(r)=>this.setData({n:r.tempFilePaths.length}), fail:()=>this.setData({n:-2})});
           }});",
        "{}",
        &[],
    );
    let opts = EngineOptions { host_apis: vec!["chooseImage".into()], ..Default::default() };
    let mut e = engine(&root, 375, 667, 1.0, opts);
    e.pump(1);
    // 微信里这些能力总是函数：点名的交给宿主；没点名的是引擎的默认实现，立刻 fail 并写明原因
    // （从前是 undefined —— 框架包装层 `uni.getLocation → wx.getLocation` 拿到 undefined 直接 TypeError）
    assert_eq!(js(&e, "getCurrentPages()[0].data.typ"), "function/function");
    e.pump(2);
    assert!(
        js(&e, "getCurrentPages()[0].data.loc").starts_with("getLocation:fail"),
        "没点名的立刻 fail，不发 hostCall"
    );
    let evs = events(&mut e);
    let call = evs.iter().find(|v| v["type"] == "hostCall").expect("要有 hostCall 事件");
    assert_eq!(call["api"], "chooseImage");
    assert_eq!(call["args"]["count"], 2);
    let id = call["id"].as_u64().unwrap() as u32;
    assert!(e.host_reply(id, true, r#"{"tempFilePaths":["/a.jpg","/b.jpg"]}"#));
    assert!(!e.host_reply(id, true, "{}"), "同一个 id 回第二次要报不认识");
    assert!(e.pump(17), "回包里 setData 了，要出新画面");
    assert_eq!(js(&e, "String(getCurrentPages()[0].data.n)"), "2");
}

#[test]
fn 页面与导航栏事件() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "navbar",
        "<view>页</view>",
        "",
        "Page({onLoad(){ console.log('夹具日志'); wx.setNavigationBarTitle({title:'改过的标题'}); }});",
        r#"{"navigationBarTitleText":"页面标题","navigationBarTextStyle":"white"}"#,
        &[],
    );
    let mut e = engine(&root, 375, 667, 1.0, EngineOptions::default());
    e.pump(1);
    let evs = events(&mut e);
    let page = evs.iter().find(|v| v["type"] == "page").expect("page 事件");
    assert_eq!(page["route"], "pages/p/p");
    assert_eq!(page["depth"], 1);
    // 运行时改动压过页面 json，页面 json 压过 app.json
    assert_eq!(page["window"]["navigationBarTitleText"], "改过的标题");
    assert_eq!(page["window"]["navigationBarTextStyle"], "white");
    assert_eq!(page["window"]["navigationBarBackgroundColor"], "#ffffff");
    assert!(evs.iter().any(|v| v["type"] == "navigationBar" && v["title"] == "改过的标题"));
    assert!(evs.iter().any(|v| v["type"] == "log" && v["text"].as_str().unwrap_or("").contains("夹具日志")));

    e.navigate("/pages/q/q").unwrap();
    e.pump(17);
    let evs = events(&mut e);
    let page = evs.iter().find(|v| v["type"] == "page").expect("换页要有 page 事件");
    assert_eq!((page["route"].as_str(), page["depth"].as_u64()), (Some("pages/q/q"), Some(2)));
    assert_eq!(page["window"]["navigationBarTitleText"], "第二页");
    assert!(e.back());
    e.pump(33);
    let evs = events(&mut e);
    let page = evs.iter().find(|v| v["type"] == "page").expect("返回也要有 page 事件");
    assert_eq!(page["window"]["navigationBarTitleText"], "改过的标题", "返回后恢复那一页自己的标题");
}

#[test]
fn 输入框聚焦事件带键盘类型与位置() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "input",
        r#"<view style="height:100px"></view><input class="i" type="number" maxlength="6" confirm-type="search" placeholder="手机号" bindconfirm="onC"/>"#,
        ".i { width: 300px; height: 40px; }",
        "Page({data:{c:''}, onC(e){ this.setData({c: e.detail.value}); }});",
        "{}",
        &[],
    );
    let mut e = engine(&root, 375, 667, 1.0, EngineOptions::default());
    e.pump(1);
    events(&mut e);
    e.pointer_down(50.0, 120.0);
    e.pump(17);
    e.pointer_up(50.0, 120.0);
    e.pump(33);
    let evs = events(&mut e);
    let f = evs.iter().find(|v| v["type"] == "inputFocus").expect("点输入框要发 inputFocus");
    assert_eq!(f["inputType"], "number");
    assert_eq!(f["maxlength"], 6);
    assert_eq!(f["confirmType"], "search");
    assert_eq!(f["placeholder"], "手机号");
    let r = &f["rect"];
    assert!(r["y"].as_f64().unwrap() >= 99.0 && r["y"].as_f64().unwrap() <= 101.0, "rect={r}");
    assert!((r["height"].as_f64().unwrap() - 40.0).abs() < 1.0);
    // 聚焦期间静止的 pump 不再每帧都报新画面（只有光标翻转那帧）
    let produced = (0..10u64).filter(|i| e.pump_ex(40 + i * 5) & NEW != 0).count();
    assert!(produced <= 2, "聚焦后 10 次紧挨着的 pump 出了 {produced} 帧");
    e.text_input("123");
    e.key("enter");
    e.pump(200);
    assert_eq!(js(&e, "getCurrentPages()[0].data.c"), "123");
    assert!(events(&mut e).iter().any(|v| v["type"] == "inputBlur"), "确认后要发 inputBlur（收键盘）");
}

#[test]
fn 宿主时钟下长按照样触发() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "longpress",
        r#"<view class="b" bindlongpress="onL">按住</view>"#,
        ".b { width: 200px; height: 200px; }",
        "Page({data:{l:0}, onL(){ this.setData({l:1}); }});",
        "{}",
        &[],
    );
    let mut e = engine(&root, 375, 667, 1.0, EngineOptions::default());
    // 宿主时钟是开机时间（几十亿毫秒量级），与引擎自己的时钟不是一个量纲 ——
    // 从前按下用墙上时钟、长按计时用宿主时钟，长按永远不触发
    let base = 3_000_000_000u64;
    e.pump(base);
    e.pointer_down(100.0, 100.0);
    let mut t = base;
    for _ in 0..30 {
        t += 16;
        let f = e.pump_ex(t);
        assert!(f & WANTS_MORE != 0, "手指按住不动时也要一直要帧（长按靠帧驱动）");
    }
    e.pointer_up(100.0, 100.0);
    assert_eq!(js(&e, "String(getCurrentPages()[0].data.l)"), "1", "按住 480ms 应触发 longpress");
}

#[test]
fn 原生组件只报位置不自己画_事件回传() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "native",
        r#"<view style="height:300px"></view><video id="v" src="https://example.invalid/a.mp4" style="width:200px;height:100px" bindended="onEnd"></video><view style="height:2000px"></view>"#,
        "",
        "Page({data:{ended:0}, onEnd(e){ this.setData({ended: e.detail.n}); }});",
        "{}",
        &[],
    );
    let opts = EngineOptions { native_components: vec!["video".into()], ..Default::default() };
    let mut e = engine(&root, 375, 667, 1.0, opts);
    e.pump(1);
    let evs = events(&mut e);
    let nv = evs.iter().find(|v| v["type"] == "nativeViews").expect("要报原生组件摆放");
    let v = &nv["views"][0];
    assert_eq!(v["key"], "v");
    assert_eq!(v["tag"], "video");
    assert_eq!(v["attrs"]["src"], "https://example.invalid/a.mp4");
    assert!((v["y"].as_f64().unwrap() - 300.0).abs() < 1.0 && (v["height"].as_f64().unwrap() - 100.0).abs() < 1.0, "{v}");
    // 不变就不重复报
    e.pump(17);
    assert!(!events(&mut e).iter().any(|v| v["type"] == "nativeViews"));
    // 滚动之后位置跟着变（视口坐标）
    e.scroll_to(100.0);
    e.pump(33);
    let evs = events(&mut e);
    let v = &evs.iter().find(|v| v["type"] == "nativeViews").expect("滚动后要重报")["views"][0];
    assert!((v["y"].as_f64().unwrap() - 200.0).abs() < 1.0, "{v}");
    assert!(e.dispatch_native_view_event("v", "ended", serde_json::json!({"n": 7})));
    assert!(!e.dispatch_native_view_event("v", "nope", serde_json::json!({})), "没绑的事件返回 false");
    assert_eq!(js(&e, "String(getCurrentPages()[0].data.ended)"), "7");
    assert!(!mini_render::renderer::components::has_playing_video(), "宿主承载时引擎不该自己建播放器");
}

#[test]
fn 改尺寸后布局按新宽度_并派发_onresize() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "resize",
        r#"<view class="half"></view>"#,
        ".half { width: 375rpx; height: 10px; background-color: #0000ff; }",
        "Page({data:{r:''}, onResize(res){ this.setData({r: res.size.windowWidth + 'x' + res.size.windowHeight}); }});",
        "{}",
        &[],
    );
    let mut e = engine(&root, 375, 667, 1.0, EngineOptions::default());
    e.pump(1);
    let blue_cols = |e: &mut MiniEngine| {
        let (w, _) = e.pixel_size();
        let px = e.rgba().to_vec();
        (0..w as usize).filter(|x| px[x * 4 + 2] > 200 && px[x * 4] < 60).count()
    };
    assert_eq!(blue_cols(&mut e), 188, "375rpx@375 宽 ≈ 一半");
    assert!(e.resize(800, 600, 1.0));
    assert_eq!(e.pixel_size(), (800, 600));
    assert_eq!(blue_cols(&mut e), 400, "rpx 要按新屏宽换算");
    assert_eq!(js(&e, "getCurrentPages()[0].data.r"), "800x600");
    let info: Value = serde_json::from_str(&js(&e, "JSON.stringify(wx.getSystemInfoSync())")).unwrap();
    assert_eq!(info["deviceOrientation"], "landscape");
}

#[test]
fn 冷启动深链带_query() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("launchq", "<view>{{x}}</view>", "", "Page({data:{x:''}, onLoad(q){ this.setData({x: q.x}); }});", "{}", &[]);
    let data = root.join("_data");
    let mut e = MiniEngine::new_with_options(
        &root.to_string_lossy(),
        Some(&data.to_string_lossy()),
        375,
        667,
        1.0,
        HostEnv::default(),
        EngineOptions::default(),
    )
    .unwrap();
    e.launch(Some("/pages/p/p?x=5")).expect("带 query 的路由也要能打开");
    assert_eq!(e.current_route(), "pages/p/p");
    assert_eq!(js(&e, "getCurrentPages()[0].data.x"), "5");
}

#[test]
fn picker_面板贴着真实视口底边() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "picker",
        r#"<picker mode="selector" range="{{items}}" value="0" bindchange="onP"><view class="p">选</view></picker>"#,
        ".p { width: 200px; height: 50px; }",
        "Page({data:{items:['甲','乙','丙'], v:-1}, onP(e){ this.setData({v: e.detail.value, t: typeof e.detail.value}); }});",
        "{}",
        &[],
    );
    // 852 高（不是写死的 667）：面板头部在 852 - SHEET_H
    let mut e = engine(&root, 393, 852, 1.0, EngineOptions::default());
    e.pump(1);
    e.pointer_down(50.0, 20.0);
    e.pump(17);
    e.pointer_up(50.0, 20.0);
    let mut t = 33;
    for _ in 0..30 {
        e.pump(t);
        t += 16;
        std::thread::sleep(Duration::from_millis(10)); // 入场动画按墙上时钟走（0.22s）
    }
    let top = 852.0 - mini_render::host::picker_sheet::SHEET_H;
    // 点「确定」：头部右半边
    e.pointer_down(380.0, top + 10.0);
    e.pump(t);
    e.pointer_up(380.0, top + 10.0);
    e.pump(t + 16);
    assert_eq!(js(&e, "String(getCurrentPages()[0].data.v)"), "0", "确定没点中（面板位置还按 667 算？）");
    assert_eq!(js(&e, "getCurrentPages()[0].data.t"), "number", "selector 的 detail.value 是 number");
}
