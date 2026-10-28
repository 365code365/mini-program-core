//! 图片的读取与解码（后台线程，game-doc/02 §5.3）。
//!
//! 两条工作线程从一个队列取活：读字节（包内文件 / `wxfile://` / `http(s)` / `data:` URI）→ `image` 解码成 RGBA8。
//! 为什么在后台：一张 1024² 的 PNG 在一加 PLY110 上解码 20~40ms，放在引擎线程上就是一次掉帧。
//! 去重（同一 src 同时只解一次）在 store.rs，这里只管「给一个 src，交回一份结果」。
//!
//! 包内路径一律按包根解析：`/a.png`、`a.png`、`./a.png` 相同；带 `..` 跑出包根的拒绝
//! （小程序那份 `assets::resolve` 会回落到进程工作目录与内置示例，游戏不要这个）。

use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 解码结果：非预乘 RGBA8
pub struct Decoded {
    pub w: u32,
    pub h: u32,
    pub rgba: Arc<Vec<u8>>,
    /// drawImage 1:1 快路的分段表，第一次画的时候算（image2d::RowSpans）
    pub spans: std::sync::OnceLock<crate::renderer::components::canvas_image::RowSpans>,
}

impl Decoded {
    pub fn spans(&self) -> &crate::renderer::components::canvas_image::RowSpans {
        self.spans.get_or_init(|| crate::renderer::components::canvas_image::RowSpans::build(&self.rgba, self.w, self.h))
    }

    pub fn bytes(&self) -> usize {
        self.rgba.len()
    }
}

pub struct Done {
    pub src: String,
    pub result: Result<Decoded, String>,
    pub ms: f64,
}

/// 网络图的上限：下载超过它直接失败（防止一张图把内存吃光）
const MAX_NET_BYTES: u64 = 32 * 1024 * 1024;
/// 解码后的单边上限（与画布一致）：超过的拒绝，否则一张 20000² 的图解出来 1.6GB
const MAX_SIDE: u32 = 8192;
const WORKERS: usize = 2;

enum Job {
    Load { src: String, root: PathBuf },
}

pub struct Loader {
    tx: Option<Sender<Job>>,
    rx: Receiver<Done>,
}

impl Loader {
    pub fn new() -> Self {
        let (tx, jobs) = channel::<Job>();
        let (done_tx, rx) = channel::<Done>();
        let jobs = Arc::new(Mutex::new(jobs));
        for i in 0..WORKERS {
            let jobs = jobs.clone();
            let done_tx = done_tx.clone();
            let _ = std::thread::Builder::new().name(format!("mini-game-img{i}")).spawn(move || loop {
                // 队列关了（实例销毁）就退出
                let job = match jobs.lock().map(|r| r.recv()) {
                    Ok(Ok(j)) => j,
                    _ => return,
                };
                let Job::Load { src, root } = job;
                let t = Instant::now();
                // 畸形文件让解码器 panic（GT-X4）：只算这一张失败，线程照常接下一张
                let result = std::panic::catch_unwind(|| load(&src, &root))
                    .unwrap_or_else(|_| Err("图片解码失败：解码器内部错误".to_string()));
                let ms = t.elapsed().as_secs_f64() * 1000.0;
                if done_tx.send(Done { src, result, ms }).is_err() {
                    return;
                }
            });
        }
        Self { tx: Some(tx), rx }
    }

    pub fn submit(&self, src: &str, root: &Path) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Job::Load { src: src.to_string(), root: root.to_path_buf() });
        }
    }

    pub fn try_recv(&self) -> Option<Done> {
        self.rx.try_recv().ok()
    }

    /// 等一份结果（无头测试用：固定时钟下帧与帧之间不真等，解码线程可能还没做完）
    pub fn recv_timeout(&self, d: Duration) -> Option<Done> {
        self.rx.recv_timeout(d).ok()
    }
}

impl Drop for Loader {
    fn drop(&mut self) {
        // 关掉队列：工作线程的 recv 返回 Err 后退出（正在解的那一张做完再走，不 join，不卡销毁）
        self.tx.take();
    }
}

fn load(src: &str, root: &Path) -> Result<Decoded, String> {
    let bytes = read(src, root)?;
    let img = image::load_from_memory(&bytes).map_err(|e| format!("图片解码失败：{e}"))?;
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE {
        return Err(format!("图片尺寸 {w}×{h} 超出范围（单边 1~{MAX_SIDE}）"));
    }
    // 动图（GIF / APNG / WebP）：load_from_memory 只给第一帧（Canvas 规范的 drawImage 也是第一帧）
    Ok(Decoded { w, h, rgba: Arc::new(img.to_rgba8().into_raw()), spans: Default::default() })
}

fn read(src: &str, root: &Path) -> Result<Vec<u8>, String> {
    if let Some(rest) = src.strip_prefix("data:") {
        return data_uri(rest);
    }
    if src.starts_with("http://") || src.starts_with("https://") {
        return http(src);
    }
    let path = if src.starts_with(crate::wxfile::SCHEME) {
        crate::wxfile::to_path(src).ok_or_else(|| format!("无效的路径：{src}"))?
    } else {
        package_path(src, root).ok_or_else(|| format!("路径不能跑出包根：{src}"))?
    };
    std::fs::read(&path).map_err(|e| format!("读不出 {src}：{e}"))
}

/// 包内路径 → 真实路径；`..` 跑出包根返回 None
pub fn package_path(src: &str, root: &Path) -> Option<PathBuf> {
    let rel = src.trim_start_matches('/');
    let mut out = root.to_path_buf();
    let mut depth = 0i32;
    for c in Path::new(rel).components() {
        match c {
            Component::Normal(p) => {
                out.push(p);
                depth += 1;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
                out.pop();
            }
            _ => return None,
        }
    }
    Some(out)
}

fn data_uri(rest: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    let (meta, payload) = rest.split_once(',').ok_or("data URI 缺逗号")?;
    if meta.ends_with(";base64") {
        // 有的游戏把换行 / 空格也塞进来：先去掉
        let clean: String = payload.chars().filter(|c| !c.is_ascii_whitespace()).collect();
        base64::engine::general_purpose::STANDARD.decode(clean.as_bytes()).map_err(|e| format!("data URI 的 base64 无效：{e}"))
    } else {
        Ok(percent_decode(payload))
    }
}

fn percent_decode(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

fn http(url: &str) -> Result<Vec<u8>, String> {
    let config = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(30))).max_redirects(5).build();
    let agent = ureq::Agent::new_with_config(config);
    let resp = agent.get(url).call().map_err(|e| format!("下载失败：{e}"))?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!("下载失败：HTTP {status}"));
    }
    let mut body = resp.into_body();
    body.with_config().limit(MAX_NET_BYTES).read_to_vec().map_err(|e| format!("下载失败：{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 包内路径三种写法相同_跑出包根拒绝() {
        let root = Path::new("/pkg");
        assert_eq!(package_path("/img/a.png", root), Some(PathBuf::from("/pkg/img/a.png")));
        assert_eq!(package_path("img/a.png", root), Some(PathBuf::from("/pkg/img/a.png")));
        assert_eq!(package_path("./img/../img/a.png", root), Some(PathBuf::from("/pkg/img/a.png")));
        assert_eq!(package_path("../secret.png", root), None);
        assert_eq!(package_path("img/../../x", root), None);
    }

    #[test]
    fn data_uri_两种编码() {
        assert_eq!(data_uri("image/png;base64,AAEC").unwrap(), vec![0, 1, 2]);
        assert_eq!(data_uri("text/plain,a%20b").unwrap(), b"a b".to_vec());
        assert!(data_uri("image/png;base64").is_err());
    }
}
