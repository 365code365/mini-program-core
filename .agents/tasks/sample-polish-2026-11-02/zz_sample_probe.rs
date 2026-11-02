//! 临时探针（本轮完善示例小程序用，用完删源文件）：按脚本驱动 `sample/<app>`，走 harness 的真指针链路。
//!
//! ```bash
//! PROBE_APP=coffee PROBE_SCRIPT=target/_probe/coffee.txt \
//!   cargo test --release --test zz_sample_probe -- --nocapture
//! ```
//!
//! 脚本一行一条（`#` 开头是注释），失败时报出行号与当时的页面：
//!
//! | 命令 | 含义 |
//! |---|---|
//! | `seed KEY => 值` | 下一次 `open` 之前往 storage 里放一项（值原样是字符串） |
//! | `demo` | 本机起演示接口，下一次 `open` 时 `demoApiBase` 指过去（光影放映厅 / 云听音乐 / 拾光图库 / 跳转中心） |
//! | `waitfor 表达式 [毫秒]` | 等到 JS 表达式为真（默认最多 5000ms） |
//! | `open [路由]` | 清空 storage 重新打开（不给路由 = 首页） |
//! | `smoke [路由?参数 ...]` | 每一页单独打开：不报错、不空白（要参数的页面在后面给） |
//! | `tap SEL` / `tapn I SEL` / `tapxy X Y` | 点（不在视口里先 pageScrollTo），点完等页面安定 |
//! | `long SEL` | 长按 |
//! | `type SEL => 文本` / `key 名` / `blur` | 输入 |
//! | `wait MS` / `frames N` / `settle` | 等 |
//! | `modal ok\|cancel` / `sheet I N` | 系统弹窗 / 操作菜单（共 N 项，点第 I 项；I == N 是取消） |
//! | `picker SEL` / `pickrow 列 列数 偏移` / `pickok` / `pickcancel` | picker 面板 |
//! | `back` / `drag X Y DX DY` / `show SEL` | 返回 / 拖动 / 把元素滚进视口 |
//! | `eval JS` / `print 表达式` / `data 路径` | 执行 / 打印 |
//! | `expect 表达式` | JS 表达式必须为真 |
//! | `route R` / `exists SEL` / `absent SEL` / `count SEL N` | 断言 |
//! | `modal?` / `nomodal` | 断言系统弹窗开着 / 没开 |
//! | `toast 文字` | 最近一次 showToast 的标题包含该文字 |
//! | `shot 路径.png` / `noerr` | 存图 / 断言到此没有逻辑层报错 |
//!
//! `wx.showToast / showModal / showActionSheet` 被记进 `globalThis.__ui`（`{k, title, content, items}`），
//! 脚本里可以 `expect __ui.slice(-1)[0].title === '…'`。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use serde_json::Value;

const UI_HOOK: &str = r#"
globalThis.__ui = [];
(function () {
  function wrap(name, pick) {
    var f = wx[name];
    if (typeof f !== 'function' || f.__probe) return;
    var g = function (o) { try { __ui.push(pick(o || {})); } catch (e) {} return f.apply(this, arguments); };
    g.__probe = true;
    wx[name] = g;
  }
  wrap('showToast', function (o) { return { k: 'toast', title: o.title }; });
  wrap('showModal', function (o) { return { k: 'modal', title: o.title, content: o.content }; });
  wrap('showActionSheet', function (o) { return { k: 'sheet', items: o.itemList }; });
})();
"#;

struct Probe {
    app: String,
    a: Option<App>,
    seeds: Vec<(String, String)>,
    opens: u32,
}

impl Probe {
    fn a(&mut self) -> &mut App {
        self.a.as_mut().expect("脚本要先 open")
    }

    fn open(&mut self, route: Option<&str>) {
        self.a = None;
        self.opens += 1;
        let seeds: Vec<(&str, &str)> = self.seeds.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let a = App::open_seeded(&self.app, route, &format!("probe-{}", self.opens), Default::default(), &seeds);
        a.js(UI_HOOK);
        self.seeds.clear();
        self.a = Some(a);
    }

    /// 元素不在视口里：先按页面滚动把它挪到视口中间（在 scroll-view 里的挪不动，交给脚本自己 drag）
    fn show(&mut self, sel: &str, i: usize) {
        let a = self.a();
        let r = a.rects(sel);
        let Some(&(_, y, _, h)) = r.get(i) else { return };
        let cy = y + h / 2.0;
        // 底部留出 tabBar（自定义 tabBar 常有凸起的中间按钮，按 120 算）：引擎里 tab 页的视口含 tabBar 区域
        if cy > 40.0 && cy < H - 120.0 {
            return;
        }
        let top = (a.e.scroll_position() + cy - H / 2.0).max(0.0);
        a.js(&format!("wx.pageScrollTo({{ scrollTop: {top}, duration: 0 }})"));
        a.settle();
        a.wait_scroll_still();
    }

    fn tap(&mut self, sel: &str, i: usize) {
        self.show(sel, i);
        let a = self.a();
        a.tap_nth(sel, i);
        a.settle();
    }

    fn truthy(&mut self, expr: &str) -> (bool, Value) {
        let a = self.a();
        let v = a.val(&format!("(function () {{ try {{ return {{ ok: !!({expr}), v: ({expr}) }}; }} catch (e) {{ return {{ ok: false, v: 'throw: ' + e }}; }} }})()"));
        (v["ok"].as_bool().unwrap_or(false), v["v"].clone())
    }

    fn step(&mut self, line: &str) -> Result<(), String> {
        let (cmd, rest) = line.split_once(' ').map(|(c, r)| (c, r.trim())).unwrap_or((line, ""));
        let arrow = |s: &str| s.split_once(" => ").map(|(a, b)| (a.trim().to_string(), b.to_string()));
        let nums = |s: &str| s.split_whitespace().map(|x| x.parse::<f32>().unwrap_or(0.0)).collect::<Vec<_>>();
        match cmd {
            "seed" => {
                let (k, v) = arrow(rest).ok_or("seed KEY => 值")?;
                self.seeds.push((k, v));
            }
            // 拉网络数据的示例：本机起演示接口（tools/mini-store/server.py），下一次 open 时 demoApiBase 指过去
            "demo" => {
                let base = demo_server::base();
                println!("      演示接口 {base}");
                self.seeds.push(("demoApiBase".into(), base));
            }
            // 等到 JS 表达式为真（网络、定时器），最多 N 毫秒（默认 5000）
            "waitfor" => {
                let (expr, ms) = match rest.rsplit_once(' ') {
                    Some((e, n)) if n.parse::<u64>().is_ok() => (e.to_string(), n.parse::<u64>().unwrap()),
                    _ => (rest.to_string(), 5000),
                };
                let t0 = std::time::Instant::now();
                loop {
                    if self.truthy(&expr).0 {
                        break;
                    }
                    if t0.elapsed().as_millis() as u64 > ms {
                        let (_, v) = self.truthy(&expr);
                        return Err(format!("等了 {ms}ms 还不为真：{expr}\n      当前：{v}"));
                    }
                    self.a().wait_ms(100);
                }
            }
            "open" => self.open(if rest.is_empty() { None } else { Some(rest) }),
            "smoke" => {
                let qs: Vec<(String, String)> = rest
                    .split_whitespace()
                    .filter_map(|r| r.split_once('?').map(|(p, q)| (p.to_string(), q.to_string())))
                    .collect();
                let qs: Vec<(&str, &str)> = qs.iter().map(|(p, q)| (p.as_str(), q.as_str())).collect();
                smoke_all_pages(&self.app, &qs);
            }
            "tap" => self.tap(rest, 0),
            "tapn" => {
                let (i, sel) = rest.split_once(' ').ok_or("tapn I SEL")?;
                self.tap(sel.trim(), i.parse().map_err(|_| "tapn 的 I 不是数")?);
            }
            "tapxy" => {
                let n = nums(rest);
                let a = self.a();
                a.tap_xy(n[0], n[1]);
                a.settle();
            }
            "long" => {
                self.show(rest, 0);
                self.a().long_press(rest);
                self.a().settle();
            }
            "type" => {
                let (sel, text) = arrow(rest).ok_or("type SEL => 文本")?;
                self.show(&sel, 0);
                self.a().type_into(&sel, &text);
            }
            "key" => self.a().key(rest),
            "blur" => self.a().blur(),
            "wait" => self.a().wait_ms(rest.parse().unwrap_or(300)),
            "frames" => self.a().frames(rest.parse().unwrap_or(10)),
            "settle" => self.a().settle(),
            "modal" => self.a().modal_tap(rest != "cancel"),
            "sheet" => {
                let n = nums(rest);
                self.a().sheet_tap(n[0] as usize, n[1] as usize);
            }
            "picker" => {
                self.show(rest, 0);
                self.a().picker(rest);
            }
            "pickrow" => {
                let n = nums(rest);
                self.a().picker_row(n[0] as usize, n[1] as usize, n[2] as i32);
            }
            "pickok" => self.a().picker_confirm(),
            "pickcancel" => self.a().picker_cancel(),
            "back" => self.a().back(),
            "drag" => {
                let n = nums(rest);
                let a = self.a();
                a.drag(n[0], n[1], n[2], n[3], 12);
                a.wait_scroll_still();
            }
            "show" => self.show(rest, 0),
            "eval" => {
                let out = self.a().js(rest);
                println!("      = {}", out.trim());
                self.a().settle();
            }
            "print" => {
                let v = self.a().val(rest);
                println!("      {rest} = {v}");
            }
            "data" => {
                let v = self.a().data(rest);
                println!("      data.{rest} = {v}");
            }
            "expect" => {
                let (ok, v) = self.truthy(rest);
                if !ok {
                    return Err(format!("期望为真：{rest}\n      实际：{v}"));
                }
            }
            "route" => {
                let r = self.a().route();
                if r != rest {
                    return Err(format!("期望在 {rest}，实际在 {r}"));
                }
            }
            "exists" => {
                if !self.a().exists(rest) {
                    return Err(format!("页面上没有 {rest}"));
                }
            }
            "absent" => {
                if self.a().exists(rest) {
                    return Err(format!("页面上不该有 {rest}"));
                }
            }
            "count" => {
                let (n, sel) = rest.split_once(' ').ok_or("count N SEL")?;
                let got = self.a().count(sel.trim());
                if got.to_string() != n {
                    return Err(format!("{sel} 期望 {n} 个，实际 {got} 个"));
                }
            }
            "modal?" => {
                if !self.a().modal_open() {
                    return Err("期望有系统弹窗".into());
                }
            }
            "nomodal" => {
                if self.a().modal_open() {
                    return Err("不该有系统弹窗".into());
                }
            }
            "toast" => {
                let (ok, v) = self.truthy(&format!(
                    "(__ui.filter(function (x) {{ return x.k === 'toast'; }}).slice(-1)[0] || {{}}).title"
                ));
                let t = v.as_str().unwrap_or("");
                if !ok || !t.contains(rest) {
                    return Err(format!("最近的 toast 是 {v}，期望包含「{rest}」"));
                }
            }
            "shot" => {
                let a = self.a();
                a.frames(2);
                let (w, h) = a.e.pixel_size();
                if let Some(dir) = std::path::Path::new(rest).parent() {
                    std::fs::create_dir_all(dir).ok();
                }
                image::save_buffer(rest, a.e.rgba(), w, h, image::ColorType::Rgba8).map_err(|e| e.to_string())?;
            }
            "noerr" => {
                let errs = self.a().errors();
                if !errs.is_empty() {
                    return Err(format!("逻辑层报错：{errs:#?}"));
                }
            }
            _ => return Err(format!("不认识的命令 {cmd}")),
        }
        Ok(())
    }
}

#[test]
fn probe() {
    let _g = serial();
    let app = std::env::var("PROBE_APP").expect("要 PROBE_APP=<示例名>");
    let path = std::env::var("PROBE_SCRIPT").expect("要 PROBE_SCRIPT=<脚本路径>");
    let script = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读不了 {path}：{e}"));
    let mut p = Probe { app: app.clone(), a: None, seeds: vec![], opens: 0 };
    for (no, raw) in script.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        println!("[{app}:{}] {line}", no + 1);
        if let Err(msg) = p.step(line) {
            let route = p.a.as_ref().map(|a| a.route()).unwrap_or_default();
            let errs = p.a.as_ref().map(|a| a.errors()).unwrap_or_default();
            panic!("\n❌ {path}:{} `{line}`\n   页面 {route}\n   {msg}\n   逻辑层报错：{errs:#?}", no + 1);
        }
    }
    if let Some(a) = p.a.as_ref() {
        a.assert_no_errors();
    }
    println!("✅ {app} 脚本 {path} 全部通过");
}
