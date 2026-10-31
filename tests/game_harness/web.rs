//! 高性能模式（game/web：游戏 JS 跑在 WebView 里）的 e2e 驱动：与 [`super::GameHarness`] 同一套夹具与 `T:*` 日志，
//! 页面跑在 headless Chrome 里（`game::web::cdp`），原生侧由 `game::web::devserver` 扮演 iOS 宿主。
//! 找不到 Chrome 时 [`WebHarness::open`] 返回 None，用例打一行「跳过」后直接通过（CI 上设 `MINI_CHROME`）。
#![allow(dead_code)]
use mini_render::game::web::{cdp::Chrome, devserver::DevServer, WebGame};
use mini_render::game::GameOptions;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct WebOpts {
    pub query: &'static str,
    pub host_apis: &'static [&'static str],
    pub app_id: &'static str,
    /// 视口（CSS 像素）；None = 按 game.json 的方向给 375×667 / 667×375
    pub size: Option<(u32, u32)>,
    /// 高性能+：画布 / WebGL 交回渲染端回放（game/remote），像素从渲染端取
    pub plus: bool,
}
impl Default for WebOpts {
    fn default() -> Self {
        Self { query: "", host_apis: &[], app_id: "wxweb000000000000", size: None, plus: false }
    }
}

pub struct WebHarness {
    pub chrome: Chrome,
    pub srv: DevServer,
    pub log: Arc<Mutex<Vec<String>>>,
    pub host: Arc<Mutex<Vec<Value>>>,
    pub errors: Arc<Mutex<Vec<Value>>>,
    pub dir: PathBuf,
    pub data: PathBuf,
    pub size: (u32, u32),
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let (p, q) = (e.path(), to.join(e.file_name()));
        if p.is_dir() { copy_dir(&p, &q) } else { std::fs::copy(&p, &q).unwrap(); }
    }
}

impl WebHarness {
    /// 打开 `sample-game/<name>`（连同 `_lib`）
    pub fn open(name: &str, opts: WebOpts) -> Option<Self> {
        let dir = Self::scratch(name);
        copy_dir(&repo().join("sample-game").join(name), &dir);
        copy_dir(&repo().join("sample-game/_lib"), &dir.join("_lib"));
        Self::start(dir, opts)
    }
    /// 现场写一个夹具（game.json + game.js；`extra` 是其余文件）
    pub fn open_inline(name: &str, game_json: &str, game_js: &str, extra: &[(&str, &[u8])], opts: WebOpts) -> Option<Self> {
        let dir = Self::scratch(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("game.json"), game_json).unwrap();
        std::fs::write(dir.join("game.js"), game_js).unwrap();
        for (p, b) in extra {
            let f = dir.join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, b).unwrap();
        }
        copy_dir(&repo().join("sample-game/_lib"), &dir.join("_lib"));
        Self::start(dir, opts)
    }
    /// 同一个包、同一个数据目录再开一次（重启之后读存档）
    pub fn reopen(mut self, opts: WebOpts) -> Option<Self> {
        // 路径拿走之后 Drop 不删目录：只关 Chrome、停这一份实例（存档落盘）
        let (dir, data) = (std::mem::take(&mut self.dir), std::mem::take(&mut self.data));
        drop(self);
        Self::start_with(dir, data, opts)
    }

    fn scratch(name: &str) -> PathBuf {
        let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("web-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(d.with_extension("data"));
        d
    }

    fn start(dir: PathBuf, opts: WebOpts) -> Option<Self> {
        let data = dir.with_extension("data");
        Self::start_with(dir, data, opts)
    }

    fn start_with(dir: PathBuf, data: PathBuf, opts: WebOpts) -> Option<Self> {
        if mini_render::game::web::cdp::chrome_path().is_none() {
            eprintln!("⚠ 跳过高性能模式用例：找不到 Chrome（设 MINI_CHROME=<路径>）");
            return None;
        }
        let query: serde_json::Map<String, Value> = opts.query.split('&').filter(|s| !s.is_empty())
            .filter_map(|kv| kv.split_once('=').map(|(k, v)| (k.to_string(), Value::String(v.to_string())))).collect();
        let go = GameOptions {
            app_id: Some(opts.app_id.into()),
            launch_options: Some(serde_json::json!({ "query": query, "scene": 1001 })),
            host_apis: opts.host_apis.iter().map(|s| s.to_string()).collect(),
            remote: opts.plus,
            ..Default::default()
        };
        let land = std::fs::read_to_string(dir.join("game.json")).ok()
            .and_then(|t| mini_render::game::config::GameConfig::parse(&t).ok())
            .is_some_and(|c| c.device_orientation.is_landscape());
        let size = opts.size.unwrap_or(if land { (667, 375) } else { (375, 667) });
        let game = WebGame::new(dir.to_str().unwrap(), Some(data.to_str().unwrap()), size.0, size.1, 2.0, &go).expect("建实例");
        let log: Arc<Mutex<Vec<String>>> = Default::default();
        let host: Arc<Mutex<Vec<Value>>> = Default::default();
        let errors: Arc<Mutex<Vec<Value>>> = Default::default();
        let (l, h, e) = (log.clone(), host.clone(), errors.clone());
        let srv = DevServer::start(game, move |raw| {
            let v: Value = serde_json::from_str(raw).unwrap_or_default();
            match v["type"].as_str() {
                Some("log") => l.lock().unwrap().extend(v["text"].as_str().unwrap_or("").lines().map(str::to_string)),
                Some("error") => e.lock().unwrap().push(v),
                _ => h.lock().unwrap().push(v),
            }
        }).expect("起服务");
        let mut chrome = Chrome::launch(size.0, size.1, 2.0).expect("起 Chrome");
        chrome.navigate(&srv.url()).expect("打开页面");
        let mut w = Self { chrome, srv, log, host, errors, dir, data, size };
        w.until_host("started", 20.0);
        Some(w)
    }

    /// 等日志里出现 `pat`（墙上时钟 `secs` 秒）；超时 panic 并打最后 40 行
    pub fn until(&mut self, pat: &str, secs: f64) -> &mut Self {
        let t0 = Instant::now();
        while !self.has(pat) {
            if t0.elapsed().as_secs_f64() > secs {
                let log = self.log.lock().unwrap();
                let tail: Vec<_> = log.iter().rev().take(40).rev().cloned().collect();
                panic!("{secs}s 内没等到 {pat:?}；最后的日志：\n{}\n错误：{:?}", tail.join("\n"), self.errors.lock().unwrap());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        self
    }

    /// 等一条 type 为 `ty` 的宿主事件，返回它
    pub fn until_host(&mut self, ty: &str, secs: f64) -> Value {
        let t0 = Instant::now();
        loop {
            if let Some(v) = self.host.lock().unwrap().iter().find(|v| v["type"] == ty) {
                return v.clone();
            }
            if t0.elapsed().as_secs_f64() > secs {
                panic!("{secs}s 内没等到宿主事件 {ty}；日志：{:?}", self.log.lock().unwrap().iter().rev().take(30).collect::<Vec<_>>());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn sleep(&mut self, ms: u64) -> &mut Self {
        std::thread::sleep(Duration::from_millis(ms));
        self
    }

    pub fn has(&self, pat: &str) -> bool {
        self.log.lock().unwrap().iter().any(|l| l.contains(pat))
    }

    pub fn events(&self, name: &str) -> Vec<Value> {
        let p = format!("T:EV {name} ");
        self.log.lock().unwrap().iter().filter_map(|l| l.find(&p).map(|i| l[i + p.len()..].to_string()))
            .filter_map(|j| serde_json::from_str(&j).ok()).collect()
    }

    pub fn assert_no_fail(&self) {
        let fails: Vec<_> = self.log.lock().unwrap().iter().filter(|l| l.contains("T:FAIL")).cloned().collect();
        assert!(fails.is_empty(), "夹具自检失败：{fails:#?}");
    }

    pub fn errors(&self) -> Vec<Value> {
        self.errors.lock().unwrap().clone()
    }

    /// 在页面里执行（返回值按 JSON）
    pub fn eval(&mut self, js: &str) -> Value {
        self.chrome.eval(js).unwrap_or_else(|e| panic!("eval {js:?}：{e}"))
    }

    pub fn tap(&mut self, x: f32, y: f32) {
        self.chrome.tap(x, y).unwrap();
    }

    /// 高性能+ 渲染端的输出帧里 (x, y)（逻辑像素；主画布可能是 dpr 倍，按尺寸换算）
    pub fn native_pixel(&self, x: u32, y: u32) -> [u8; 3] {
        let (w, h, px) = self.srv.renderer.as_ref().expect("不是高性能+").frame();
        let (sx, sy) = (w as f32 / self.size.0 as f32, h as f32 / self.size.1 as f32);
        let i = (((y as f32 * sy) as u32).min(h - 1) * w + ((x as f32 * sx) as u32).min(w - 1)) as usize * 4;
        [px[i], px[i + 1], px[i + 2]]
    }

    /// 渲染端输出帧的颜色数（画出东西了没有）
    pub fn native_colors(&self) -> usize {
        let (_, _, px) = self.srv.renderer.as_ref().expect("不是高性能+").frame();
        px.chunks_exact(4).map(|c| [c[0], c[1], c[2]]).collect::<std::collections::HashSet<_>>().len()
    }

    /// 截图里 (x, y)（CSS 像素）的颜色
    pub fn pixel(&mut self, x: u32, y: u32) -> [u8; 3] {
        let png = self.chrome.screenshot().unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgb8();
        let p = img.get_pixel((x * 2).min(img.width() - 1), (y * 2).min(img.height() - 1));
        [p[0], p[1], p[2]]
    }

    /// 宿主做的事：前后台、改尺寸、回 hostCall、推事件
    pub fn with_game<R>(&self, f: impl FnOnce(&mut WebGame) -> R) -> R {
        f(&mut self.srv.game.lock().unwrap())
    }

    pub fn host_events(&self, ty: &str) -> Vec<Value> {
        self.host.lock().unwrap().iter().filter(|v| v["type"] == ty).cloned().collect()
    }
}

impl Drop for WebHarness {
    fn drop(&mut self) {
        if !self.dir.as_os_str().is_empty() {
            let _ = std::fs::remove_dir_all(&self.dir);
            let _ = std::fs::remove_dir_all(&self.data);
        }
    }
}
