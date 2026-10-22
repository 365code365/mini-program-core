//! 移动端 SDK（`MiniEngine`）的**返回**回归：宿主栈与逻辑层页面栈的交接。
//!
//! 现象（Flutter 示例，2026-10-21）：WXML 页 → 详情页 → 详情页里 `wx.navigateBack()`，
//! 回来的 WXML 页上 `{{a}}` `{{user.name}}` 全空、「点我切换」点了没反应。
//! 原因是逻辑层被退了两层：`wx.navigateBack` 自己出一次栈，SDK 收到请求又 `__popPage(1)`，
//! 回到的页面拿着上上一页的 data 渲染、方法也是上上一页的。onUnload 还派给了要回去的那页。
//! 桌面窗体按路由比来防重复出栈，所以桌面上看不出来 —— 只有 SDK 这条路坏了。
//!
//! 交接是显式的：`wx.navigateBack` 记下替宿主出了几层（`__takeLogicPopped`），宿主据此只退
//! 自己的栈。不按两边栈的层数对齐 —— 两边层数不一定相等（桌面 `--route` 启动时逻辑层多压了
//! 一个首页），按层数对齐在那种情况下又会多退一层。
//!
//! 独立集成测试二进制、用例之间串行（`SERIAL`）：小程序根目录是进程级的（`assets::set_app_root`），
//! 放进 lib 的单测里会和用 `sample-app` 的引擎用例互相读错目录（同 `form_e2e.rs`）。
use mini_render::host::MiniEngine;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

static SERIAL: Mutex<()> = Mutex::new(());

const PAGE_JS: &str = r#"
var __navLog = globalThis.__navLog || (globalThis.__navLog = []);
Page({
  data: { name: 'NAME', n: 0 },
  bump() { this.setData({ n: this.data.n + 1 }); },
  onShow() { __navLog.push('NAME.onShow'); },
  onUnload() { __navLog.push('NAME.onUnload'); }
});
"#;

/// 引擎 + 串行锁 + 用完删掉的夹具目录
struct Fx {
    e: MiniEngine,
    root: PathBuf,
    _serial: MutexGuard<'static, ()>,
}
impl Drop for Fx {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}
impl std::ops::Deref for Fx {
    type Target = MiniEngine;
    fn deref(&self) -> &MiniEngine {
        &self.e
    }
}
impl std::ops::DerefMut for Fx {
    fn deref_mut(&mut self) -> &mut MiniEngine {
        &mut self.e
    }
}

/// 三个页面 a / b / c，各自往 `__navLog` 里记 onShow / onUnload；从 `start` 页启动
fn fixture_at(slot: &str, start: &str) -> Fx {
    let serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("navigate-back-e2e").join(slot);
    std::fs::remove_dir_all(&root).ok();
    let data = root.join("_data");
    let w = |rel: &str, body: &str| {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    };
    w("app.json", r#"{"pages":["pages/a/a","pages/b/b","pages/c/c"]}"#);
    w("app.js", "App({});");
    for name in ["a", "b", "c"] {
        let up = name.to_uppercase();
        w(&format!("pages/{name}/{name}.js"), &PAGE_JS.replace("NAME", &up));
        w(&format!("pages/{name}/{name}.wxml"), "<view>{{name}}-{{n}}</view>");
    }
    let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0)
        .expect("创建引擎");
    e.launch(Some(start)).expect("打开首页");
    e.pump(1);
    Fx { e, root, _serial: serial }
}

fn fixture(slot: &str) -> Fx {
    fixture_at(slot, "pages/a/a")
}

/// 跑一段逻辑层代码，再 pump 一次让宿主取走路由请求
fn js(e: &mut MiniEngine, code: &str) {
    e.eval(code).expect(code);
    e.pump(0);
}

fn val(e: &MiniEngine, code: &str) -> String {
    e.eval(code).expect(code)
}

fn log(e: &MiniEngine) -> String {
    val(e, "JSON.stringify(__navLog)")
}

#[test]
fn wx_navigate_back_回到上一页时_data_与方法都是上一页的() {
    let mut e = fixture("back1");
    js(&mut e, "__currentPage.bump()");
    js(&mut e, "wx.navigateTo({ url: '/pages/b/b' })");
    js(&mut e, "wx.navigateTo({ url: '/pages/c/c' })");
    assert_eq!(e.page_depth(), 3);
    js(&mut e, "__navLog.length = 0");

    js(&mut e, "wx.navigateBack()");
    assert_eq!(e.current_route(), "pages/b/b");
    assert_eq!(e.page_depth(), 2);
    assert_eq!(val(&e, "getCurrentPages().length"), "2", "逻辑层只退一层");
    assert_eq!(val(&e, "__currentPage.data.name"), "B");
    // 只有关掉的 C 收到 onUnload，回到的 B 收到 onShow
    assert_eq!(log(&e), r#"["C.onUnload","B.onShow"]"#);

    js(&mut e, "wx.navigateBack()");
    assert_eq!(e.current_route(), "pages/a/a");
    assert_eq!(val(&e, "__currentPage.data.name"), "A");
    assert_eq!(val(&e, "String(__currentPage.data.n)"), "1", "A 页离开前的状态还在");
    // 方法是这一页的：点了能改到这一页的 data
    js(&mut e, "__currentPage.bump()");
    assert_eq!(val(&e, "String(__currentPage.data.n)"), "2");
}

#[test]
fn wx_navigate_back_delta_按层数退_同一路由叠两层也不漏() {
    let mut e = fixture("delta");
    js(&mut e, "wx.navigateTo({ url: '/pages/b/b' })");
    js(&mut e, "wx.navigateTo({ url: '/pages/b/b' })");
    js(&mut e, "wx.navigateTo({ url: '/pages/c/c' })");
    assert_eq!(e.page_depth(), 4);
    js(&mut e, "__navLog.length = 0");

    js(&mut e, "wx.navigateBack({ delta: 2 })");
    assert_eq!(e.page_depth(), 2, "delta 生效");
    assert_eq!(val(&e, "getCurrentPages().length"), "2");
    assert_eq!(e.current_route(), "pages/b/b");
    assert_eq!(log(&e), r#"["C.onUnload","B.onUnload","B.onShow"]"#);

    // 第二层的 B 退回第一层的 B：路由相同，按路由比的话会以为逻辑层已经出过栈
    js(&mut e, "__navLog.length = 0");
    assert!(e.back());
    assert_eq!(e.page_depth(), 1);
    assert_eq!(val(&e, "getCurrentPages().length"), "1");
    assert_eq!(val(&e, "__currentPage.data.name"), "A");
    assert_eq!(log(&e), r#"["B.onUnload","A.onShow"]"#);
}

#[test]
fn 宿主返回键_逻辑层跟着退一层_onUnload_给关掉的那页() {
    let mut e = fixture("host");
    js(&mut e, "wx.navigateTo({ url: '/pages/b/b' })");
    js(&mut e, "__navLog.length = 0");
    assert!(e.back());
    e.pump(0);
    assert_eq!(e.current_route(), "pages/a/a");
    assert_eq!(val(&e, "getCurrentPages().length"), "1");
    assert_eq!(val(&e, "__currentPage.data.name"), "A");
    assert_eq!(log(&e), r#"["B.onUnload","A.onShow"]"#);
    // 只剩一页：交还给宿主关小程序，栈不动
    assert!(!e.back());
    assert_eq!(val(&e, "getCurrentPages().length"), "1");
}

#[test]
fn 深链进来的页面里_wx_navigate_back_只退一层() {
    // 不从首页启动：逻辑层与宿主两边的层数不一定相等，交接不能靠比层数
    let mut e = fixture_at("deeplink", "pages/b/b");
    js(&mut e, "wx.navigateTo({ url: '/pages/c/c' })");
    let before = val(&e, "getCurrentPages().length").parse::<usize>().unwrap();
    js(&mut e, "wx.navigateBack()");
    assert_eq!(e.current_route(), "pages/b/b");
    assert_eq!(val(&e, "getCurrentPages().length").parse::<usize>().unwrap(), before - 1);
    assert_eq!(val(&e, "__currentPage.route"), "pages/b/b");
    assert_eq!(val(&e, "__currentPage.data.name"), "B");
}

#[test]
fn 只剩一页时_wx_navigate_back_不留账给下一次返回() {
    let mut e = fixture("root");
    js(&mut e, "wx.navigateBack()"); // 栈底：什么都不做
    js(&mut e, "wx.navigateTo({ url: '/pages/b/b' })");
    js(&mut e, "__navLog.length = 0");
    // 宿主返回键：逻辑层要跟着退（上一次 navigateBack 没出栈，不能算成「已经替宿主出过」）
    assert!(e.back());
    assert_eq!(val(&e, "getCurrentPages().length"), "1");
    assert_eq!(log(&e), r#"["B.onUnload","A.onShow"]"#);
}
