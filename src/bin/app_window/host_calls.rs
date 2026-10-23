//! 桌面窗体自己接住的宿主能力：选图 / 选视频 / 选文件 / 存相册（hostCall，见 `runtime/app_host.rs`）。
//!
//! - 交互窗体：系统的文件对话框（rfd，macOS / Windows），选完回 `file://<路径>`，引擎把文件**拷**进沙盒
//!   （用户自己的照片不能被挪走、也不能被小程序改到）；
//! - 无头 / 自动化：`MINI_PICK_FILES=/a.png,/b.jpg` 当作用户选了这些（不弹框），`MINI_SAVE_DIR=<目录>`
//!   存相册时拷进这个目录。没给时无头一律 `<api>:fail cancel` —— 弹一个没人点的对话框会把脚本卡死；
//! - 桌面没有相机：`sourceType` 只有 camera 也开文件对话框（微信 PC 版就是这样），
//!   「拍照 / 从相册选」的菜单也不弹（host-media.js 按 platform 判断，桌面是 devtools）。
//!
//! 对话框是模态的，会在里面转一个嵌套的事件循环，所以**不在出帧路径里弹**：
//! `capsule_sync_host_events`（出帧入口）只把调用记下，`about_to_wait` 再处理（[`MiniAppWindow::run_host_calls`]）。
//! 视频的时长 / 宽高桌面给 0（引擎里没有现成的 mp4 元信息探测，播放器自己会读）。
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// 桌面窗体点名的宿主 API（`MiniApp::install_host_apis`，在 app.js 执行之前）
pub const DESKTOP_HOST_APIS: &[&str] = &[
    "chooseImage",
    "chooseMedia",
    "chooseVideo",
    "chooseMessageFile",
    "saveImageToPhotosAlbum",
    "saveVideoToPhotosAlbum",
];

const IMAGE_EXT: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp"];
const VIDEO_EXT: &[&str] = &["mp4", "mov", "m4v", "webm"];

/// 还没处理的 hostCall（出帧时记下，事件循环空下来再弹对话框）
#[derive(Default)]
pub struct HostCalls {
    pending: Vec<Value>,
}

pub fn desktop_host_apis() -> Vec<String> {
    DESKTOP_HOST_APIS.iter().map(|s| s.to_string()).collect()
}

/// 选什么：对话框的过滤器与标题
struct PickSpec {
    title: &'static str,
    filters: Vec<(&'static str, Vec<String>)>,
    count: usize,
}

impl crate::MiniAppWindow {
    /// `capsule_sync_host_events` 收到 hostCall：无头立刻答（不弹框），交互窗体等 about_to_wait
    pub(crate) fn queue_host_call(&mut self, call: Value) {
        self.host_calls.pending.push(call);
        if self.window.is_none() {
            self.run_host_calls();
        }
    }

    /// 处理记下的 hostCall（交互窗体在 about_to_wait 里调：这时不在出帧中途，可以弹模态对话框）
    pub(crate) fn run_host_calls(&mut self) {
        if self.host_calls.pending.is_empty() {
            return;
        }
        for call in std::mem::take(&mut self.host_calls.pending) {
            let id = call["id"].as_u64().unwrap_or(0) as u32;
            let api = call["api"].as_str().unwrap_or("").to_string();
            let (ok, res) = match self.answer(&api, &call) {
                Ok(v) => (true, v),
                Err(msg) => (false, json!({ "errMsg": format!("{api}:fail {msg}") })),
            };
            println!("🧰 hostCall {api} → {}", if ok { "ok" } else { res["errMsg"].as_str().unwrap_or("") });
            self.app.resolve_host_call(id, ok, &res.to_string());
        }
        // 回调里多半 setData 了：下一帧取数据重画（take_data_dirty 在出帧入口）
        self.needs_redraw = true;
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn answer(&self, api: &str, call: &Value) -> Result<Value, String> {
        let args = &call["args"];
        match api {
            "chooseImage" | "chooseMedia" | "chooseVideo" | "chooseMessageFile" => {
                let spec = pick_spec(api, args);
                let picked = self.pick(&spec)?;
                Ok(pick_result(api, &picked))
            }
            "saveImageToPhotosAlbum" | "saveVideoToPhotosAlbum" => {
                let virt = args["filePath"].as_str().unwrap_or("");
                let real = call["files"][virt].as_str().map(PathBuf::from).ok_or("file not found")?;
                let name = real.file_name().and_then(|n| n.to_str()).unwrap_or("image").to_string();
                let dst = self.save_target(&name)?;
                std::fs::copy(&real, &dst).map_err(|e| e.to_string())?;
                println!("💾 已存到 {}", dst.display());
                Ok(json!({}))
            }
            _ => Err("桌面窗体没有实现".into()),
        }
    }

    /// 选文件：`MINI_PICK_FILES` → 无头取消 → 系统对话框
    fn pick(&self, spec: &PickSpec) -> Result<Vec<PathBuf>, String> {
        let mut files: Vec<PathBuf> = if let Ok(list) = std::env::var("MINI_PICK_FILES") {
            list.split(',').map(str::trim).filter(|s| !s.is_empty()).map(PathBuf::from).filter(|p| p.is_file()).collect()
        } else if self.window.is_none() {
            return Err("cancel".into());
        } else {
            self.dialog_pick(spec)?
        };
        files.truncate(spec.count.max(1));
        if files.is_empty() {
            return Err("cancel".into());
        }
        Ok(files)
    }

    fn save_target(&self, name: &str) -> Result<PathBuf, String> {
        if let Ok(dir) = std::env::var("MINI_SAVE_DIR") {
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            return Ok(Path::new(&dir).join(name));
        }
        if self.window.is_none() {
            return Err("cancel".into());
        }
        self.dialog_save(name)
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    fn dialog_pick(&self, spec: &PickSpec) -> Result<Vec<PathBuf>, String> {
        let mut d = rfd::FileDialog::new().set_title(spec.title);
        for (label, exts) in &spec.filters {
            d = d.add_filter(*label, exts);
        }
        if let Some(w) = &self.window {
            d = d.set_parent(w.as_ref());
        }
        let r = if spec.count > 1 { d.pick_files() } else { d.pick_file().map(|p| vec![p]) };
        r.ok_or_else(|| "cancel".into())
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    fn dialog_save(&self, name: &str) -> Result<PathBuf, String> {
        let mut d = rfd::FileDialog::new().set_title("存到本机").set_file_name(name);
        if let Some(w) = &self.window {
            d = d.set_parent(w.as_ref());
        }
        d.save_file().ok_or_else(|| "cancel".into())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    fn dialog_pick(&self, _spec: &PickSpec) -> Result<Vec<PathBuf>, String> {
        Err("当前桌面平台没有文件对话框（用 MINI_PICK_FILES 指定文件）".into())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    fn dialog_save(&self, _name: &str) -> Result<PathBuf, String> {
        Err("当前桌面平台没有文件对话框（用 MINI_SAVE_DIR 指定目录）".into())
    }
}

/// 按 API 与参数（host-media.js 已补齐默认值）定过滤器与张数
fn pick_spec(api: &str, args: &Value) -> PickSpec {
    let exts = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let count = args["count"].as_u64().unwrap_or(1) as usize;
    match api {
        "chooseImage" => PickSpec { title: "选择图片", filters: vec![("图片", exts(IMAGE_EXT))], count },
        "chooseVideo" => PickSpec { title: "选择视频", filters: vec![("视频", exts(VIDEO_EXT))], count: 1 },
        "chooseMedia" => {
            let types: Vec<&str> = args["mediaType"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
            let (img, vid) = (types.iter().any(|t| *t == "image" || *t == "mix"), types.iter().any(|t| *t == "video" || *t == "mix"));
            let mut filters = Vec::new();
            if img && vid {
                filters.push(("图片和视频", IMAGE_EXT.iter().chain(VIDEO_EXT).map(|s| s.to_string()).collect()));
            }
            if img || !vid {
                filters.push(("图片", exts(IMAGE_EXT)));
            }
            if vid {
                filters.push(("视频", exts(VIDEO_EXT)));
            }
            PickSpec { title: "选择图片或视频", filters, count }
        }
        _ => {
            // chooseMessageFile：type 决定过滤器；extension 只在 type 为 file 时有效（官方语义）
            let filters = match args["type"].as_str().unwrap_or("all") {
                "image" => vec![("图片", exts(IMAGE_EXT))],
                "video" => vec![("视频", exts(VIDEO_EXT))],
                "file" => {
                    let list: Vec<String> = args["extension"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
                    if list.is_empty() { vec![] } else { vec![("文件", list)] }
                }
                _ => vec![],
            };
            PickSpec { title: "选择文件", filters, count }
        }
    }
}

/// 选到的文件 → 回包（路径写成 `file://`，引擎拷进沙盒后换成 `wxfile://tmp/…`；其余字段 host-media.js 补齐）
fn pick_result(api: &str, files: &[PathBuf]) -> Value {
    let uri = |p: &Path| format!("file://{}", p.display());
    let size = |p: &Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    let is_video = |p: &Path| {
        p.extension().and_then(|e| e.to_str()).map(|e| VIDEO_EXT.contains(&e.to_ascii_lowercase().as_str())).unwrap_or(false)
    };
    let dims = |p: &Path| image::image_dimensions(p).unwrap_or((0, 0));
    match api {
        "chooseImage" => json!({
            "tempFilePaths": files.iter().map(|p| uri(p)).collect::<Vec<_>>(),
            "tempFiles": files.iter().map(|p| json!({ "path": uri(p), "size": size(p) })).collect::<Vec<_>>(),
        }),
        "chooseMedia" => json!({
            "tempFiles": files.iter().map(|p| {
                let (w, h) = if is_video(p) { (0, 0) } else { dims(p) };
                json!({ "tempFilePath": uri(p), "size": size(p), "fileType": if is_video(p) { "video" } else { "image" }, "width": w, "height": h })
            }).collect::<Vec<_>>(),
        }),
        "chooseVideo" => json!({ "tempFilePath": uri(&files[0]), "size": size(&files[0]) }),
        _ => json!({
            "tempFiles": files.iter().map(|p| {
                let time = std::fs::metadata(p).and_then(|m| m.modified()).ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
                json!({
                    "path": uri(p),
                    "size": size(p),
                    "name": p.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                    "time": time,
                })
            }).collect::<Vec<_>>(),
        }),
    }
}
