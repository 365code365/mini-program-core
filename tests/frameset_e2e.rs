//! 分栏模式（微信 frameset：阔折叠展开 / PC 窗口拉宽，src/host/engine_frameset.rs）。
//!
//! 判据：
//! - 不开分栏、或开了但窗口窄：与从前逐字节相同，一个 `frameset` 事件都没有；
//! - 拉宽到 ≥600：引擎按一栏（一半宽）排，`windowWidth` 是一栏宽；单页时活页面在左、右栏是占位；
//! - 进下一页：活页面挪到右栏，左栏是上一页 —— 与「一开始就按一栏宽打开上一页」逐字节相同；
//! - 左栏滚轮只动左栏；窄回去退回单栏，页面栈不丢。
//!
//! 夹具写在 `CARGO_TARGET_TMPDIR` 下，串行跑（小程序根目录是进程级的，见 embed_host.rs）。
use mini_render::compositor::HostEnv;
use mini_render::host::{EngineOptions, MiniEngine, PixelFormat};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());
const H: u32 = 640;

/// p：顶上一条红、下面 2000 高的绿色长页；q：白页
fn fixture(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("frameset").join(name);
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/p/p","pages/q/q"]}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", r#"<view class="bar" bindtap="inc">左栏 {{n}}</view><view class="item" hover-class="hv" bindtap="open" data-id="1">一</view><view class="item" hover-class="hv" bindtap="open" data-id="2">二</view><view class="long">长页</view><view class="end">底</view>"#),
        ("pages/p/p.wxss", "page{background-color:#00ff00;} .bar{height:80px;background:#ff0000;} .item{height:60px;} .hv{background:#888888;} .long{height:2000px;} .end{height:100px;background:#0000ff;}"),
        ("pages/p/p.js", "Page({data:{n:1}, inc(){this.setData({n:this.data.n+1})}, open(e){wx.navigateTo({url:'/pages/q/q?id='+e.currentTarget.dataset.id})}});"),
        ("pages/p/p.json", "{}"),
        ("pages/q/q.wxml", "<view>第二页 {{id}}</view>"),
        ("pages/q/q.wxss", "page{background-color:#ffffff;}"),
        ("pages/q/q.js", "Page({onLoad(o){this.id=o.id||'';this.setData({id:this.id});wx.setNavigationBarTitle({title:'Q'+this.id})}, onUnload(){globalThis.__gone=(globalThis.__gone||'')+this.id}});"),
        ("pages/q/q.json", "{}"),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn engine(root: &PathBuf, w: u32, opts: &str) -> MiniEngine {
    let data = root.join("_data");
    let opts = EngineOptions::from_json(opts).unwrap();
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), w, H, 1.0, HostEnv::default(), opts)
        .expect("创建引擎");
    e.launch(None).expect("打开首页");
    e.pump(1);
    e
}

fn frameset_events(e: &mut MiniEngine) -> Vec<Value> {
    let mut out = Vec::new();
    while let Some(ev) = e.poll_event() {
        let v: Value = serde_json::from_str(&ev).unwrap();
        if v["type"] == "frameset" {
            out.push(v);
        }
    }
    out
}

fn side(e: &mut MiniEngine) -> Vec<u8> {
    let (w, h) = e.side_pixel_size();
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let n = e.side_pixels_into(&mut buf, (w * 4) as usize, PixelFormat::Rgba8);
    assert_eq!(n, buf.len(), "另一栏取得到图");
    buf
}

fn px(buf: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * w + x) * 4) as usize;
    [buf[i], buf[i + 1], buf[i + 2]]
}

fn window_width(e: &MiniEngine) -> f64 {
    e.eval("wx.getSystemInfoSync().windowWidth").unwrap().trim().parse().unwrap()
}

#[test]
fn 窄窗口与不开分栏时和从前逐字节相同() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("narrow");
    let mut old = engine(&root, 390, "{}");
    let base = old.rgba().to_vec();
    let mut auto = engine(&root, 390, r#"{"frameset":"auto"}"#);
    assert_eq!(auto.rgba(), &base[..], "开了 auto、窗口窄：画面逐字节相同");
    assert!(frameset_events(&mut auto).is_empty(), "没分栏过就不发事件");
    // 不开分栏（app.json 也没写 frameset）：再宽也是单栏
    old.resize(900, H, 1.0);
    old.pump(20);
    assert_eq!(old.pixel_size(), (900, H));
    assert_eq!(old.side_pixel_size(), (0, 0));
    assert!(frameset_events(&mut old).is_empty());
}

#[test]
fn 拉宽分两栏_进下一页活页面到右栏_左栏与按一栏宽打开的上一页相同() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("wide");
    let mut e = engine(&root, 390, r#"{"frameset":"auto"}"#);
    frameset_events(&mut e);
    assert!(e.resize(900, H, 1.0));
    let mut t = 20;
    let flags = e.pump_ex(t);
    assert_ne!(flags & 64, 0, "FRAME_SIDE");
    assert_eq!(e.pixel_size(), (450, H), "引擎按一栏宽排");
    assert_eq!(window_width(&e), 450.0, "windowWidth 是一栏宽");
    let ev = frameset_events(&mut e);
    assert_eq!(ev.len(), 1, "{ev:?}");
    assert_eq!((ev[0]["on"].as_bool(), ev[0]["livePane"].as_str(), ev[0]["paneWidth"].as_u64()), (Some(true), Some("left"), Some(450)));
    // 单页：右栏是占位底色
    let s = side(&mut e);
    assert_eq!(e.side_pixel_size(), (450, H));
    assert_eq!(px(&s, 450, 225, 320), [0xF7, 0xF7, 0xF7]);
    let p_frame = e.rgba().to_vec();
    assert_eq!(px(&p_frame, 450, 440, 10), [0xFF, 0, 0], "红条铺满一栏宽");

    e.navigate("/pages/q/q").unwrap();
    t += 16;
    assert_ne!(e.pump_ex(t) & 64, 0, "换页：另一栏换图");
    let ev = frameset_events(&mut e);
    assert_eq!(ev.last().unwrap()["livePane"], "right");
    assert_eq!(side(&mut e), p_frame, "左栏就是进下一页之前的 p");
    assert_eq!(px(e.rgba(), 450, 225, 320), [0xFF, 0xFF, 0xFF], "右栏是 q");

    // 左栏滚轮：只重画左栏，右栏（活页面）不变
    let q_frame = e.rgba().to_vec();
    e.side_wheel(300.0, true);
    t += 16;
    assert_ne!(e.pump_ex(t) & 64, 0);
    let s = side(&mut e);
    assert_ne!(s, p_frame, "左栏滚了");
    assert_eq!(px(&s, 450, 225, 10), [0, 0xFF, 0], "红条滚出去了");
    assert_eq!(e.rgba(), &q_frame[..], "右栏不动");

    // 窄回去：单栏，栈还在，返回后 p 停在左栏滚到的位置
    assert!(e.resize(390, H, 1.0));
    t += 16;
    e.pump(t);
    assert_eq!(e.pixel_size(), (390, H));
    assert_eq!(e.side_pixel_size(), (0, 0));
    assert_eq!(frameset_events(&mut e).last().unwrap()["on"], false);
    assert_eq!(e.page_depth(), 2);
    assert!(e.back());
    t += 16;
    e.pump(t);
    assert!((e.scroll_position() - 300.0).abs() < 1.0, "p 停在左栏滚到的 300：{}", e.scroll_position());
}

#[test]
fn 先进下一页再拉宽_左栏按新宽重画上一页() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("repaint");
    // 参照：一开始就 900 宽打开 p（一栏 450 宽）
    // 带引擎画的导航栏：q 在 onLoad 里改了标题，左栏重画 p 时不能用 q 的标题
    let opts = r#"{"frameset":"auto","navigationBar":true}"#;
    let mut reference = engine(&root, 900, opts);
    assert_eq!(reference.pixel_size(), (450, H), "一打开就够宽：从一开始就是一栏宽");
    let want = reference.rgba().to_vec();
    drop(reference);

    let mut e = engine(&root, 390, opts);
    e.navigate("/pages/q/q").unwrap();
    e.pump(20);
    assert!(e.resize(900, H, 1.0));
    e.pump(40);
    assert_eq!(e.side_pixel_size(), (450, H));
    assert_eq!(side(&mut e), want, "左栏 = 按一栏宽打开的 p（逐字节）");
    assert_eq!(px(e.rgba(), 450, 225, 320), [0xFF, 0xFF, 0xFF], "活页面仍是 q");
    // 再返回：p 回到左栏、右栏占位
    assert!(e.back());
    e.pump(60);
    assert_eq!(e.rgba(), &want[..], "回到 p：与参照相同");
    assert_eq!(px(&side(&mut e), 450, 225, 320), [0xF7, 0xF7, 0xF7]);
}

fn tap(e: &mut MiniEngine, x: f32, y: f32, t: &mut u64) {
    e.pointer_down(x, y);
    *t += 16;
    e.pump(*t);
    e.pointer_up(x, y);
    for _ in 0..3 {
        *t += 16;
        e.pump(*t);
    }
}

fn js(e: &MiniEngine, code: &str) -> String {
    e.eval(code).unwrap().trim().to_string()
}

#[test]
fn 右栏开着时左栏照样能点_左栏打开的页换掉右栏() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("focus");
    let mut e = engine(&root, 900, r#"{"frameset":"auto"}"#);
    let mut t = 10;
    // 单页：活页面在左栏，点「一」→ 右栏打开 q?id=1
    tap(&mut e, 100.0, 110.0, &mut t);
    assert_eq!(e.page_depth(), 2);
    assert_eq!(frameset_events(&mut e).last().unwrap()["livePane"], "right");
    let q1 = e.rgba().to_vec();

    // 点左栏：左栏换成活页面，右栏那页留着
    assert!(!e.frameset_focus(false), "右栏已经是活页面");
    assert!(e.frameset_focus(true));
    t += 16;
    assert_ne!(e.pump_ex(t) & 64, 0, "另一栏换成右栏那页的图");
    let ev = frameset_events(&mut e);
    assert_eq!((ev.last().unwrap()["livePane"].as_str(), ev.last().unwrap()["sidePage"].as_bool()), (Some("left"), Some(true)));
    assert_eq!(side(&mut e), q1, "右栏显示的是刚才的 q");
    assert_eq!(js(&e, "getCurrentPages().length"), "2", "逻辑层的页面栈不动");
    // 左栏里不跳页的点击：p 自己的 setData，右栏不受影响
    tap(&mut e, 100.0, 40.0, &mut t);
    assert_eq!(js(&e, "getCurrentPages()[0].data.n"), "2", "点在左栏的 inc 上");
    assert_eq!(e.page_depth(), 2);
    assert_eq!(side(&mut e), q1);
    assert_eq!(js(&e, "globalThis.__gone||''"), "");

    // 左栏点「二」：右栏的 q1 关掉（onUnload），q2 进右栏
    tap(&mut e, 100.0, 170.0, &mut t);
    assert_eq!(e.page_depth(), 2);
    assert_eq!(js(&e, "getCurrentPages().length"), "2");
    assert_eq!(js(&e, "getCurrentPages()[1].id"), "2");
    assert_eq!(js(&e, "globalThis.__gone"), "1", "右栏原来那页 onUnload");
    assert_eq!(frameset_events(&mut e).last().unwrap()["livePane"], "right");
    assert_eq!(js(&e, "getCurrentPages()[0].data.n"), "2", "左栏 p 的状态还在");

    // 再点左栏、再窄回单栏：单栏显示栈顶（q2），返回回到 p
    assert!(e.frameset_focus(true));
    e.resize(390, H, 1.0);
    t += 16;
    e.pump(t);
    assert_eq!(e.current_route(), "pages/q/q");
    assert!(e.back());
    t += 16;
    e.pump(t);
    assert_eq!(e.current_route(), "pages/p/p");
    assert_eq!(js(&e, "globalThis.__gone"), "12");
}

#[test]
fn 左栏是活页面时返回关掉的是右栏那页() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("focus-back");
    let mut e = engine(&root, 900, r#"{"frameset":"auto"}"#);
    let mut t = 10;
    tap(&mut e, 100.0, 110.0, &mut t);
    let p_frame = side(&mut e);
    assert!(e.frameset_focus(true));
    t += 16;
    e.pump(t);
    assert!(e.back());
    t += 16;
    e.pump(t);
    assert_eq!((e.page_depth(), e.current_route().as_str()), (1, "pages/p/p"));
    assert_eq!(js(&e, "globalThis.__gone"), "1");
    assert_eq!(frameset_events(&mut e).last().unwrap()["sidePage"], false, "右栏回到占位");
    assert_eq!(e.rgba(), &p_frame[..], "左栏的 p 与对调前一样");
}

#[test]
fn 换栏后紧接着按下也点得中_宿主不等一帧() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("focus-now");
    let mut e = engine(&root, 900, r#"{"frameset":"auto"}"#);
    let mut t = 10;
    let p_frame = e.rgba().to_vec();
    tap(&mut e, 100.0, 110.0, &mut t);
    assert_eq!(e.page_depth(), 2);
    // 宿主（macOS）的做法：按下时先 focus、紧接着 pointer_down，中间没有 pump
    frameset_events(&mut e);
    assert!(e.frameset_focus(true));
    e.pointer_down(100.0, 170.0);
    e.pointer_up(100.0, 170.0);
    t += 16;
    e.pump(t);
    assert_eq!(js(&e, "getCurrentPages()[1].id"), "2", "左栏的「二」点中了，右栏换成 q2");
    // 前后都是「活页面在右栏」，也要报一次：宿主按下时已经把坐标改按左栏算了
    let ev = frameset_events(&mut e);
    assert_eq!(ev.last().map(|v| v["livePane"].clone()), Some("right".into()), "{ev:?}");
    assert_eq!(side(&mut e), p_frame, "左栏还是 p（字段不能空：从前按右栏那页的数据重画）");
    assert_eq!(js(&e, "globalThis.__gone"), "1");
}

fn region_eq(a: &[u8], b: &[u8], w: u32, r: (u32, u32, u32, u32)) -> bool {
    (r.1..r.1 + r.3).all(|y| (r.0..r.0 + r.2).all(|x| px(a, w, x, y) == px(b, w, x, y)))
}

#[test]
fn 胶囊只在窗口右上角_左栏没有() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("capsule");
    let opts = r#"{"frameset":"auto","navigationBar":true,"menuButton":true}"#;
    // 参照：单栏 450 宽、不开胶囊的 p（左栏该是这个样子）
    let mut plain = engine(&root, 450, r#"{"navigationBar":true}"#);
    let p_plain = plain.rgba().to_vec();
    drop(plain);
    let mut narrow = engine(&root, 450, r#"{"navigationBar":true,"menuButton":true}"#);
    let cap = narrow.menu_button_rect().expect("单栏有胶囊");
    drop(narrow);
    let r = (cap.x as u32, cap.y as u32, cap.width as u32, cap.height as u32);

    let mut e = engine(&root, 900, opts);
    let mut t = 10;
    t += 16;
    e.pump(t);
    assert!(e.menu_button_rect().is_none(), "活页面在左栏：没有胶囊（也点不到）");
    assert_eq!(e.rgba(), &p_plain[..], "左栏与不开胶囊时逐字节相同");
    let ev = frameset_events(&mut e);
    assert!(ev.last().unwrap()["sideCapsule"].is_object(), "胶囊画在右栏的占位上：{ev:?}");
    let s = side(&mut e);
    assert!(!region_eq(&s, &vec![0xF7; s.len()], 450, r), "占位图上有胶囊");

    // 右栏打开一页：胶囊跟着右栏的活页面；左栏的 p 仍没有胶囊（引擎画了 64 高的导航栏，「一」在 144~204）
    tap(&mut e, 100.0, 170.0, &mut t);
    assert_eq!(e.page_depth(), 2);
    assert!(e.menu_button_rect().is_some());
    assert!(frameset_events(&mut e).last().unwrap()["sideCapsule"].is_null());
    assert_eq!(side(&mut e), p_plain, "挪到左栏的 p 不带胶囊");
    // 再往下开一页（右栏 → 左栏的 q1 也不带胶囊）
    e.navigate("/pages/q/q?id=3").unwrap();
    for _ in 0..30 {
        t += 16;
        e.pump(t);
    }
    let mut q_plain = engine(&root, 450, r#"{"navigationBar":true}"#);
    q_plain.navigate("/pages/q/q?id=1").unwrap();
    q_plain.pump(20);
    let want = q_plain.rgba().to_vec();
    drop(q_plain);
    assert!(region_eq(&side(&mut e), &want, 450, r), "左栏 q1 的胶囊位置与不开胶囊时相同");
}

#[test]
fn 分栏时右栏里照样有转场_两栏之间挪的那次一换就到() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("route");
    let mut e = engine(&root, 900, r#"{"frameset":"auto","routeAnimation":true}"#);
    let mut t = 10;
    // 单页 → 右栏打开第一页：活页面换栏，没有转场
    tap(&mut e, 100.0, 110.0, &mut t);
    let q1 = e.rgba().to_vec();
    // 右栏里再进一页：从右边滑进来（第一帧还看得到下面的 q1，几百毫秒后落定）
    e.navigate("/pages/q/q?id=3").unwrap();
    t += 16;
    e.pump(t);
    let first = e.rgba().to_vec();
    for _ in 0..40 {
        t += 16;
        e.pump(t);
    }
    let settled = e.rgba().to_vec();
    assert_ne!(first, settled, "进页有转场");
    assert_eq!(side(&mut e), q1, "左栏直接换成 q1");
    // 返回：右栏的 q3 滑出去
    assert!(e.back());
    t += 16;
    e.pump(t);
    let first = e.rgba().to_vec();
    for _ in 0..40 {
        t += 16;
        e.pump(t);
    }
    assert_ne!(first, e.rgba().to_vec(), "返回有转场");
    assert_eq!(e.rgba(), &q1[..], "落定后右栏是 q1");
}

#[test]
fn 左栏的首页没有返回箭头_浮层不冻进留图() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("arrow-toast");
    let opts = r#"{"frameset":"auto","navigationBar":true,"menuButton":true}"#;
    let mut plain = engine(&root, 450, r#"{"navigationBar":true}"#);
    let p_plain = plain.rgba().to_vec();
    drop(plain);
    let mut e = engine(&root, 900, opts);
    let mut t = 10;
    tap(&mut e, 100.0, 170.0, &mut t);
    assert_eq!(e.page_depth(), 2);
    // 点左栏：左栏的 p 换成活页面 —— 它是首页，导航栏上不该冒出返回箭头（也没有胶囊）
    assert!(e.frameset_focus(true));
    t += 16;
    e.pump(t);
    assert_eq!(e.rgba(), &p_plain[..], "左栏活页面与单栏首页逐字节相同");
    // 左栏弹一个 Toast，再在左栏打开另一页：挪回左栏的 p 的留图里不能有 Toast
    e.eval("wx.showToast({title:'已复制',duration:100000})").unwrap();
    for _ in 0..3 {
        t += 16;
        e.pump(t);
    }
    assert_ne!(e.rgba(), &p_plain[..], "Toast 画在活页面上");
    tap(&mut e, 100.0, 230.0, &mut t);
    assert_eq!(js(&e, "getCurrentPages()[1].id"), "2");
    assert_eq!(side(&mut e), p_plain, "左栏 p 的留图里没有 Toast");
}

#[test]
fn 新页_onload_里改的标题第一帧就画上_不闪() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("title");
    for opts in [r#"{"frameset":"auto","navigationBar":true}"#, r#"{"navigationBar":true}"#] {
        let mut e = engine(&root, 900, opts);
        let mut t = 10;
        e.navigate("/pages/q/q?id=7").unwrap();
        let (w, _) = e.pixel_size();
        let band = (w * 64 * 4) as usize;
        let mut bars = vec![];
        for _ in 0..10 {
            t += 16;
            if e.pump(t) {
                bars.push(e.rgba()[..band].to_vec());
            }
        }
        assert!(!bars.is_empty());
        assert!(bars.windows(2).all(|p| p[0] == p[1]), "{opts}: 导航栏在第一帧之后又变了（先画 json 标题、再换成 Q7）");
    }
}

/// 像真手指那样按住 ~100ms 再抬（点击态在 hover-start-time 之后才亮）
fn press(e: &mut MiniEngine, x: f32, y: f32, t: &mut u64) {
    e.pointer_down(x, y);
    // 点击态的出现时刻按墙上时钟走（hover-start-time 50ms）
    for _ in 0..6 {
        std::thread::sleep(std::time::Duration::from_millis(16));
        *t += 16;
        e.pump(*t);
    }
    e.pointer_up(x, y);
    for _ in 0..30 {
        *t += 16;
        e.pump(*t);
    }
}

#[test]
fn 点会话打开聊天_那一行的点击态不冻进左栏() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("press");
    let mut e = engine(&root, 900, r#"{"frameset":"auto"}"#);
    let mut t = 10;
    let p_plain = e.rgba().to_vec();
    // 先确认点击态真的会亮（不然这条测不到东西）
    e.pointer_down(100.0, 110.0);
    for _ in 0..6 {
        std::thread::sleep(std::time::Duration::from_millis(16));
        t += 16;
        e.pump(t);
    }
    assert_eq!(px(e.rgba(), 450, 100, 110), [0x88, 0x88, 0x88], "按住时那一行是点击态");
    e.pointer_up(100.0, 110.0);
    for _ in 0..30 {
        t += 16;
        e.pump(t);
    }
    assert_eq!(e.page_depth(), 2);
    assert_eq!(side(&mut e), p_plain, "左栏的 p 没有那一行的点击态");
    // 从左栏再点另一行：换栏 → 按住 → 抬起开新页，左栏留图同样干净
    assert!(e.frameset_focus(true));
    press(&mut e, 100.0, 170.0, &mut t);
    assert_eq!(js(&e, "getCurrentPages()[1].id"), "2");
    assert_eq!(side(&mut e), p_plain, "换栏之后点的那一行也不冻进去");
}
