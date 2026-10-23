//! Flutter 内置示例里「对照微信补齐」的三页（`pages/skyline`、`pages/builtin`、`pages/newapi`）端到端跑一遍：
//! 走 `MiniEngine`（SDK 链路）、按钮用真实指针点（`pointer_down/up`，触摸状态机产出 tap），拖动用逐步 `pointer_move`。
//! 判据是页面自己写在 data 里的结果文字、SelectorQuery 量出的位置与像素 —— 和人在手机上看到的是同一份。
//!
//! 按钮的定位：每页的按钮都在 `.btns` 里，`.btns > view` 按文档顺序编号（注释里写着第几个是哪个）。
use mini_render::host::MiniEngine;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static SERIAL: Mutex<()> = Mutex::new(());

struct E {
    e: MiniEngine,
    t: u64,
}

impl E {
    fn open(route: &str, slot: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk/flutter/example/assets/demo");
        let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("demo-wx-gap").join(slot);
        std::fs::remove_dir_all(&data).ok();
        let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
        e.launch(Some(route)).expect("打开页面");
        let mut me = E { e, t: 1000 };
        me.wait(150);
        me
    }
    fn frames(&mut self, n: u32) {
        for _ in 0..n {
            self.t += 16;
            self.e.pump(self.t);
        }
    }
    /// 按墙上时钟等（页面里的 setTimeout、动画、Worker 消息都按真实时间走）
    fn wait(&mut self, ms: u64) {
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(ms) {
            self.frames(1);
            std::thread::sleep(Duration::from_millis(8));
        }
    }
    fn wait_until(&mut self, ms: u64, mut cond: impl FnMut(&Self) -> bool) -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(ms) {
            self.frames(1);
            if cond(self) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(8));
        }
        false
    }
    fn text(&self, code: &str) -> String {
        let s = self.e.eval(&format!("String({code})")).unwrap_or_default();
        serde_json::from_str::<String>(&s).unwrap_or(s)
    }
    /// 当前页 data 里的字段
    fn data(&self, k: &str) -> String {
        self.text(&format!("getCurrentPages().slice(-1)[0].data.{k}"))
    }
    fn route(&self) -> String {
        self.text("getCurrentPages().slice(-1)[0].route")
    }
    /// SelectorQuery 量出的视口矩形 [left, top, width, height]
    fn rects(&mut self, sel: &str) -> Vec<[f32; 4]> {
        self.e
            .eval(&format!(
                "globalThis.__q = null; wx.createSelectorQuery().selectAll({sel:?}).boundingClientRect()\
                 .exec(r => globalThis.__q = JSON.stringify((r[0] || []).map(x => [x.left, x.top, x.width, x.height])))"
            ))
            .ok();
        for _ in 0..30 {
            self.frames(1);
            if self.text("globalThis.__q") != "null" {
                break;
            }
        }
        serde_json::from_str(&self.text("globalThis.__q")).unwrap_or_default()
    }
    /// 第 i 个命中的节点滚进视口（上下各留一截）后的矩形
    fn visible(&mut self, sel: &str, i: usize) -> [f32; 4] {
        let r = *self.rects(sel).get(i).unwrap_or_else(|| panic!("{sel} 第 {i} 个不存在"));
        if r[1] >= 80.0 && r[1] + r[3] <= 600.0 {
            return r;
        }
        let top = (self.e.scroll_position() + r[1] - 200.0).max(0.0);
        self.e.eval(&format!("wx.pageScrollTo({{ scrollTop: {top}, duration: 0 }})")).ok();
        self.wait(120);
        self.rects(sel)[i]
    }
    fn tap_at(&mut self, x: f32, y: f32) {
        self.e.pointer_down(x, y);
        self.frames(1);
        self.e.pointer_up(x, y);
        self.frames(4);
    }
    fn tap(&mut self, sel: &str, i: usize) {
        let r = self.visible(sel, i);
        self.tap_at(r[0] + r[2] / 2.0, r[1] + r[3] / 2.0);
    }
    fn drag(&mut self, from: (f32, f32), to: (f32, f32), steps: u32) {
        self.e.pointer_down(from.0, from.1);
        self.frames(1);
        for i in 1..=steps {
            let k = i as f32 / steps as f32;
            self.e.pointer_move(from.0 + (to.0 - from.0) * k, from.1 + (to.1 - from.1) * k);
            self.frames(1);
            std::thread::sleep(Duration::from_millis(16));
        }
        self.e.pointer_up(to.0, to.1);
        self.frames(2);
    }
    /// 视口坐标（逻辑像素）处的颜色
    fn px(&mut self, x: f32, y: f32) -> [u8; 3] {
        let (w, _) = self.e.pixel_size();
        let (x, y) = ((x * 2.0) as u32, (y * 2.0) as u32);
        let p = self.e.rgba();
        let i = ((y * w + x) * 4) as usize;
        [p[i], p[i + 1], p[i + 2]]
    }
}

/// skyline 页 `.btns > view`：0 masonry、1 aligned、2 列数、3 移到中间、4 回左上角、5 scrollTo 0.9、6 scrollTo 0.2、
/// 7 this.animate、8 worklet.spring、9 clearAnimation
#[test]
fn skyline_页_网格_movable_手势_面板_动画() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut e = E::open("pages/skyline/skyline", "skyline");
    const B: &str = ".btns > view";

    // grid-view：瀑布流里 #3 在 #1 正下方（80 + 行距 8）；切成对齐网格后 #3 在第二行、从两项里高的那个（140）下面开始
    let c = e.rects(".cell");
    assert_eq!(c.len(), 9);
    assert!((c[2][1] - (c[0][1] + 88.0)).abs() < 0.6 && (c[2][0] - c[0][0]).abs() < 0.6, "masonry：{c:?}");
    e.tap(B, 1);
    let c = e.rects(".cell");
    assert!((c[2][1] - (c[0][1] + 148.0)).abs() < 0.6, "aligned 第二行从 140 + 8 开始：{c:?}");
    e.tap(B, 2);
    let c = e.rects(".cell");
    assert!((c[2][1] - c[0][1]).abs() < 0.6 && c[2][0] > c[1][0], "3 列：#3 回到第一行：{c:?}");

    // movable-view：从圆心往右下拖 300：夹在区域里（out-of-bounds 松手回弹到边上），change 的 source 是 touch
    let area = e.visible("#area", 0);
    let mv = e.rects(".mv")[0];
    let (cx, cy) = (mv[0] + 32.0, mv[1] + 32.0);
    e.drag((cx, cy), (cx + 300.0, cy + 300.0), 10);
    let want = format!("圆 x={} y={}", (area[2] - 64.0).round(), (area[3] - 64.0).round());
    assert!(e.wait_until(1500, |e| e.data("moveText").starts_with(&want)), "回弹到右下角 {want}：{}", e.data("moveText"));
    let mv = e.rects(".mv")[0];
    assert!((mv[0] - (area[0] + area[2] - 64.0)).abs() < 1.0 && (mv[1] - (area[1] + area[3] - 64.0)).abs() < 1.0, "{mv:?} {area:?}");
    // 「移到中间」：setData 改 x / y，change 的 source 是空串
    e.tap(B, 3);
    let want = format!("圆 x={} y={}", ((area[2] - 64.0) / 2.0).round(), ((area[3] - 64.0) / 2.0).round());
    assert!(e.wait_until(1500, |e| e.data("moveText") == format!("{want}  source=''")), "{}", e.data("moveText"));

    // pan 手势：在灰色区域里横拖 100，松手报位移；圆点 spring 回到原位
    let zone = e.visible(".pan-zone", 0);
    let (zx, zy) = (zone[0] + 60.0, zone[1] + zone[3] / 2.0);
    let scroll_before = e.e.scroll_position();
    e.drag((zx, zy), (zx + 100.0, zy), 8);
    assert!(e.data("gestureText").starts_with("pan 结束：位移 100, 0"), "{}", e.data("gestureText"));
    assert_eq!(e.e.scroll_position(), scroll_before, "pan 手势抓住了这串触摸，页面不滚");
    assert!(e.wait_until(2500, |e| e.text("getCurrentPages()[0].dx.value") == "0"), "spring 回到 0：{}", e.text("getCurrentPages()[0].dx.value"));
    // 双击（两下间隔不到 300ms）
    let boxes = e.visible(".gest-box", 0);
    let (bx, by) = (boxes[0] + 20.0, boxes[1] + 20.0);
    e.tap_at(bx, by);
    e.tap_at(bx, by);
    assert_eq!(e.data("gestureText"), "双击了（double-tap-gesture-handler）");
    // 长按：按住 600ms 报 BEGIN，松手报 END
    let lp = e.rects(".gest-box")[1];
    e.e.pointer_down(lp[0] + 20.0, lp[1] + 20.0);
    e.wait(650);
    assert_eq!(e.data("gestureText"), "长按开始（BEGIN）");
    e.e.pointer_up(lp[0] + 20.0, lp[1] + 20.0);
    e.frames(3);
    assert_eq!(e.data("gestureText"), "长按松手（END）");

    // draggable-sheet：scrollTo 0.9 → 面板高 = 0.9 × 240；往下拖 90px 松手，吸到最近的吸附点 0.5，页面不跟着滚
    e.tap(B, 5);
    assert_eq!(e.data("sheetPct"), "90");
    let panel = e.visible(".sheet-panel", 0);
    assert!((panel[3] - 216.0).abs() < 1.0, "{panel:?}");
    let scroll_before = e.e.scroll_position();
    e.drag((panel[0] + 100.0, panel[1] + 20.0), (panel[0] + 100.0, panel[1] + 110.0), 8);
    assert_eq!(e.data("sheetPct"), "50", "松手吸到 0.5");
    assert_eq!(e.e.scroll_position(), scroll_before, "拖面板时页面不滚（catchtouchmove）");

    // this.animate：800ms 后回调、#box 停在终态颜色；进度条跟着共享值到 100%；clearAnimation 回原样
    e.tap(B, 7);
    assert!(e.wait_until(2500, |e| e.data("animText").starts_with("this.animate 结束")), "{}", e.data("animText"));
    let bar = e.visible("#bar", 0);
    let bg = e.rects(".bar-bg")[0];
    assert!((bar[2] - bg[2]).abs() < 1.0, "进度条到 100%：{bar:?} / {bg:?}");
    let bx = e.rects("#box")[0];
    assert_eq!(e.px(bx[0] + bx[2] / 2.0, bx[1] + bx[3] / 2.0), [0xfa, 0x9d, 0x3b], "终态 #fa9d3b 保留");
    e.tap(B, 9);
    e.wait(100);
    assert_eq!(e.data("animText"), "已清掉 #box 的动画样式");
    let bx = e.rects("#box")[0];
    assert_eq!(e.px(bx[0] + bx[2] / 2.0, bx[1] + bx[3] / 2.0), [0x07, 0xc1, 0x60], "回到原来的 #07c160");
    let bar = e.rects("#bar")[0];
    assert!(bar[2] < 1.0, "进度条清零：{bar:?}");
}

/// builtin 页 `.btns > view`：0 底部弹出、1 居中、2 右侧、3 root-portal、4 audio.play、5 pause、
/// 6 setContents、7 insertText、8 getContents、9 clear
#[test]
fn builtin_页_page_container_portal_media_editor_占位() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut e = E::open("pages/builtin/builtin", "builtin");
    const B: &str = ".btns > view";
    assert_eq!(e.text("getCurrentPages()[0].data.navTitle"), "补齐的官方组件");

    // page-container：弹出后返回键只关它，不离开页面
    e.tap(B, 0);
    e.wait(400);
    assert!(e.data("pcText").starts_with("enter"), "{}", e.data("pcText"));
    let body = e.rects(".pc-body")[0];
    assert!(body[1] + body[3] > 660.0 && body[1] > 300.0, "底部弹出、贴着窗口底边：{body:?}");
    assert!(e.px(20.0, 100.0).iter().all(|c| *c < 120), "遮罩压暗了页面：{:?}", e.px(20.0, 100.0));
    e.e.back();
    assert!(e.wait_until(1000, |e| e.data("pcText") == "afterleave：关掉了"), "{}", e.data("pcText"));
    assert_eq!(e.route(), "pages/builtin/builtin", "返回键只关了 page-container");
    assert_eq!(e.rects(".pc-body").len(), 0);

    // root-portal：enable 后脱离文档流，贴窗口顶部（页面滚到哪都在 0）
    e.tap(B, 3);
    e.frames(3);
    let p = e.rects(".portal")[0];
    assert_eq!((p[0], p[1], p[2]), (0.0, 0.0, 375.0), "{p:?} portal={} route={}", e.data("portal"), e.route());
    e.e.eval("wx.pageScrollTo({ scrollTop: 300, duration: 0 })").ok();
    e.wait(100);
    assert_eq!(e.rects(".portal")[0][1], 0.0);
    assert_eq!(e.px(10.0, 5.0), [0x07, 0xc1, 0x60], "portal 盖在最上面");
    e.tap_at(10.0, 5.0); // 点它不会穿透
    e.tap(B, 3);
    assert!(e.rects(".portal")[0][1] > 0.0, "收回原处");

    // match-media：375 宽只看得到 max-width 399 与 portrait 那两块
    assert_eq!((e.rects(".mm-a").len(), e.rects(".mm-b").len(), e.rects(".mm-c").len()), (1, 0, 1));

    // editor：setContents 之后 getContents 回 html / text
    assert!(e.wait_until(500, |e| e.data("edText").contains("EditorContext")), "{}", e.data("edText"));
    e.tap(B, 6);
    e.tap(B, 7);
    e.tap(B, 8);
    let ed = e.data("edText");
    assert!(ed.contains("<p>第一段</p><p>第二段 &amp; 特殊字符【插入】</p>"), "{ed}");
    // text 末尾带一个换行：微信 editor 底下是 Quill，getText 总以 \n 结尾
    assert!(ed.contains(r#"text: "第一段\n第二段 & 特殊字符【插入】\n""#), "{ed}");
    e.tap(B, 9);
    assert_eq!(e.data("edText"), r#"bindinput："\n""#, "清空后只剩 Quill 的那个换行");

    // audio：src 是 createBufferURL 出来的 wav
    assert!(e.data("toneSrc").starts_with("wxfile://tmp/") && e.data("toneSrc").ends_with(".wav"), "{}", e.data("toneSrc"));

    // 原生组件占位：map 画浅灰底，cover-view（半透明黑底）盖在它上面
    let m = e.visible(".ph-map", 0);
    assert_eq!(e.px(m[0] + m[2] - 20.0, m[1] + m[3] - 20.0), [0xf0, 0xf0, 0xf0], "map 占位");
    let cv = e.rects(".cv")[0];
    let c = e.px(cv[0] + cv[2] - 12.0, cv[1] + cv[3] / 2.0);
    assert!(c.iter().all(|v| *v < 120), "cover-view 盖在 map 上：{c:?}");
    let cam = e.visible(".ph-cam", 0);
    assert_eq!(e.px(cam[0] + 10.0, cam[1] + 10.0), [0xf0, 0xf0, 0xf0], "camera 占位");
}

/// newapi 页 `.btns > view`：0 navigateTo+events、1 不存在的页面、2 返回询问、3 Worker、4 UDP、5 TCP、6 本机 IP、
/// 7 随机数、8 批量存储、9 不存在的 key、10 getStorageInfo、11 振荡器、12 decodeAudioData、13 蓝牙、14 生物认证、15 微信运动
#[test]
fn newapi_页_路由_worker_网络_存储_webaudio_宿主能力() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut e = E::open("pages/newapi/newapi", "newapi");
    const B: &str = ".btns > view";

    // EventChannel：打开方 success 里 emit，新页面收到；新页面 emit 回去并返回
    e.tap(B, 0);
    assert!(e.wait_until(1500, |e| e.route() == "pages/channel/channel" && e.data("got").contains("你好，新页面")), "{} {}", e.route(), e.data("got"));
    assert_eq!(e.data("query"), r#"{"from":"newapi"}"#);
    e.wait(400); // 转场走完
    e.tap(".btns > view", 0);
    assert!(e.wait_until(1500, |e| e.route() == "pages/newapi/newapi" && e.data("routeText").contains("收到了，我是 channel 页")), "{} {}", e.route(), e.data("routeText"));
    e.wait(400);
    // 路由校验：不存在的页面立刻 fail
    e.tap(B, 1);
    assert!(e.wait_until(500, |e| e.data("routeText").starts_with("fail：navigateTo:fail")), "{}", e.data("routeText"));
    // 返回询问：开着时返回键先弹确认，页面不走
    e.tap(B, 2);
    e.e.back();
    e.wait(300);
    assert_eq!(e.route(), "pages/newapi/newapi", "开了返回询问，返回键没直接离开");
    e.e.eval("wx.disableAlertBeforeUnload()").ok();

    // Worker
    e.tap(B, 3);
    assert!(e.wait_until(3000, |e| e.data("workerText").starts_with("Worker 回来：fib(25) = 75025")), "{}", e.data("workerText"));
    // UDP 回环 / TCP 连不上 / 本机 IP
    e.tap(B, 4);
    assert!(e.wait_until(2000, |e| e.data("netText").starts_with("UDP 收到 \"ping")), "{}", e.data("netText"));
    e.tap(B, 5);
    assert!(e.wait_until(3000, |e| e.data("netText").starts_with("TCP onError：")), "{}", e.data("netText"));
    e.tap(B, 6);
    assert!(e.wait_until(1000, |e| e.data("netText").starts_with("本机 IP：")), "{}", e.data("netText"));

    // 随机数（12 个字节）/ 批量存储 / 不存在的 key 走 Promise reject / getStorageInfo
    e.tap(B, 7);
    assert!(e.wait_until(500, |e| e.data("dataText").starts_with("randomValues")), "{}", e.data("dataText"));
    let hexes = e.data("dataText").split('：').nth(1).unwrap_or("").split(' ').count();
    assert_eq!(hexes, 12);
    e.tap(B, 8);
    assert!(e.wait_until(500, |e| e.data("dataText") == r#"batchGetStorage dataList：[1,{"ok":true},""]"#), "{}", e.data("dataText"));
    e.tap(B, 9);
    assert!(e.wait_until(500, |e| e.data("dataText") == "Promise reject：getStorage:fail data not found"), "{}", e.data("dataText"));
    e.tap(B, 10);
    assert!(e.wait_until(500, |e| e.data("dataText").contains("demo_a")), "{}", e.data("dataText"));

    // WebAudio
    e.tap(B, 11);
    assert!(e.wait_until(2500, |e| e.data("audioText").starts_with("onended：振荡器停了")), "{}", e.data("audioText"));
    e.tap(B, 12);
    assert!(e.wait_until(1000, |e| e.data("audioText").starts_with("decodeAudioData：8000Hz 1 声道 0.60s")), "{}", e.data("audioText"));
    assert!(e.wait_until(2500, |e| e.data("audioText") == "BufferSource 播完了"), "{}", e.data("audioText"));

    // 媒体查询：375 宽不满足 minWidth 400
    assert!(e.data("mqText").contains("matches=false（窗口 375×667）"), "{}", e.data("mqText"));
    // 旧版画布：clip 只留左半，虚线一段实一段空
    let cv = e.visible(".legacy", 0);
    assert_eq!(e.px(cv[0] + 30.0, cv[1] + 30.0), [0x07, 0xc1, 0x60], "clip 里画上了");
    assert_ne!(e.px(cv[0] + 250.0, cv[1] + 20.0), [0x07, 0xc1, 0x60], "clip 外没画");
    assert_eq!(e.px(cv[0] + 15.0, cv[1] + 80.0), [0x16, 0x77, 0xff], "虚线第一段实");
    assert_ne!(e.px(cv[0] + 26.0, cv[1] + 80.0), [0x16, 0x77, 0xff], "虚线第一段空");

    // 宿主能力：没接管的立刻 fail，canIUse 为 false
    e.tap(B, 13);
    let h = e.data("hostText");
    assert!(h.starts_with("startBluetoothDevicesDiscovery → fail：") && h.ends_with("canIUse(startBluetoothDevicesDiscovery) = false"), "{h}");
}
