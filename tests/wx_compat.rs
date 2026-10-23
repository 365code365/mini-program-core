//! 对照微信补齐的那批 API 与内置组件的端到端用例（`.agents/tasks/wx-gap-2026-10-23/gap.md`）。
//! 每个用例现场写一个小程序，用 `MiniEngine` 跑起来，按可判定的判据断言（值、类型、事件顺序、栈深、像素）。
//!
//! 独立的集成测试（同 `tests/embed_host.rs` 的理由）：小程序根目录与数据目录是进程级的。
use mini_render::compositor::HostEnv;
use mini_render::host::{EngineOptions, MiniEngine};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static SERIAL: Mutex<()> = Mutex::new(());

/// 两页（p 是 tab 页、q 普通页）的小程序；`p_wxml` / `p_js` 是首页，`extra` 是额外文件
fn fixture(name: &str, p_wxml: &str, p_js: &str, extra: &[(&str, &str)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("wx-compat").join(name);
    std::fs::remove_dir_all(&root).ok();
    let app_json = r#"{"pages":["pages/p/p","pages/q/q","pages/t/t"],"tabBar":{"list":[{"pagePath":"pages/p/p","text":"P"},{"pagePath":"pages/t/t","text":"T"}]}}"#;
    let files: Vec<(&str, &str)> = vec![
        ("app.json", app_json),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", p_wxml),
        ("pages/p/p.wxss", ""),
        ("pages/p/p.js", p_js),
        ("pages/p/p.json", "{}"),
        ("pages/q/q.wxml", "<view>q</view>"),
        ("pages/q/q.wxss", ""),
        (
            "pages/q/q.js",
            "Page({ onLoad(q){ getApp().log.push('q.onLoad:' + JSON.stringify(q)); \
               var ch = this.getOpenerEventChannel(); \
               ch.on('fromOpener', (d) => { getApp().log.push('q got ' + d.n); ch.emit('fromOpened', { n: d.n + 1 }); }); } });",
        ),
        ("pages/q/q.json", "{}"),
        ("pages/t/t.wxml", "<view>t</view>"),
        ("pages/t/t.wxss", ""),
        ("pages/t/t.js", "Page({});"),
        ("pages/t/t.json", "{}"),
    ];
    for (rel, text) in files.iter().chain(extra.iter()) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    // app.js 里放一个日志数组，页面们往里记
    std::fs::write(root.join("app.js"), "App({ log: [] });").unwrap();
    root
}

fn engine(root: &PathBuf, opts: EngineOptions) -> MiniEngine {
    let data = root.join("_data");
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 1.0, HostEnv::default(), opts)
        .expect("创建引擎");
    e.launch(None).expect("打开首页");
    e.pump(1);
    e
}

fn js(e: &MiniEngine, code: &str) -> String {
    e.eval(code).unwrap_or_default()
}

/// 按 16ms 一帧 pump，直到 `cond` 成立或超时
fn pump_until(e: &mut MiniEngine, ms: u64, mut cond: impl FnMut(&MiniEngine) -> bool) -> bool {
    let start = Instant::now();
    let mut t = 100u64;
    while start.elapsed() < Duration::from_millis(ms) {
        e.pump(t);
        if cond(e) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(8));
        t += 16;
    }
    false
}

fn log(e: &MiniEngine) -> Vec<String> {
    serde_json::from_str(&js(e, "JSON.stringify(getApp().log)")).unwrap_or_default()
}

#[test]
fn 路由_校验_事件顺序_success_时机_eventchannel_双向() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "route",
        "<view>p</view>",
        r#"Page({ onLoad(){
            const L = getApp().log;
            ['BeforeAppRoute','AppRoute','AppRouteDone','BeforePageLoad','AfterPageLoad','BeforePageUnload','AfterPageUnload']
              .forEach(n => wx['on' + n](r => L.push(n + ':' + r.openType + ':' + r.path)));
          },
          go(){
            const L = getApp().log;
            wx.navigateTo({ url: '/pages/nope/nope', fail: r => L.push('nope ' + r.errMsg) });
            wx.navigateTo({ url: '/pages/t/t', fail: r => L.push('tab ' + r.errMsg) });
            wx.switchTab({ url: '/pages/q/q', fail: r => L.push('switch ' + r.errMsg) });
            wx.navigateTo({ url: '../q/q?id=7', events: { fromOpened: d => L.push('p got ' + d.n) },
              success: r => { L.push('success ' + r.errMsg); r.eventChannel.emit('fromOpener', { n: 1 }); } });
            L.push('after call');
          } });"#,
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    js(&e, "getApp().log.length = 0; getCurrentPages()[0].go()");
    assert!(pump_until(&mut e, 2000, |e| e.page_depth() == 2 && log(e).iter().any(|l| l == "p got 2")), "{:?}", log(&e));
    // fail 回调在下一轮事件循环：再走几帧
    pump_until(&mut e, 150, |_| false);
    let l = log(&e);
    let idx = |s: &str| l.iter().position(|x| x.starts_with(s)).unwrap_or_else(|| panic!("缺 {s}：{l:?}"));
    // 校验失败的三种：页面不存在、navigateTo 到 tab 页、switchTab 到非 tab 页（都在下一轮事件循环 fail）
    assert!(l.iter().any(|x| x == r#"nope navigateTo:fail page "pages/nope/nope" is not found"#), "{l:?}");
    assert!(l.iter().any(|x| x == "tab navigateTo:fail can not navigateTo a tabbar page"), "{l:?}");
    assert!(l.iter().any(|x| x == "switch switchTab:fail can not switch to no-tabBar page"), "{l:?}");
    // success 在新页面 onLoad 之后（不再同步回）；opener 在 success 里 emit，新页面收得到，回发也收得到
    assert!(idx("after call") < idx("success navigateTo:ok"));
    assert!(idx("q.onLoad") < idx("success navigateTo:ok"));
    assert!(l.contains(&r#"q.onLoad:{"id":"7"}"#.to_string()), "相对路径按当前页解析、query 原样：{l:?}");
    // 事件顺序：BeforeAppRoute → BeforePageLoad → (onLoad) → AfterPageLoad → AppRoute → AppRouteDone
    let order = ["BeforeAppRoute:navigateTo:pages/q/q", "BeforePageLoad:navigateTo:pages/q/q", "AfterPageLoad:navigateTo:pages/q/q", "AppRoute:navigateTo:pages/q/q"];
    let pos: Vec<usize> = order.iter().map(|s| idx(s)).collect();
    assert!(pos.windows(2).all(|w| w[0] < w[1]), "顺序不对：{l:?}");
    // 宿主发起的返回：卸载事件 + navigateBack 的路由事件
    js(&e, "getApp().log.length = 0");
    assert!(e.back());
    assert!(pump_until(&mut e, 1000, |e| log(e).iter().any(|x| x.starts_with("AppRouteDone:navigateBack"))), "{:?}", log(&e));
    let l = log(&e);
    assert!(l.iter().any(|x| x.starts_with("BeforePageUnload")) && l.iter().any(|x| x.starts_with("AfterPageUnload")), "{l:?}");
    assert!(l.iter().any(|x| x == "AppRoute:navigateBack:pages/p/p"), "{l:?}");
}

#[test]
fn 返回询问与_page_container_接住返回() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "back",
        r#"<page-container show="{{show}}" bind:afterleave="onLeave" bind:afterenter="onEnter"><view style="height:200px;background-color:#ff0000">弹层</view></page-container>"#,
        r#"Page({ data: { show: false },
            onEnter(){ getApp().log.push('afterenter'); },
            onLeave(){ getApp().log.push('afterleave'); this.setData({ show: false }); } });"#,
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    // 弹层：show → afterenter；返回键先关它（栈深不变、返回 true），afterleave 回来
    js(&e, "getCurrentPages()[0].setData({ show: true })");
    assert!(pump_until(&mut e, 2000, |e| log(e).contains(&"afterenter".to_string())), "{:?}", log(&e));
    // 弹层在屏幕底部：底部一行是红色
    let red_bottom = |e: &mut MiniEngine| {
        let (w, h) = e.pixel_size();
        let px = e.rgba();
        let i = (((h - 20) * w + w / 2) * 4) as usize;
        px[i] > 200 && px[i + 1] < 60
    };
    assert!(red_bottom(&mut e), "弹层画在底部");
    assert!(e.back(), "接住了返回：宿主别关小程序");
    assert_eq!(e.page_depth(), 1);
    assert!(pump_until(&mut e, 2000, |e| log(e).contains(&"afterleave".to_string())), "{:?}", log(&e));
    pump_until(&mut e, 200, |_| false);
    assert!(!red_bottom(&mut e), "关掉之后底部不再是弹层");

    // 返回询问：进 q 页、开询问，返回键弹 Modal；取消留下，确定才返回
    js(&e, "wx.navigateTo({ url: '/pages/q/q' })");
    assert!(pump_until(&mut e, 1000, |e| e.page_depth() == 2));
    js(&e, "wx.enableAlertBeforeUnload({ message: '确定离开吗' })");
    assert!(e.back());
    e.pump(5000);
    assert_eq!(e.page_depth(), 2, "弹询问时不退栈");
    assert_eq!(js(&e, "__modalConfig ? __modalConfig.content : ''"), "确定离开吗");
    js(&e, "__handleModalResult(false)");
    e.pump(5016);
    assert_eq!(e.page_depth(), 2, "取消就留下");
    assert!(e.back());
    js(&e, "__handleModalResult(true)");
    assert!(pump_until(&mut e, 1000, |e| e.page_depth() == 1), "确定就返回");
}

#[test]
fn 宿主推事件_宿主能力与对象方法走_hostcall() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "host",
        "<view>p</view>",
        r#"Page({ data: { acc: '', bt: '', map: '' }, onLoad(){
            wx.onAccelerometerChange(r => this.setData({ acc: r.x + ',' + r.y + ',' + r.z }));
            wx.startBluetoothDevicesDiscovery({ fail: r => this.setData({ bt: r.errMsg }) });
            wx.onNetworkStatusChange(r => getApp().log.push('net ' + r.networkType + ' ' + r.isConnected));
          } });"#,
        &[],
    );
    let opts = EngineOptions { host_apis: vec!["MapContext.moveToLocation".into()], ..Default::default() };
    let mut e = engine(&root, opts);
    assert_eq!(e.dispatch_event("accelerometerChange", r#"{"x":0.1,"y":-0.5,"z":9.8}"#), 1, "一个监听者");
    assert_eq!(js(&e, "getCurrentPages()[0].data.acc"), "0.1,-0.5,9.8");
    assert_eq!(e.dispatch_event("onCompassChange", "{}"), 0, "没人听回 0");
    e.dispatch_event("networkStatusChange", r#"{"networkType":"none","isConnected":false}"#);
    assert!(log(&e).contains(&"net none false".to_string()), "{:?}", log(&e));
    assert!(pump_until(&mut e, 500, |e| js(e, "getCurrentPages()[0].data.bt").contains("startBluetoothDevicesDiscovery:fail")));
    assert_eq!(js(&e, "String(wx.canIUse('startBluetoothDevicesDiscovery'))"), "false");
    // 对象方法：点名了的发 hostCall（带 contextId），没点名的立刻 fail
    while e.poll_event().is_some() {}
    js(&e, "var m = wx.createMapContext('map1'); m.moveToLocation({ latitude: 1, longitude: 2 }); \
            m.getCenterLocation({ fail: r => getApp().log.push(r.errMsg) })");
    e.pump(2000);
    let evs: Vec<Value> = std::iter::from_fn(|| e.poll_event()).map(|s| serde_json::from_str(&s).unwrap()).collect();
    let call = evs.iter().find(|v| v["type"] == "hostCall").expect("hostCall");
    assert_eq!(call["api"], "MapContext.moveToLocation");
    assert_eq!(call["args"], json!({ "latitude": 1, "longitude": 2, "contextId": "map1" }));
    assert!(pump_until(&mut e, 500, |e| log(e).iter().any(|x| x.starts_with("getCenterLocation:fail"))), "{:?}", log(&e));
}

#[test]
fn worker_双向消息_udp_回环_随机数与_buffer_url() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "worker",
        r#"<image wx:if="{{u}}" src="{{u}}" style="width:50px;height:50px"></image>"#,
        r#"Page({ data: { u: '' }, onLoad(){
            const L = getApp().log;
            const w = wx.createWorker('workers/w.js');
            w.onMessage(m => L.push('main got ' + m.v));
            w.postMessage({ v: 20 });
            const s = wx.createUDPSocket();
            const port = s.bind();
            L.push('port ' + (port > 0));
            s.onMessage(r => L.push('udp ' + String.fromCharCode.apply(null, new Uint8Array(r.message)) + ' ' + r.remoteInfo.size));
            s.send({ address: '127.0.0.1', port: port, message: 'hey' });
            wx.getRandomValues({ length: 16, success: r => L.push('rand ' + r.randomValues.byteLength) });
            // 8×8 的红色 PNG（base64）→ createBufferURL → <image> 直接画
            const png = 'iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAAEklEQVR4nGP4z8CAFWEXHbQSACj/P8Fu7N9hAAAAAElFTkSuQmCC';
            this.setData({ u: wx.createBufferURL(wx.base64ToArrayBuffer(png)) });
          } });"#,
        &[("workers/w.js", "worker.onMessage(function (m) { worker.postMessage({ v: m.v * 2 + 2 }); });")],
    );
    let mut e = engine(&root, EngineOptions::default());
    assert!(
        pump_until(&mut e, 3000, |e| {
            let l = log(e);
            l.contains(&"main got 42".to_string()) && l.iter().any(|x| x.starts_with("udp hey 3")) && l.contains(&"rand 16".to_string())
        }),
        "{:?}",
        log(&e)
    );
    assert!(log(&e).contains(&"port true".to_string()));
    let u = js(&e, "getCurrentPages()[0].data.u");
    assert!(u.starts_with("wxfile://tmp/") && u.ends_with(".png"), "{u}");
    pump_until(&mut e, 300, |_| false);
    let (w, _) = e.pixel_size();
    let px = e.rgba();
    let i = ((25 * w + 25) * 4) as usize;
    assert_eq!(&px[i..i + 3], &[255, 0, 0], "createBufferURL 的图画出来了");
    js(&e, &format!("wx.revokeBufferURL('{u}')"));
}

#[test]
fn match_media_与手势组件() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "mm",
        r#"<match-media min-width="300"><view id="wide" style="height:40px;background-color:#00ff00"></view></match-media>
           <match-media min-width="800"><view id="huge" style="height:40px;background-color:#0000ff"></view></match-media>
           <pan-gesture-handler on-gesture-event="onPan"><view style="height:200px;background-color:#eeeeee"></view></pan-gesture-handler>"#,
        r#"Page({ onPan(e){ getApp().log.push('pan ' + e.state + ' ' + Math.round(e.translationX)); } });"#,
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 300, |_| false);
    let (w, _) = e.pixel_size();
    let at = |e: &mut MiniEngine, y: u32| {
        let px = e.rgba();
        let i = ((y * w + 10) * 4) as usize;
        [px[i], px[i + 1], px[i + 2]]
    };
    assert_eq!(at(&mut e, 20), [0, 255, 0], "375 宽满足 min-width=300");
    assert_ne!(at(&mut e, 60), [0, 0, 255], "不满足 min-width=800 的不渲染");
    // 拖：在手势区里从 (50,100) 拖到 (150,100)
    e.pointer_down(50.0, 100.0);
    e.pump(5000);
    for (i, x) in [60.0, 80.0, 120.0, 150.0].iter().enumerate() {
        e.pointer_move(*x, 100.0);
        e.pump(5016 + i as u64 * 16);
    }
    e.pointer_up(150.0, 100.0);
    e.pump(5200);
    let l = log(&e);
    assert!(l.first().map(|s| s.starts_with("pan 1")).unwrap_or(false), "先 BEGIN：{l:?}");
    assert!(l.iter().any(|s| s.starts_with("pan 2")), "再 ACTIVE：{l:?}");
    assert_eq!(l.last().map(|s| s.as_str()), Some("pan 3 100"), "最后 END，累计位移 100：{l:?}");
}

#[test]
fn 旧版画布_裁剪与虚线() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "canvas",
        r#"<canvas canvas-id="c" style="width:100px;height:100px"></canvas>"#,
        r#"Page({ onReady(){
            const c = wx.createCanvasContext('c');
            c.save(); c.beginPath(); c.rect(0, 0, 50, 100); c.clip();
            c.setFillStyle('#ff0000'); c.fillRect(0, 0, 100, 50); c.restore();
            c.setStrokeStyle('#0000ff'); c.setLineWidth(4); c.setLineDash([10, 10]);
            c.beginPath(); c.moveTo(0, 80); c.lineTo(100, 80); c.stroke();
            c.draw();
            getApp().log.push('w ' + c.measureText('abc').width);
          } });"#,
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 400, |_| false);
    let (w, _) = e.pixel_size();
    let at = |e: &mut MiniEngine, x: u32, y: u32| {
        let px = e.rgba();
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2]]
    };
    assert_eq!(at(&mut e, 25, 25), [255, 0, 0], "裁剪区里画上了");
    assert_eq!(at(&mut e, 75, 25), [255, 255, 255], "裁剪区外没画（restore 前的 fillRect 被裁掉）");
    assert_eq!(at(&mut e, 5, 80), [0, 0, 255], "虚线第一段实");
    assert_eq!(at(&mut e, 15, 80), [255, 255, 255], "虚线第一段空");
    let l = log(&e);
    let width: f32 = l.iter().find_map(|s| s.strip_prefix("w ")).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    assert!(width > 5.0 && width < 40.0, "measureText 按字号量出宽度：{l:?}");
}

#[test]
fn movable_view_拖动夹在区域里_change_事件_setdata_移动() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    // 区域 300×200，视图 50×50 → 可移动范围 x∈[0,250]、y∈[0,150]
    let root = fixture(
        "movable",
        r#"<movable-area style="width:300px;height:200px;background-color:#eeeeee">
             <movable-view direction="all" x="{{x}}" y="{{y}}" animation="{{false}}" bindchange="onChange"
               style="width:50px;height:50px;background-color:#ff0000"></movable-view>
           </movable-area>"#,
        r#"Page({ data: { x: 20, y: 10 },
             onChange(e){ getApp().log.push(e.detail.source + ' ' + Math.round(e.detail.x) + ',' + Math.round(e.detail.y)); } });"#,
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 300, |_| false);
    let (w, _) = e.pixel_size();
    let at = |e: &mut MiniEngine, x: u32, y: u32| {
        let px = e.rgba();
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2]]
    };
    assert_eq!(at(&mut e, 25, 15), [255, 0, 0], "初始 x=20 y=10");
    assert_eq!(at(&mut e, 10, 5), [0xee, 0xee, 0xee], "区域底色");
    // 从视图里 (40,30) 拖到 (340,330)：超出范围，夹到右下角 (250,150)
    e.pointer_down(40.0, 30.0);
    e.pump(5000);
    for (i, p) in [(60.0, 50.0), (120.0, 110.0), (240.0, 230.0), (340.0, 330.0)].iter().enumerate() {
        e.pointer_move(p.0, p.1);
        e.pump(5016 + i as u64 * 16);
    }
    e.pointer_up(340.0, 330.0);
    pump_until(&mut e, 200, |_| false);
    let l = log(&e);
    assert_eq!(l.last().map(|s| s.as_str()), Some("touch 250,150"), "拖动发 change(source=touch)，夹在范围里：{l:?}");
    assert_eq!(at(&mut e, 255, 155), [255, 0, 0], "画在右下角");
    assert_eq!(at(&mut e, 25, 15), [0xee, 0xee, 0xee], "原位置空了");
    // 使用方 setData 改 x / y：移过去，source 为空串
    js(&e, "getCurrentPages()[0].setData({ x: 100, y: 50 })");
    pump_until(&mut e, 200, |_| false);
    assert_eq!(log(&e).last().map(|s| s.as_str()), Some(" 100,50"));
    assert_eq!(at(&mut e, 105, 55), [255, 0, 0]);
}

#[test]
fn 触摸序列锁定在_touchstart_的节点上() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    // A 在上、B 在下：从 A 按下拖进 B 再抬起，move / end 归 A（微信、DOM 一致），B 一个都收不到
    let root = fixture(
        "capture",
        r#"<view id="a" style="height:100px" bindtouchmove="onA" bindtouchend="onA"></view>
           <view id="b" style="height:100px" bindtouchmove="onB" bindtouchend="onB"></view>"#,
        r#"Page({
             onA(e){ getApp().log.push('A ' + e.type + ' ' + e.target.id + ' ' + Math.round(e.changedTouches[0].clientY)); },
             onB(e){ getApp().log.push('B ' + e.type); } });"#,
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    e.pointer_down(100.0, 50.0);
    e.pump(5000);
    for (i, y) in [60.0, 120.0, 150.0].iter().enumerate() {
        e.pointer_move(100.0, *y);
        e.pump(5016 + i as u64 * 16);
    }
    e.pointer_up(100.0, 150.0);
    e.pump(5100);
    let l = log(&e);
    assert!(l.iter().all(|s| s.starts_with("A ")), "B 不该收到：{l:?}");
    assert!(l.contains(&"A touchmove a 150".to_string()), "手指在 B 上时 move 仍归 A，坐标是手指当前位置：{l:?}");
    assert_eq!(l.last().map(|s| s.as_str()), Some("A touchend a 150"), "{l:?}");
}

#[test]
fn this_animate_与_worklet_写节点样式_webaudio_出_ended() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture(
        "animate",
        r#"<view id="box" style="width:50px;height:50px;background-color:#ff0000"></view>
           <view id="w" style="width:10px;height:20px;background-color:#00ff00"></view>"#,
        r#"Page({ onReady(){
            const L = getApp().log, wk = wx.worklet;
            this.animate('#box', [{ backgroundColor: '#ff0000' }, { backgroundColor: '#0000ff' }], 100, () => L.push('animate done'));
            const sv = wk.shared(10), dbl = wk.derived(() => sv.value * 2);
            this.applyAnimatedStyle('#w', () => ({ width: sv.value }));
            sv.value = wk.timing(100, { duration: 100 }, (f) => L.push('timing ' + f + ' ' + dbl.value));
            const sp = wk.shared(0);
            sp.value = wk.spring(1, { stiffness: 400, damping: 40 }, (f) => L.push('spring ' + f + ' ' + sp.value));
            const seq = wk.shared(0);
            seq.value = wk.sequence(wk.timing(5, { duration: 20 }), wk.delay(20, wk.timing(7, { duration: 20 })));
            setTimeout(() => L.push('seq ' + seq.value), 400);
            L.push('caniuse ' + [wx.canIUse('movable-view'), wx.canIUse('grid-view'), wx.canIUse('worklet'), wx.canIUse('no-such-tag')].join(','));
            const ctx = wx.createWebAudioContext(), buf = ctx.createBuffer(1, 4410, 44100), d = buf.getChannelData(0);
            for (let i = 0; i < d.length; i++) d[i] = Math.sin(i / 10);
            const src = ctx.createBufferSource(), g = ctx.createGain();
            g.gain.value = 0.5; src.buffer = buf; src.connect(g); g.connect(ctx.destination);
            src.onended = () => L.push('ended ' + ctx.state + ' ' + (ctx.currentTime > 0.05));
            src.start();
            // 16 位单声道 8kHz、800 个样本的 WAV：decodeAudioData 出 0.1 秒
            const n = 800, ab = new ArrayBuffer(44 + n * 2), dv = new DataView(ab), w = (o, s) => { for (let i = 0; i < 4; i++) dv.setUint8(o + i, s.charCodeAt(i)); };
            w(0, 'RIFF'); dv.setUint32(4, 36 + n * 2, true); w(8, 'WAVE'); w(12, 'fmt '); dv.setUint32(16, 16, true); dv.setUint16(20, 1, true);
            dv.setUint16(22, 1, true); dv.setUint32(24, 8000, true); dv.setUint32(28, 16000, true); dv.setUint16(32, 2, true); dv.setUint16(34, 16, true);
            w(36, 'data'); dv.setUint32(40, n * 2, true);
            ctx.decodeAudioData(ab, (b) => L.push('decoded ' + b.sampleRate + ' ' + b.numberOfChannels + ' ' + b.duration.toFixed(2)), (e) => L.push('decode fail ' + e.errMsg));
          } });"#,
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    let done = |e: &MiniEngine| {
        let l = log(e);
        ["animate done", "timing", "spring", "seq", "ended", "decoded"].iter().all(|k| l.iter().any(|s| s.starts_with(k)))
    };
    assert!(pump_until(&mut e, 3000, done), "全部回调到齐：{:?}", log(&e));
    pump_until(&mut e, 100, |_| false);
    let l = log(&e);
    assert!(l.contains(&"caniuse true,true,true,false".to_string()), "{l:?}");
    assert!(l.contains(&"timing true 200".to_string()), "timing 走完，派生值跟着变：{l:?}");
    assert!(l.contains(&"spring true 1".to_string()), "spring 落定在终点：{l:?}");
    assert!(l.contains(&"seq 7".to_string()), "sequence + delay 依次走完：{l:?}");
    assert!(l.contains(&"ended running true".to_string()), "{l:?}");
    assert!(l.contains(&"decoded 8000 1 0.10".to_string()), "{l:?}");
    let (w, _) = e.pixel_size();
    let at = |e: &mut MiniEngine, x: u32, y: u32| {
        let px = e.rgba();
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2]]
    };
    assert_eq!(at(&mut e, 25, 25), [0, 0, 255], "animate 结束后保留最终样式");
    assert_eq!(at(&mut e, 95, 60), [0, 255, 0], "applyAnimatedStyle 把宽度跟着共享值改到 100");
    assert_ne!(at(&mut e, 105, 60), [0, 255, 0], "宽度停在 100，不再往外画");
    js(&e, "getCurrentPages()[0].clearAnimation('#box')");
    pump_until(&mut e, 150, |_| false);
    assert_eq!(at(&mut e, 25, 25), [255, 0, 0], "clearAnimation 清掉之后回到原样式");
}
