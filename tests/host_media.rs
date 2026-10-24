//! 宿主能力里「文件进出沙盒」那一段的端到端用例：选图 / 选文件 / 存相册（`runtime/app_host.rs` 与
//! `js/prelude/host-media.js`）。每个用例现场写一个最小小程序，用 `MiniEngine` 跑起来，扮演宿主回包。
//!
//! 为什么是独立的集成测试：小程序根目录与数据目录是进程级的（见 `tests/embed_host.rs` 的说明），
//! 而 `wxfile://` 的真实位置正由这两个决定。
use mini_render::compositor::HostEnv;
use mini_render::host::{EngineOptions, MiniEngine};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

/// 一页的小程序：一张 100×100 的图（`p` 有值才显示），`pick(opts)` / `call(name, opts)` 把回调结果写进 data
fn fixture(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("host-media").join(name);
    std::fs::remove_dir_all(&root).ok();
    let js = r#"Page({
        data: { p: '', res: '', err: '' },
        call(name, o) {
            wx[name](Object.assign({}, o || {}, {
                success: (r) => this.setData({ res: JSON.stringify(r), p: (r.tempFilePaths || [])[0] || this.data.p }),
                fail: (r) => this.setData({ err: r.errMsg })
            }));
        }
    });"#;
    let files = [
        ("app.json", r#"{"pages":["pages/p/p"]}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", r#"<image wx:if="{{p}}" src="{{p}}" style="width:100px;height:100px"></image>"#),
        ("pages/p/p.wxss", ""),
        ("pages/p/p.js", js),
        ("pages/p/p.json", "{}"),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn engine(root: &Path, opts: EngineOptions) -> MiniEngine {
    let data = root.join("_data");
    let mut e = MiniEngine::new_with_options(
        &root.to_string_lossy(),
        Some(&data.to_string_lossy()),
        375,
        667,
        1.0,
        HostEnv::default(),
        opts,
    )
    .expect("创建引擎");
    e.launch(None).expect("打开首页");
    e.pump(1);
    e
}

fn opts(apis: &[&str], platform: Option<&str>) -> EngineOptions {
    let mut o = match platform {
        Some(p) => EngineOptions::from_json(&json!({ "systemInfo": { "platform": p } }).to_string()).unwrap(),
        None => EngineOptions::default(),
    };
    o.host_apis = apis.iter().map(|s| s.to_string()).collect();
    o
}

fn host_calls(e: &mut MiniEngine) -> Vec<Value> {
    std::iter::from_fn(|| e.poll_event())
        .map(|s| serde_json::from_str::<Value>(&s).unwrap())
        .filter(|v| v["type"] == "hostCall")
        .collect()
}

fn data(e: &MiniEngine, key: &str) -> String {
    e.eval(&format!("String(getCurrentPages()[0].data.{key})")).unwrap_or_default()
}

fn call(e: &mut MiniEngine, name: &str, o: Value) {
    e.eval(&format!("getCurrentPages()[0].call('{name}', {o})")).unwrap();
    e.pump(17);
}

/// 一张纯红的 PNG
fn red_png(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 0, 0, 255])).save(path).unwrap();
}

fn pixel(e: &mut MiniEngine, x: u32, y: u32) -> [u8; 4] {
    let (w, _) = e.pixel_size();
    let px = e.rgba();
    let i = ((y * w + x) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

#[test]
fn 选图回包里的_file_路径导入沙盒_页面能直接显示() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("import");
    let mut e = engine(&root, opts(&["chooseImage"], None));
    call(&mut e, "chooseImage", json!({ "count": 1 }));
    let c = host_calls(&mut e).pop().expect("要有 hostCall");
    // 参数按官方默认值补齐；桌面（platform 缺省 devtools）不弹「拍照 / 相册」菜单，两种来源原样交给宿主
    assert_eq!(c["args"]["count"], 1);
    assert_eq!(c["args"]["sizeType"], json!(["original", "compressed"]));
    assert_eq!(c["args"]["sourceType"], json!(["album", "camera"]));
    let temp_dir = PathBuf::from(c["tempDir"].as_str().expect("hostCall 要带 tempDir"));
    assert!(temp_dir.is_dir() && temp_dir.starts_with(root.join("_data")), "tempDir={}", temp_dir.display());

    // 宿主把选到的文件放在沙盒外（桌面：用户自己的照片），回 file://
    let src = root.join("_outside").join("红 图.png");
    red_png(&src);
    let uri = format!("file://{}", src.display());
    let id = c["id"].as_u64().unwrap() as u32;
    assert!(e.host_reply(id, true, &json!({ "tempFilePaths": [uri], "tempFiles": [{ "path": uri, "size": 1 }] }).to_string()));
    let p = data(&e, "p");
    assert!(p.starts_with("wxfile://tmp/") && p.ends_with(".png"), "p={p}");
    let res: Value = serde_json::from_str(&data(&e, "res")).unwrap();
    assert_eq!(res["tempFiles"][0]["path"], p.as_str(), "同一个文件出现两次要换成同一个虚拟路径");
    assert_eq!(res["errMsg"], "chooseImage:ok");
    assert!(src.is_file(), "沙盒外的原文件是拷贝，不能被挪走");
    let real = temp_dir.join(p.trim_start_matches("wxfile://tmp/"));
    assert!(real.is_file(), "拷进沙盒的文件应在 tempDir 下：{}", real.display());

    // `<image src="wxfile://…">` 直接画得出来（图在 (0,0) 100×100）
    e.pump(33);
    assert_eq!(pixel(&mut e, 50, 50), [255, 0, 0, 255]);

    // 别的文件 API 同样认它：getImageInfo、FileSystemManager、saveFile（存成 wxfile://store/…）
    let page = "getCurrentPages()[0]";
    e.eval(&format!("wx.getImageInfo({{src: {page}.data.p, success: (r) => {page}.setData({{info: r.width + 'x' + r.height}}), fail: (r) => {page}.setData({{info: r.errMsg}})}})")).unwrap();
    // 回调在 setTimeout(0) 里：第一帧登记定时器、第二帧到点
    e.pump(49);
    e.pump(57);
    assert_eq!(data(&e, "info"), "8x8");
    let b64 = e.eval(&format!("wx.getFileSystemManager().readFileSync({page}.data.p, 'base64').length")).unwrap();
    assert!(b64.parse::<usize>().unwrap() > 0, "readFileSync 读不到导入的文件");
    e.eval(&format!(
        "wx.saveFile({{tempFilePath: {page}.data.p, success: (r) => {page}.setData({{saved: r.savedFilePath}}), fail: (r) => {page}.setData({{saved: r.errMsg}})}})"
    ))
    .unwrap();
    e.pump(65);
    e.pump(73);
    let saved = data(&e, "saved");
    assert!(saved.starts_with("wxfile://store/"), "saveFile 应得到 wxfile://store/…：{saved}");
}

#[test]
fn 宿主直接写进_tempdir_的文件只换名不拷贝() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("tempdir");
    let mut e = engine(&root, opts(&["chooseImage"], None));
    call(&mut e, "chooseImage", json!({}));
    let c = host_calls(&mut e).pop().unwrap();
    let file = PathBuf::from(c["tempDir"].as_str().unwrap()).join("host_1.png");
    red_png(&file);
    // 只给 tempFiles：tempFilePaths 由引擎补
    let reply = json!({ "tempFiles": [{ "path": format!("file://{}", file.display()), "size": 7 }] });
    assert!(e.host_reply(c["id"].as_u64().unwrap() as u32, true, &reply.to_string()));
    assert_eq!(data(&e, "p"), "wxfile://tmp/host_1.png");
    let res: Value = serde_json::from_str(&data(&e, "res")).unwrap();
    assert_eq!(res["tempFiles"], json!([{ "path": "wxfile://tmp/host_1.png", "size": 7 }]));
    let n = std::fs::read_dir(file.parent().unwrap()).unwrap().count();
    assert_eq!(n, 1, "tmp 目录里只该有宿主写的那一个文件（没有再拷一份）");
}

#[test]
fn 回包里斜杠被转义的_file_路径也导入() {
    // Android 的 org.json、iOS 的 JSONSerialization 默认把 `/` 写成 `\/`：原生 Android 视图的回包原文是
    // `"file:\/\/\/data\/…"`。从前按字面量 `file://` 筛，这种回包整个跳过、小程序拿到的是 file:// 路径，
    // `<image>` 显示不出来（模拟器上选完图只剩占位图）
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("escaped");
    let mut e = engine(&root, opts(&["chooseImage"], None));
    call(&mut e, "chooseImage", json!({ "count": 1 }));
    let c = host_calls(&mut e).pop().expect("要有 hostCall");
    let file = PathBuf::from(c["tempDir"].as_str().unwrap()).join("host_2.png");
    red_png(&file);
    let uri = format!("file://{}", file.display()).replace('/', "\\/");
    let reply = format!(r#"{{"tempFilePaths":["{uri}"],"tempFiles":[{{"path":"{uri}","size":7}}]}}"#);
    assert!(reply.contains(r"file:\/\/"), "{reply}");
    assert!(e.host_reply(c["id"].as_u64().unwrap() as u32, true, &reply));
    assert_eq!(data(&e, "p"), "wxfile://tmp/host_2.png");
}

#[test]
fn 移动端两种来源先问拍照还是相册_取消走_fail() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("sheet");
    let mut e = engine(&root, opts(&["chooseImage", "chooseMedia"], Some("android")));
    call(&mut e, "chooseImage", json!({ "count": 3 }));
    assert!(host_calls(&mut e).is_empty(), "先弹操作菜单，选完才交给宿主");
    // 点「从手机相册选择」（第 2 项）
    e.eval("__resolveActionSheet(__asSeq, 1)").unwrap();
    e.pump(33);
    let c = host_calls(&mut e).pop().expect("选完来源要发 hostCall");
    assert_eq!(c["args"]["sourceType"], json!(["album"]));
    assert_eq!(c["args"]["count"], 3);

    // 取消菜单：`chooseMedia:fail cancel`，宿主什么都收不到
    call(&mut e, "chooseMedia", json!({}));
    e.eval("__resolveActionSheet(__asSeq, -1)").unwrap();
    e.pump(49);
    e.pump(65);
    assert!(host_calls(&mut e).is_empty());
    assert_eq!(data(&e, "err"), "chooseMedia:fail cancel");

    // 只给一种来源：不问
    call(&mut e, "chooseMedia", json!({ "sourceType": ["camera"], "mediaType": ["video"], "maxDuration": 99 }));
    let c = host_calls(&mut e).pop().expect("只有相机时直接交给宿主");
    assert_eq!(c["args"]["sourceType"], json!(["camera"]));
    assert_eq!(c["args"]["maxDuration"], 60, "maxDuration 夹到 3~60");
    assert_eq!(c["args"]["camera"], "back");
}

#[test]
fn 回包按官方形状补齐_参数不对不发给宿主() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("shape");
    let mut e = engine(&root, opts(&["chooseMedia", "chooseMessageFile", "chooseVideo"], None));

    call(&mut e, "chooseMedia", json!({}));
    let c = host_calls(&mut e).pop().unwrap();
    let reply = json!({ "tempFiles": [{ "path": "a.jpg", "size": 3 }, { "tempFilePath": "b.mp4", "duration": 2.5 }] });
    e.host_reply(c["id"].as_u64().unwrap() as u32, true, &reply.to_string());
    let res: Value = serde_json::from_str(&data(&e, "res")).unwrap();
    assert_eq!(res["type"], "mix");
    assert_eq!(res["tempFiles"][0]["tempFilePath"], "a.jpg");
    assert_eq!(res["tempFiles"][0]["fileType"], "image");
    assert_eq!(res["tempFiles"][1]["fileType"], "video");
    assert_eq!(res["tempFiles"][1]["duration"], 2.5);
    assert_eq!(res["tempFiles"][1]["thumbTempFilePath"], "");

    // chooseMessageFile 的 count 是必填：不给就是参数错误，宿主收不到
    call(&mut e, "chooseMessageFile", json!({}));
    e.pump(33);
    assert!(host_calls(&mut e).is_empty());
    assert_eq!(
        data(&e, "err"),
        "chooseMessageFile:fail parameter error: parameter.count should be Number instead of Undefined;"
    );
    call(&mut e, "chooseMessageFile", json!({ "count": 200, "type": "file", "extension": [".pdf", ""] }));
    let c = host_calls(&mut e).pop().unwrap();
    assert_eq!((c["args"]["count"].as_u64(), c["args"]["extension"].clone()), (Some(100), json!(["pdf"])));
    let src = root.join("_outside").join("报告.pdf");
    std::fs::create_dir_all(src.parent().unwrap()).unwrap();
    std::fs::write(&src, b"%PDF-1.4").unwrap();
    let reply = json!({ "tempFiles": [{ "path": format!("file://{}", src.display()), "size": 8, "name": "报告.pdf" }] });
    e.host_reply(c["id"].as_u64().unwrap() as u32, true, &reply.to_string());
    let res: Value = serde_json::from_str(&data(&e, "res")).unwrap();
    let f = &res["tempFiles"][0];
    assert!(f["path"].as_str().unwrap().starts_with("wxfile://tmp/") && f["path"].as_str().unwrap().ends_with(".pdf"));
    assert_eq!((f["name"].as_str(), f["type"].as_str(), f["size"].as_u64()), (Some("报告.pdf"), Some("file"), Some(8)));
    assert!(f["time"].as_u64().unwrap() > 1_600_000_000);

    // chooseVideo：宿主回 tempFiles[0] 也认，拍平成官方的单文件形状
    call(&mut e, "chooseVideo", json!({}));
    let c = host_calls(&mut e).pop().unwrap();
    assert_eq!(c["args"]["compressed"], true);
    e.host_reply(c["id"].as_u64().unwrap() as u32, true, r#"{"tempFiles":[{"path":"v.mp4","duration":3,"width":640,"height":360}]}"#);
    let res: Value = serde_json::from_str(&data(&e, "res")).unwrap();
    assert_eq!(res, json!({ "errMsg": "chooseVideo:ok", "tempFilePath": "v.mp4", "duration": 3, "size": 0, "width": 640, "height": 360 }));
}

#[test]
fn 存相册的_hostcall_带文件真实路径() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("save");
    let mut e = engine(&root, opts(&["chooseImage", "saveImageToPhotosAlbum"], None));
    // 先选一张进沙盒
    call(&mut e, "chooseImage", json!({}));
    let c = host_calls(&mut e).pop().unwrap();
    let src = root.join("_outside").join("a.png");
    red_png(&src);
    e.host_reply(c["id"].as_u64().unwrap() as u32, true, &json!({ "tempFilePaths": [format!("file://{}", src.display())] }).to_string());
    let p = data(&e, "p");
    assert!(p.starts_with("wxfile://tmp/"));

    call(&mut e, "saveImageToPhotosAlbum", json!({ "filePath": p }));
    let c = host_calls(&mut e).pop().expect("要发 hostCall");
    let real = c["files"][p.as_str()].as_str().expect("files 里要有它的真实路径");
    assert_eq!(std::fs::read(real).unwrap(), std::fs::read(&src).unwrap());

    // 不存在的虚拟路径不列进 files；filePath 不给就是参数错误
    call(&mut e, "saveImageToPhotosAlbum", json!({ "filePath": "wxfile://tmp/nope.png" }));
    let c = host_calls(&mut e).pop().unwrap();
    assert!(c.get("files").is_none(), "{c}");
    call(&mut e, "saveImageToPhotosAlbum", json!({}));
    e.pump(33);
    assert!(host_calls(&mut e).is_empty());
    assert_eq!(
        data(&e, "err"),
        "saveImageToPhotosAlbum:fail parameter error: parameter.filePath should be String instead of Undefined;"
    );
}

#[test]
fn 沙盒里的_file_路径_saveFile_也认_沙盒外的不认() {
    // 宿主 / 旧版引擎原样交来 `file://<沙盒里>`：saveFile 照样存得下（从前报 no such file or directory, open file:///…）
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("fileuri");
    let mut e = engine(&root, opts(&["chooseImage"], None));
    call(&mut e, "chooseImage", json!({}));
    let c = host_calls(&mut e).pop().unwrap();
    let file = PathBuf::from(c["tempDir"].as_str().unwrap()).join("host_9.png");
    red_png(&file);
    let page = "getCurrentPages()[0]";
    let save = |e: &mut MiniEngine, p: &str| {
        e.eval(&format!(
            "wx.saveFile({{tempFilePath: {p:?}, success: (r) => {page}.setData({{saved: r.savedFilePath}}), fail: (r) => {page}.setData({{saved: r.errMsg}})}})"
        ))
        .unwrap();
        e.pump(81);
        e.pump(89);
        data(e, "saved")
    };
    let saved = save(&mut e, &format!("file://{}", file.display()));
    assert!(saved.starts_with("wxfile://store/"), "{saved}");
    let outside = root.join("_outside").join("x.png");
    red_png(&outside);
    let saved = save(&mut e, &format!("file://{}", outside.display()));
    assert!(saved.contains("fail"), "沙盒外的 file:// 不能认：{saved}");
}

#[test]
fn 大图后台解码_不卡渲染线程_几帧后画出来() {
    // 选图的原图动辄 5MB：同步解码在一加上一张 300ms+、三张就卡一秒（页面「超卡」的来源）。
    // 超过 768KB 的本地图后台解码：第一帧画占位、解码完下一帧画出来；长边超 2048 的缩小
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("bigimg");
    let mut e = engine(&root, opts(&["chooseImage"], None));
    call(&mut e, "chooseImage", json!({ "count": 1 }));
    let c = host_calls(&mut e).pop().unwrap();
    // 不压缩的 BMP：2400×600×3 ≈ 4.3MB，纯红
    let file = PathBuf::from(c["tempDir"].as_str().unwrap()).join("host_big.bmp");
    image::RgbImage::from_pixel(2400, 600, image::Rgb([255, 0, 0])).save(&file).unwrap();
    assert!(std::fs::metadata(&file).unwrap().len() > 768 * 1024);
    let uri = format!("file://{}", file.display());
    let t0 = std::time::Instant::now();
    assert!(e.host_reply(c["id"].as_u64().unwrap() as u32, true, &json!({ "tempFiles": [{ "path": uri, "size": 1 }] }).to_string()));
    e.pump(33);
    assert!(t0.elapsed().as_millis() < 150, "大图不该在渲染线程上同步解码：{:?}", t0.elapsed());
    let mut red = false;
    for i in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(20));
        e.pump(49 + i * 16);
        if pixel(&mut e, 50, 50) == [255, 0, 0, 255] {
            red = true;
            break;
        }
    }
    assert!(red, "后台解码完要画出来：{:?}", pixel(&mut e, 50, 50));
}
