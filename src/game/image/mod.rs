//! `wx.createImage()` 的原生侧（game-doc/02 §5.3、05 GT-I）：按实例的图片对象表 + 解码缓存。
//!
//! - **对象**（JS 的 Image）只记 `src` 与解好的图（`Arc<Decoded>`）；**缓存**按 src 存解码结果，同一 src
//!   同时只解一次（在飞的登记等待者，GT-I4），解完之后再设同一 src 直接命中（GT-I5：drawImage 一万次解码 1 次）；
//! - 结果在下一次 pump（输入之后、rAF 之前）回填尺寸、调 onload / onerror —— 与浏览器一样**总是异步**，
//!   哪怕命中缓存（游戏常在设 src 之后才挂 onload）；
//! - 预算（默认 256MB，`MINI_GAME_IMG_BUDGET_MB`）：超了按 LRU 只淘汰**没有 Image 对象引用**的缓存条目；
//!   剩下的全被引用还超，发一次 `wx.onMemoryWarning`（淘汰被引用的会让 drawImage 画空，不做）；
//! - 改 src 让旧的那次加载作废（代数 gen 对不上的结果丢掉）；src 设成空串 = 放掉引用、不发事件；
//! - JS 对象被回收（FinalizationRegistry）→ `free`，它对解码结果的引用随之放掉。

pub mod loader;
mod natives;

use loader::{Decoded, Loader};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub use natives::register;

const DEFAULT_BUDGET_MB: usize = 256;

struct Obj {
    src: String,
    gen: u64,
    img: Option<Arc<Decoded>>,
}

struct Entry {
    img: Arc<Decoded>,
    used: u64,
}

/// 一次加载的结果，等着交给 JS
pub struct Ready {
    pub id: u32,
    pub result: Result<(u32, u32), String>,
}

pub struct ImageStore {
    root: PathBuf,
    loader: Loader,
    objs: HashMap<u32, Obj>,
    next_id: u32,
    cache: HashMap<String, Entry>,
    /// 在飞的 src → 等它的 (对象, 代数)
    inflight: HashMap<String, Vec<(u32, u64)>>,
    ready: Vec<Ready>,
    tick: u64,
    budget: usize,
    decodes: u64,
    warned: bool,
    warn_pending: bool,
    log: bool,
    logs: Vec<String>,
}

impl ImageStore {
    pub fn new(root: PathBuf) -> Self {
        let mb = std::env::var("MINI_GAME_IMG_BUDGET_MB").ok().and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_BUDGET_MB);
        Self {
            root,
            loader: Loader::new(),
            objs: HashMap::new(),
            next_id: 1,
            cache: HashMap::new(),
            inflight: HashMap::new(),
            ready: Vec::new(),
            tick: 0,
            budget: mb * 1024 * 1024,
            decodes: 0,
            warned: false,
            warn_pending: false,
            log: std::env::var("MINI_IMG_LOG").is_ok(),
            logs: Vec::new(),
        }
    }

    pub fn create(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.objs.insert(id, Obj { src: String::new(), gen: 0, img: None });
        id
    }

    pub fn set_src(&mut self, id: u32, src: &str) {
        let Some(o) = self.objs.get_mut(&id) else { return };
        o.gen += 1;
        // 缓存键：包内路径规范化（`/a.png`、`a.png`、`./x/../a.png` 是同一张，只解一次）；其它来源原样
        let src = &cache_key(src);
        o.src = src.to_string();
        o.img = None;
        if src.is_empty() {
            return;
        }
        let gen = o.gen;
        self.tick += 1;
        if let Some(e) = self.cache.get_mut(src) {
            e.used = self.tick;
            o.img = Some(e.img.clone());
            self.ready.push(Ready { id, result: Ok((e.img.w, e.img.h)) });
            return;
        }
        match self.inflight.get_mut(src) {
            Some(waiters) => waiters.push((id, gen)),
            None => {
                self.inflight.insert(src.to_string(), vec![(id, gen)]);
                self.loader.submit(src, &self.root);
            }
        }
    }

    pub fn free(&mut self, id: u32) {
        self.objs.remove(&id);
    }

    /// 对象当前的图（没加载完 / 失败 / 没设 src 时 None）
    pub fn get(&mut self, id: u32) -> Option<Arc<Decoded>> {
        let o = self.objs.get(&id)?;
        let img = o.img.clone()?;
        self.tick += 1;
        if let Some(e) = self.cache.get_mut(&o.src) {
            e.used = self.tick;
        }
        Some(img)
    }

    /// 收后台线程的结果；`block` 给了就最多等这么久（无头固定时钟下用，见 GameEngine::wait_images）
    pub fn poll(&mut self, block: Option<std::time::Duration>) {
        // 阻塞模式：等到在飞的全部做完（或到期限），于是固定时钟下「设 src → 下一帧 onload」逐次可复现
        let deadline = block.map(|d| std::time::Instant::now() + d);
        loop {
            let d = match deadline {
                Some(t) if !self.inflight.is_empty() => {
                    let left = t.saturating_duration_since(std::time::Instant::now());
                    if left.is_zero() {
                        break;
                    }
                    self.loader.recv_timeout(left)
                }
                _ => self.loader.try_recv(),
            };
            let Some(d) = d else { break };
            let waiters = self.inflight.remove(&d.src).unwrap_or_default();
            match d.result {
                Ok(img) => {
                    self.decodes += 1;
                    if self.log {
                        self.logs.push(format!("🖼 解码 #{} {}×{} {:.1}ms {}", self.decodes, img.w, img.h, d.ms, short(&d.src)));
                    }
                    let img = Arc::new(img);
                    self.tick += 1;
                    self.cache.insert(d.src.clone(), Entry { img: img.clone(), used: self.tick });
                    for (id, gen) in waiters {
                        if let Some(o) = self.objs.get_mut(&id).filter(|o| o.gen == gen) {
                            o.img = Some(img.clone());
                            self.ready.push(Ready { id, result: Ok((img.w, img.h)) });
                        }
                    }
                }
                Err(e) => {
                    if self.log {
                        self.logs.push(format!("🖼 失败 {}：{e}", short(&d.src)));
                    }
                    for (id, gen) in waiters {
                        if self.objs.get(&id).is_some_and(|o| o.gen == gen) {
                            self.ready.push(Ready { id, result: Err(e.clone()) });
                        }
                    }
                }
            }
        }
        self.evict();
    }

    /// 超预算：LRU 淘汰没有对象引用的条目；全被引用还超 → 记一次 memoryWarning（回到预算以内之前不重复发）
    fn evict(&mut self) {
        let mut bytes: usize = self.cache.values().map(|e| e.img.bytes()).sum();
        if bytes <= self.budget {
            self.warned = false;
            return;
        }
        let mut free: Vec<(u64, String)> = self.cache.iter().filter(|(_, e)| Arc::strong_count(&e.img) == 1).map(|(k, e)| (e.used, k.clone())).collect();
        free.sort_unstable();
        for (_, k) in free {
            if bytes <= self.budget {
                break;
            }
            if let Some(e) = self.cache.remove(&k) {
                bytes -= e.img.bytes();
                if self.log {
                    self.logs.push(format!("🖼 淘汰 {}（{}KB）", short(&k), e.img.bytes() / 1024));
                }
            }
        }
        if bytes > self.budget && !self.warned {
            self.warned = true;
            self.logs.push(format!("⚠️ 图片缓存 {}MB 超过预算 {}MB，且都还被 Image 对象引用：发 memoryWarning", bytes >> 20, self.budget >> 20));
            self.warn_pending = true;
        }
    }

    pub fn take_ready(&mut self) -> Vec<Ready> {
        std::mem::take(&mut self.ready)
    }

    /// 要不要发 memoryWarning（取走即清）
    pub fn take_memory_warning(&mut self) -> bool {
        std::mem::take(&mut self.warn_pending)
    }

    pub fn take_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.logs)
    }

    /// 有在飞的加载（宿主据此继续出帧，加载完才能回调 onload）
    pub fn loading(&self) -> bool {
        !self.inflight.is_empty() || !self.ready.is_empty()
    }

    /// 一共真正解码了几次（GT-I4 / I5 的计数）
    pub fn decode_count(&self) -> u64 {
        self.decodes
    }

    /// 缓存里的字节数（诊断 / GT-I6）
    pub fn cached_bytes(&self) -> usize {
        self.cache.values().map(|e| e.img.bytes()).sum()
    }

    /// 测试用：改预算
    pub fn set_budget(&mut self, bytes: usize) {
        self.budget = bytes;
    }
}

fn cache_key(src: &str) -> String {
    if src.is_empty() || src.contains("://") || src.starts_with("data:") {
        return src.to_string();
    }
    // 跑出包根的不规范化：原样交给加载线程，由它报错
    match loader::package_path(src, std::path::Path::new("")) {
        Some(p) => p.to_string_lossy().replace('\\', "/"),
        None => src.to_string(),
    }
}

fn short(src: &str) -> String {
    if src.len() > 80 { format!("{}…", &src[..src.char_indices().nth(60).map(|(i, _)| i).unwrap_or(src.len())]) } else { src.to_string() }
}
