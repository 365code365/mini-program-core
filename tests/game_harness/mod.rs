//! 小游戏 e2e 的驱动（game-doc/08 §1.1）：把 `sample-game/<夹具>` 连同 `_lib` 拷进临时目录，建 `GameEngine`，
//! 按无头帧时钟逐帧 pump，收集宿主事件并解析夹具的 `T:*` 日志行。
//!
//! 夹具拷贝而不是原地跑：引擎会往数据目录写 storage / wxfile，原地跑会把 `sample-game/` 弄脏。

#![allow(dead_code)]

use mini_render::game::{GameEngine, GameOptions};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct Opts {
    pub hz: u32,
    pub query: &'static str,
    pub watchdog_ms: u64,
    /// 从独立分包启动（创建选项 entrySubpackage）
    pub entry: Option<&'static str>,
    /// 宿主转不了方向（创建选项 rotateInEngine，宿主给竖屏尺寸）
    pub rotate: bool,
    /// 引擎插件目录（创建选项 pluginDir）
    pub plugin_dir: Option<std::path::PathBuf>,
}

impl Default for Opts {
    fn default() -> Self {
        Self { hz: 60, query: "", watchdog_ms: 5000, entry: None, rotate: false, plugin_dir: None }
    }
}

pub struct GameHarness {
    pub engine: GameEngine,
    pub hz: u32,
    pub frame: u64,
    /// 宿主事件里的日志行（逐行）
    pub log: Vec<String>,
    /// 宿主事件里的错误（原样 JSON）
    pub errors: Vec<Value>,
    /// 其它宿主事件（audioSession …）
    pub host: Vec<Value>,
    pub dir: PathBuf,
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        let q = to.join(e.file_name());
        if p.is_dir() {
            copy_dir(&p, &q);
        } else {
            std::fs::copy(&p, &q).unwrap();
        }
    }
}

impl GameHarness {
    /// 打开 `sample-game/<name>`
    pub fn open(name: &str, opts: Opts) -> Self {
        let dir = Self::scratch(name);
        copy_dir(&repo().join("sample-game").join(name), &dir);
        copy_dir(&repo().join("sample-game/_lib"), &dir.join("_lib"));
        Self::start(dir, opts)
    }

    /// 现场写一个只有 game.js 的夹具（一次性用例用，不进 sample-game/）
    pub fn open_inline(name: &str, game_js: &str, opts: Opts) -> Self {
        Self::open_inline_with(name, "{}", game_js, opts)
    }

    /// 同上，带一份 game.json
    pub fn open_inline_with(name: &str, game_json: &str, game_js: &str, opts: Opts) -> Self {
        let dir = Self::scratch(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("game.json"), game_json).unwrap();
        std::fs::write(dir.join("game.js"), game_js).unwrap();
        copy_dir(&repo().join("sample-game/_lib"), &dir.join("_lib"));
        Self::start(dir, opts)
    }

    fn scratch(name: &str) -> PathBuf {
        let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("game-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn start(dir: PathBuf, opts: Opts) -> Self {
        let query: serde_json::Map<String, Value> = opts
            .query
            .split('&')
            .filter(|s| !s.is_empty())
            .filter_map(|kv| kv.split_once('=').map(|(k, v)| (k.to_string(), Value::String(v.to_string()))))
            .collect();
        let go = GameOptions {
            app_id: Some("wxtest0000000000".into()),
            launch_options: Some(serde_json::json!({ "query": query, "scene": 1001 })),
            watchdog_ms: Some(opts.watchdog_ms),
            fixed_clock: true,
            entry_subpackage: opts.entry.map(str::to_string),
            rotate_in_engine: opts.rotate,
            plugin_dir: opts.plugin_dir.as_ref().map(|p| p.to_string_lossy().to_string()),
            ..Default::default()
        };
        let data = dir.with_extension("data");
        // 宿主按 game.json 锁方向：横屏包给横屏窗口（G6.3）
        let land = std::fs::read_to_string(dir.join("game.json")).ok()
            .and_then(|t| mini_render::game::config::GameConfig::parse(&t).ok())
            .is_some_and(|c| c.device_orientation.is_landscape());
        let (w, h) = if land && !opts.rotate { (667, 375) } else { (375, 667) };
        let engine = GameEngine::new(dir.to_str().unwrap(), Some(data.to_str().unwrap()), w, h, 2.0, go)
            .unwrap_or_else(|e| panic!("建游戏实例失败：{e}"));
        let mut h = Self { engine, hz: opts.hz, frame: 0, log: Vec::new(), errors: Vec::new(), host: Vec::new(), dir };
        h.collect();
        h
    }

    /// 帧 n 的时刻（微秒）：n × 1e6 / hz
    pub fn now_us(&self) -> u64 {
        (self.frame * 1_000_000 / self.hz as u64).max(1)
    }

    pub fn vsyncs(&mut self, n: u32) -> &mut Self {
        for _ in 0..n {
            self.frame += 1;
            let t = self.now_us();
            self.engine.pump_ex(t);
            self.collect();
        }
        self
    }

    /// 一直跑到日志里出现 `pat`；超过 `max` 帧 panic 并打印最后 30 行
    pub fn until(&mut self, pat: &str, max: u32) -> &mut Self {
        for _ in 0..max {
            if self.log.iter().any(|l| l.contains(pat)) {
                return self;
            }
            // 定时器目前按墙上时钟到点（帧时钟在 G2 接入），给它一点真实时间
            std::thread::sleep(std::time::Duration::from_millis(5));
            self.vsyncs(1);
        }
        let tail: Vec<_> = self.log.iter().rev().take(30).rev().cloned().collect();
        panic!("{max} 帧内没等到 {pat:?}；最后的日志：\n{}", tail.join("\n"));
    }

    fn collect(&mut self) {
        while let Some(e) = self.engine.poll_event() {
            let v: Value = serde_json::from_str(&e).unwrap();
            match v["type"].as_str() {
                Some("log") => self.log.extend(v["text"].as_str().unwrap_or("").lines().map(str::to_string)),
                Some("error") => self.errors.push(v),
                _ => self.host.push(v),
            }
        }
    }

    /// 解析出全部 `T:EV <name> <json>`
    pub fn events(&self, name: &str) -> Vec<Value> {
        let p = format!("T:EV {name} ");
        self.log.iter().filter_map(|l| l.find(&p).map(|i| &l[i + p.len()..])).filter_map(|j| serde_json::from_str(j).ok()).collect()
    }

    pub fn frames(&self) -> Vec<(u64, Value)> {
        self.log
            .iter()
            .filter_map(|l| l.find("T:FRAME ").map(|i| &l[i + 8..]))
            .filter_map(|r| {
                let (n, j) = r.split_once(' ')?;
                Some((n.parse().ok()?, serde_json::from_str(j).ok()?))
            })
            .collect()
    }

    /// 没有 `T:FAIL`
    pub fn assert_no_fail(&self) {
        let fails: Vec<_> = self.log.iter().filter(|l| l.contains("T:FAIL")).collect();
        assert!(fails.is_empty(), "夹具自检失败：{fails:#?}");
    }

    /// 主画布像素（不透明 RGBA，主画布分辨率）里某个颜色的包围盒 (x0, y0, x1, y1)，含端点
    pub fn bbox_of(&mut self, rgb: [u8; 3]) -> Option<(u32, u32, u32, u32)> {
        let (w, _h, px) = self.engine.frame_rgba();
        let mut b: Option<(u32, u32, u32, u32)> = None;
        for (i, p) in px.chunks(4).enumerate() {
            if p[..3] == rgb {
                let (x, y) = ((i as u32) % w, (i as u32) / w);
                b = Some(match b {
                    None => (x, y, x, y),
                    Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                });
            }
        }
        b
    }

    pub fn count_of(&mut self, rgb: [u8; 3]) -> usize {
        let (_, _, px) = self.engine.frame_rgba();
        px.chunks(4).filter(|p| p[..3] == rgb).count()
    }

    pub fn pixel(&mut self, x: u32, y: u32) -> [u8; 4] {
        let (w, _, px) = self.engine.frame_rgba();
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2], px[i + 3]]
    }

    pub fn has(&self, pat: &str) -> bool {
        self.log.iter().any(|l| l.contains(pat))
    }
}

impl Drop for GameHarness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
        let _ = std::fs::remove_dir_all(self.dir.with_extension("data"));
    }
}
