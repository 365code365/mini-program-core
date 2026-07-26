//! Rust 原生渲染与 HTML/Chrome 输出的一致性批量对比工具。
//!
//! 默认比较 sample-app 的全部页面：
//!   cargo run --example compare
//! 比较指定页面（可传多个路由）：
//!   cargo run --example compare -- pages/detail/detail pages/components/components
//! 常用选项：
//!   cargo run --example compare -- --all --out target/render-compare
//!   cargo run --example compare -- --browser "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
//!
//! 每个页面输出 rust.png、html.png、side-by-side.png、diff.png，根目录输出 report.json。
//! 两端固定使用 375x667 CSS 像素视口和 2 倍设备像素比，确保 fixed 元素、裁剪和滚动首屏可比较。

use image::{Rgba, RgbaImage};
use mini_render::compiler::html::HtmlTarget;
use mini_render::compiler::{
    load_app_source, write_files, AppSource, CompileTarget, PageSource, TabBarSource,
};
use mini_render::parser::wxml::{WxmlNode, WxmlNodeType};
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::WxmlRenderer;
use mini_render::{Canvas, Color};
use serde::Serialize;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const VIEWPORT_WIDTH: u32 = 375;
const VIEWPORT_HEIGHT: u32 = 667;
const DEVICE_SCALE: u32 = 2;
// 外壳窗口宽度：需明显超过 Chrome headless 的最小窗口宽度，令内嵌 375px iframe 不受影响。
// iframe 自身建立精确 375px 视口，position:fixed 因而相对 375 解析（与原生一致），
// 规避了直接加载时 Chrome 用 ~510px 布局视口导致的固定层居中偏移。
const CAPTURE_SHELL_WIDTH: u32 = 900;
const DEFAULT_PIXEL_THRESHOLD: u8 = 16;
// 给足虚拟时间预算，确保被遮挡/下方的图片也在截图前解码完成（headless 下 Chrome 会
// 按可见优先级延迟加载被遮挡图片，预算过小会拍到 broken-image 占位图）。
const DEFAULT_SETTLE_MS: u64 = 2_500;

#[derive(Debug)]
struct Config {
    app_root: PathBuf,
    html_root: PathBuf,
    output_root: PathBuf,
    browser: Option<PathBuf>,
    routes: Vec<String>,
    all: bool,
    compile_html: bool,
    pixel_threshold: u8,
    settle_ms: u64,
    /// 直接复用外部已渲染好的原生截图目录（`<dir>/<route>/rust.png`）。
    /// 用于让「窗体宿主实际出的帧」而不是示例自己的渲染参与对比。
    rust_from: Option<PathBuf>,
}

#[derive(Serialize)]
struct ComparisonReport {
    format_version: u32,
    generated_at_unix: u64,
    app_root: String,
    html_root: String,
    output_root: String,
    browser: String,
    viewport: ViewportReport,
    pixel_threshold: u8,
    pages: Vec<PageReport>,
    summary: SummaryReport,
}

#[derive(Serialize)]
struct ViewportReport {
    css_width: u32,
    css_height: u32,
    device_scale_factor: u32,
    expected_pixel_width: u32,
    expected_pixel_height: u32,
}

#[derive(Serialize)]
struct PageReport {
    route: String,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    outputs: OutputReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    rust_image: Option<ImageReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_image: Option<ImageReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    difference: Option<DiffMetrics>,
}

#[derive(Serialize)]
struct OutputReport {
    rust_png: String,
    html_png: String,
    side_by_side_png: String,
    diff_png: String,
}

#[derive(Serialize)]
struct ImageReport {
    width: u32,
    height: u32,
}

#[derive(Serialize)]
struct DiffMetrics {
    compared_width: u32,
    compared_height: u32,
    total_pixels: u64,
    exact_changed_pixels: u64,
    exact_changed_ratio: f64,
    changed_pixels: u64,
    changed_ratio: f64,
    mean_absolute_error: f64,
    root_mean_square_error: f64,
    max_channel_error: u8,
    dimensions_match: bool,
}

#[derive(Serialize)]
struct SummaryReport {
    requested_pages: usize,
    compared_pages: usize,
    failed_pages: usize,
    dimension_mismatches: usize,
    total_pixels: u64,
    changed_pixels: u64,
    changed_ratio: f64,
    mean_absolute_error: f64,
    max_channel_error: u8,
}

struct StaticServer {
    address: String,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl StaticServer {
    fn start(root: &Path) -> Result<Self, String> {
        let root = fs::canonicalize(root)
            .map_err(|error| format!("无法读取 HTML 输出目录 {}: {error}", root.display()))?;
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("无法启动本地静态服务器: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("无法配置本地静态服务器: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("无法获取本地静态服务器地址: {error}"))?
            .to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        // 关键：macOS/BSD 上从非阻塞 listener accept 出来的连接会继承
                        // O_NONBLOCK，直接 read 往往返回 WouldBlock，导致请求被当作失败
                        // 丢弃、样式表随机加载不到（表现为截图偶发丢样式）。这里显式改回阻塞。
                        let _ = stream.set_nonblocking(false);
                        let request_root = root.clone();
                        thread::spawn(move || {
                            let _ = serve_connection(stream, &request_root);
                        });
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            address,
            stop,
            worker: Some(worker),
        })
    }

    fn capture_url(&self, route: &str) -> String {
        // 外壳页：内嵌一个精确 375x667 的 iframe 承载真实页面，令 iframe 建立 375px 视口，
        // position:fixed 相对 375 解析；页面内脚本被服务器禁用，静态首屏无 JS 重渲染竞态。
        format!("http://{}/__mini_shell__/{}", self.address, route)
    }
}

impl Drop for StaticServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(&self.address);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("compare 失败: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let Some(config) = parse_args()? else {
        print_usage();
        return Ok(());
    };

    let app_root = path_text(&config.app_root);
    let html_root = path_text(&config.html_root);
    let output_root = path_text(&config.output_root);

    println!("加载小程序: {app_root}");
    let app = load_app_source(&app_root)?;
    let selected_pages = select_pages(&app, &config)?;
    if selected_pages.is_empty() {
        return Err("没有可比较的页面".to_string());
    }

    if config.compile_html {
        println!("重新编译 HTML: {html_root}");
        let files = HtmlTarget::new().compile(&app)?;
        write_files(&html_root, &files)?;
    } else if !config.html_root.join("index.html").is_file() {
        return Err(format!(
            "--no-compile 指定的目录没有 index.html: {html_root}"
        ));
    }

    fs::create_dir_all(&config.output_root)
        .map_err(|error| format!("创建输出目录 {output_root} 失败: {error}"))?;
    let browser = resolve_browser(config.browser.as_deref())?;
    println!("Chrome/Chromium: {}", browser.display());
    let server = StaticServer::start(&config.html_root)?;

    let profile_dir = config.output_root.join(".chrome-profile");
    if profile_dir.exists() {
        let _ = fs::remove_dir_all(&profile_dir);
    }
    fs::create_dir_all(&profile_dir)
        .map_err(|error| format!("创建 Chrome 临时目录失败: {error}"))?;

    let mut page_reports = Vec::with_capacity(selected_pages.len());
    for (index, page) in selected_pages.iter().enumerate() {
        println!("[{}/{}] 对比 {}", index + 1, selected_pages.len(), page.route);
        page_reports.push(compare_page(
            &config,
            &app,
            page,
            &browser,
            &profile_dir,
            &server.capture_url(&page.route),
        ));
    }
    drop(server);
    let _ = fs::remove_dir_all(&profile_dir);

    let summary = summarize(&page_reports);
    let failed_pages = summary.failed_pages;
    let report = ComparisonReport {
        format_version: 1,
        generated_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        app_root,
        html_root,
        output_root: output_root.clone(),
        browser: path_text(&browser),
        viewport: ViewportReport {
            css_width: VIEWPORT_WIDTH,
            css_height: VIEWPORT_HEIGHT,
            device_scale_factor: DEVICE_SCALE,
            expected_pixel_width: VIEWPORT_WIDTH * DEVICE_SCALE,
            expected_pixel_height: VIEWPORT_HEIGHT * DEVICE_SCALE,
        },
        pixel_threshold: config.pixel_threshold,
        pages: page_reports,
        summary,
    };

    let report_path = config.output_root.join("report.json");
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("序列化差异报告失败: {error}"))?;
    fs::write(&report_path, format!("{json}\n"))
        .map_err(|error| format!("写入 {} 失败: {error}", report_path.display()))?;

    println!(
        "完成: {} 个页面，{} 个失败，阈值差异像素 {:.2}%",
        report.summary.compared_pages,
        report.summary.failed_pages,
        report.summary.changed_ratio * 100.0
    );
    println!("报告: {}", report_path.display());

    if failed_pages > 0 {
        Err(format!(
            "{failed_pages} 个页面未能完成对比；详情见 {}",
            report_path.display()
        ))
    } else {
        Ok(())
    }
}

fn parse_args() -> Result<Option<Config>, String> {
    let mut config = Config {
        app_root: mini_render::app_dir::default_app(),
        html_root: PathBuf::from("dist-html"),
        output_root: PathBuf::from("target/render-compare"),
        browser: None,
        routes: Vec::new(),
        all: false,
        compile_html: true,
        pixel_threshold: DEFAULT_PIXEL_THRESHOLD,
        settle_ms: DEFAULT_SETTLE_MS,
        rust_from: None,
    };

    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => return Ok(None),
            "--all" => config.all = true,
            "--no-compile" => config.compile_html = false,
            // 走解析器：裸名字（news-app）、sample/xxx、任意路径都接受
            "--app" => config.app_root = mini_render::app_dir::resolve(&next_value(&mut args, "--app")?),
            "--html" => config.html_root = PathBuf::from(next_value(&mut args, "--html")?),
            "--out" => config.output_root = PathBuf::from(next_value(&mut args, "--out")?),
            "--rust-from" => {
                config.rust_from = Some(PathBuf::from(next_value(&mut args, "--rust-from")?));
            }
            "--browser" => {
                config.browser = Some(PathBuf::from(next_value(&mut args, "--browser")?));
            }
            "--pixel-threshold" => {
                let value = next_value(&mut args, "--pixel-threshold")?;
                config.pixel_threshold = value
                    .parse::<u8>()
                    .map_err(|_| format!("--pixel-threshold 必须是 0..=255，收到: {value}"))?;
            }
            "--settle-ms" => {
                let value = next_value(&mut args, "--settle-ms")?;
                config.settle_ms = value
                    .parse::<u64>()
                    .map_err(|_| format!("--settle-ms 必须是非负整数，收到: {value}"))?;
            }
            value if value.starts_with('-') => {
                return Err(format!("未知选项: {value}（使用 --help 查看用法）"));
            }
            route => config.routes.push(route.trim_matches('/').to_string()),
        }
    }

    if config.routes.is_empty() {
        config.all = true;
    }
    Ok(Some(config))
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} 缺少参数值"))
}

fn print_usage() {
    println!(
        "Rust 原生渲染与 HTML/Chrome 批量对比\n\n\
用法:\n  cargo run --example compare -- [路由 ...] [选项]\n\n\
默认不传路由时比较 app.json 中全部页面。\n\n\
选项:\n  --all                   比较全部页面（忽略位置路由）\n  --app <目录>            小程序源码目录，默认 sample-app\n  --html <目录>           HTML 编译目录，默认 dist-html\n  --out <目录>            图片和报告目录，默认 target/render-compare\n  --browser <可执行文件>  Chrome/Chromium 路径\n  --pixel-threshold <0-255>  单通道差异阈值，默认 16\n  --settle-ms <毫秒>      截图前虚拟等待时间，默认 1000\n  --rust-from <目录>      用外部原生截图（窗体 --snapshot 产物）替代内置渲染\n  --no-compile            复用已有 HTML，不重新编译\n  -h, --help              显示帮助"
    );
}

fn select_pages<'a>(app: &'a AppSource, config: &Config) -> Result<Vec<&'a PageSource>, String> {
    if config.all {
        return Ok(app.pages.iter().collect());
    }

    let mut selected = Vec::with_capacity(config.routes.len());
    for route in &config.routes {
        let page = app
            .pages
            .iter()
            .find(|page| page.route == *route)
            .ok_or_else(|| format!("app.json 中不存在页面: {route}"))?;
        if !selected
            .iter()
            .any(|selected_page: &&PageSource| selected_page.route == page.route)
        {
            selected.push(page);
        }
    }
    Ok(selected)
}

fn compare_page(
    config: &Config,
    app: &AppSource,
    page: &PageSource,
    browser: &Path,
    profile_dir: &Path,
    url: &str,
) -> PageReport {
    let page_dir = route_output_dir(&config.output_root, &page.route);
    let rust_path = page_dir.join("rust.png");
    let html_path = page_dir.join("html.png");
    let side_path = page_dir.join("side-by-side.png");
    let diff_path = page_dir.join("diff.png");
    let outputs = OutputReport {
        rust_png: relative_report_path(&config.output_root, &rust_path),
        html_png: relative_report_path(&config.output_root, &html_path),
        side_by_side_png: relative_report_path(&config.output_root, &side_path),
        diff_png: relative_report_path(&config.output_root, &diff_path),
    };
    let mut report = PageReport {
        route: page.route.clone(),
        status: "error",
        error: None,
        outputs,
        rust_image: None,
        html_image: None,
        difference: None,
    };

    if let Err(error) = fs::create_dir_all(&page_dir) {
        report.error = Some(format!("创建页面输出目录失败: {error}"));
        return report;
    }
    for stale in [&rust_path, &html_path, &side_path, &diff_path] {
        if stale.exists() {
            let _ = fs::remove_file(stale);
        }
    }

    let native_result = match &config.rust_from {
        // 复用窗体宿主产出的整帧截图：这样报告里的差异就是「真实运行的窗体 vs H5」
        Some(dir) => copy_external_rust_png(dir, &page.route, &rust_path),
        None => render_rust_page(app, page, &rust_path),
    };
    if let Err(error) = native_result {
        report.error = Some(error);
        return report;
    }
    if let Err(error) = capture_html_page(
        browser,
        profile_dir,
        url,
        &html_path,
        config.settle_ms,
    ) {
        report.error = Some(error);
        return report;
    }

    let rust_image = match load_png(&rust_path, "Rust") {
        Ok(image) => image,
        Err(error) => {
            report.error = Some(error);
            return report;
        }
    };
    let html_image = match load_png(&html_path, "HTML") {
        Ok(image) => image,
        Err(error) => {
            report.error = Some(error);
            return report;
        }
    };
    report.rust_image = Some(ImageReport {
        width: rust_image.width(),
        height: rust_image.height(),
    });
    report.html_image = Some(ImageReport {
        width: html_image.width(),
        height: html_image.height(),
    });

    if let Err(error) = save_side_by_side(&rust_image, &html_image, &side_path) {
        report.error = Some(error);
        return report;
    }
    match compare_images(
        &rust_image,
        &html_image,
        config.pixel_threshold,
        &diff_path,
    ) {
        Ok(metrics) => {
            report.status = if metrics.changed_pixels == 0 {
                "identical"
            } else {
                "different"
            };
            report.difference = Some(metrics);
        }
        Err(error) => report.error = Some(error),
    }
    report
}

/// 从外部目录取窗体已渲染好的整帧 PNG（`<dir>/<route>/rust.png`）。
fn copy_external_rust_png(dir: &Path, route: &str, output: &Path) -> Result<(), String> {
    let source = dir.join(route).join("rust.png");
    if !source.exists() {
        return Err(format!("缺少窗体截图: {}（先跑 mini-app-window --snapshot）", source.display()));
    }
    fs::copy(&source, output)
        .map(|_| ())
        .map_err(|error| format!("复制 {} 失败: {error}", source.display()))
}

fn render_rust_page(app: &AppSource, page: &PageSource, output: &Path) -> Result<(), String> {
    let source_path = Path::new(&app.root).join(format!("{}.wxml", page.route));
    let wxml = fs::read_to_string(&source_path)
        .map_err(|error| format!("读取 {} 失败: {error}", source_path.display()))?;
    let asset_root = Path::new(&app.root).join("assets");
    let asset_prefix = format!("{}/", path_text(&asset_root));
    let rewrite_assets = |value: String| value.replace("/assets/", &asset_prefix);
    let wxml = rewrite_assets(wxml);
    let wxss = rewrite_assets(format!("{}\n{}", app.app_wxss, page.wxss));
    let data_json = serde_json::to_string(&page.data)
        .map(rewrite_assets)
        .map_err(|error| format!("序列化 {} 数据失败: {error}", page.route))?;
    let data = serde_json::from_str(&data_json)
        .map_err(|error| format!("解析 {} 数据失败: {error}", page.route))?;
    let nodes = WxmlParser::new(&wxml)
        .parse()
        .map_err(|error| format!("解析 {} WXML 失败: {error}", page.route))?;
    let stylesheet = WxssParser::new(&wxss)
        .parse()
        .map_err(|error| format!("解析 {} WXSS 失败: {error}", page.route))?;

    let mut renderer = WxmlRenderer::new_with_scale(
        stylesheet,
        VIEWPORT_WIDTH as f32,
        VIEWPORT_HEIGHT as f32,
        DEVICE_SCALE as f32,
    );
    let mut canvas = Canvas::new(
        VIEWPORT_WIDTH * DEVICE_SCALE,
        VIEWPORT_HEIGHT * DEVICE_SCALE,
    );
    canvas.clear(Color::from_hex(0xf5f6f8));

    // 应用外壳：tab 页在「页面内容之上、页面 fixed 覆盖层之下」绘制自定义 tabBar，
    // 与浏览器中 #app 内 fixed 遮罩压暗底部导航的层叠结果一致。
    let shell_tab = app
        .tab_index_of(&page.route)
        .filter(|_| app.uses_custom_tab_bar())
        .and_then(|index| app.custom_tab_bar.as_ref().map(|bar| (index, bar)));
    let mut shell_error: Option<String> = None;
    renderer.render_with_shell(&mut canvas, &nodes, &data, |shell_canvas| {
        if let Some((tab_index, bar)) = shell_tab {
            if let Err(error) = render_custom_tab_bar(app, bar, tab_index, shell_canvas) {
                shell_error = Some(error);
            }
        }
    });
    if let Some(error) = shell_error {
        return Err(error);
    }

    let output_text = output
        .to_str()
        .ok_or_else(|| format!("Rust PNG 路径不是有效 UTF-8: {}", output.display()))?;
    canvas
        .save_png(output_text)
        .map_err(|error| format!("保存 Rust PNG {} 失败: {error}", output.display()))
}

/// 用原生渲染器把自定义 tabBar 画到画布底部。
///
/// 组件 WXSS 本身不含定位信息（在小程序里由宿主固定于底部），这里补一条
/// `position:fixed;left:0;right:0;bottom:0` 规则，让渲染器的固定层按视口钉住，
/// 与 HTML 端 `.wx-custom-tabbar` 的固定定位等价。
fn render_custom_tab_bar(
    app: &AppSource,
    bar: &TabBarSource,
    selected: usize,
    canvas: &mut Canvas,
) -> Result<(), String> {
    let mut data = bar.data.clone();
    if let Some(obj) = data.as_object_mut() {
        obj.insert("selected".to_string(), serde_json::json!(selected));
    }
    let wxss = format!("{}\n{}", app.app_wxss, bar.wxss);
    let stylesheet = WxssParser::new(&wxss)
        .parse()
        .map_err(|error| format!("解析 custom-tab-bar WXSS 失败: {error}"))?;
    // 组件 WXSS 不含定位信息（在小程序里由宿主固定于底部）。这里给根节点加内联
    // position:fixed 而不是按类名注入 CSS —— 类名由各小程序自定义（.custom-tabbar /
    // .news-tabbar ...），按名字写死会让别的应用的 tabBar 落回正常流并盖住页面顶部。
    let nodes = pin_nodes_to_bottom(&bar.wxml);
    let mut renderer = WxmlRenderer::new_with_scale(
        stylesheet,
        VIEWPORT_WIDTH as f32,
        VIEWPORT_HEIGHT as f32,
        DEVICE_SCALE as f32,
    );
    renderer.render(canvas, &nodes, &data);
    Ok(())
}

/// 复制节点树，并给根元素追加「固定到视口底部」的内联样式。
fn pin_nodes_to_bottom(nodes: &[WxmlNode]) -> Vec<WxmlNode> {
    const PIN: &str = "position:fixed;left:0;right:0;bottom:0";
    nodes
        .iter()
        .map(|node| {
            let mut node = node.clone();
            if node.node_type == WxmlNodeType::Element {
                let merged = match node.attributes.get("style") {
                    Some(existing) if !existing.trim().is_empty() => {
                        let sep = if existing.trim_end().ends_with(';') { "" } else { ";" };
                        format!("{existing}{sep}{PIN}")
                    }
                    _ => PIN.to_string(),
                };
                node.attributes.insert("style".to_string(), merged);
            }
            node
        })
        .collect()
}

fn capture_html_page(
    browser: &Path,
    profile_dir: &Path,
    url: &str,
    output: &Path,
    settle_ms: u64,
) -> Result<(), String> {
    let absolute_output = absolute_path(output)?;
    let raw_output = absolute_output.with_file_name("html.chrome-raw.png");
    let absolute_profile = absolute_path(profile_dir)?;
    let _ = fs::remove_file(&raw_output);
    let screenshot_arg = format!("--screenshot={}", raw_output.display());
    let profile_arg = format!("--user-data-dir={}", absolute_profile.display());
    let window_arg = format!("--window-size={CAPTURE_SHELL_WIDTH},{VIEWPORT_HEIGHT}");
    let scale_arg = format!("--force-device-scale-factor={DEVICE_SCALE}");
    let wait_arg = format!("--virtual-time-budget={settle_ms}");

    // 某些 macOS Chrome 版本写完截图后仍会被 updater 后台任务挂住，不能直接
    // wait_with_output。轮询文件并主动回收进程，同时为真正的加载失败设置硬超时。
    let mut child = Command::new(browser)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--disable-extensions",
            "--disable-background-networking",
            "--disable-breakpad",
            "--disable-component-update",
            "--disable-crash-reporter",
            "--disable-default-apps",
            "--disable-logging",
            "--disable-sync",
            "--hide-scrollbars",
            "--no-first-run",
            "--no-default-browser-check",
            "--no-pings",
            "--run-all-compositor-stages-before-draw",
            &window_arg,
            &scale_arg,
            &wait_arg,
            &profile_arg,
            &screenshot_arg,
            url,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("启动浏览器 {} 失败: {error}", browser.display()))?;

    let started = Instant::now();
    let timeout = Duration::from_millis(settle_ms.saturating_add(15_000).max(15_000));
    let mut previous_size = None;
    loop {
        let current_size = fs::metadata(&raw_output)
            .ok()
            .map(|metadata| metadata.len())
            .filter(|size| *size > 0);
        if current_size.is_some() && current_size == previous_size {
            let _ = child.kill();
            let _ = child.wait();
            break;
        }
        previous_size = current_size;

        match child.try_wait() {
            Ok(Some(status)) => {
                if raw_output.is_file() {
                    break;
                }
                return Err(format!(
                    "Chrome 截图进程提前退出（{status}），未生成 {}",
                    raw_output.display()
                ));
            }
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("检查 Chrome 截图进程失败: {error}"));
            }
        }

        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "Chrome 截图在 {} 毫秒内未完成: {url}",
                timeout.as_millis()
            ));
        }
        thread::sleep(Duration::from_millis(40));
    }

    let raw = image::open(&raw_output)
        .map_err(|error| format!("读取 Chrome 原始截图 {} 失败: {error}", raw_output.display()))?
        .to_rgba8();
    let expected_width = VIEWPORT_WIDTH * DEVICE_SCALE;
    let expected_height = VIEWPORT_HEIGHT * DEVICE_SCALE;
    if raw.width() < expected_width || raw.height() < expected_height {
        let _ = fs::remove_file(&raw_output);
        return Err(format!(
            "Chrome 原始截图尺寸 {}x{} 小于目标 {}x{}",
            raw.width(),
            raw.height(),
            expected_width,
            expected_height
        ));
    }
    let normalized = image::imageops::crop_imm(
        &raw,
        0,
        0,
        expected_width,
        expected_height,
    )
    .to_image();
    normalized
        .save(&absolute_output)
        .map_err(|error| format!("保存 HTML PNG {} 失败: {error}", absolute_output.display()))?;
    let _ = fs::remove_file(&raw_output);
    Ok(())
}

fn resolve_browser(explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        if browser_works(path) {
            return Ok(path.to_path_buf());
        }
        return Err(format!("浏览器不可执行或无法启动: {}", path.display()));
    }

    if let Some(path) = std::env::var_os("MINI_COMPARE_BROWSER").map(PathBuf::from) {
        if browser_works(&path) {
            return Ok(path);
        }
    }

    let candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "chrome",
    ];
    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if browser_works(&path) {
            return Ok(path);
        }
    }

    Err("找不到 Chrome/Chromium；使用 --browser <路径> 或 MINI_COMPARE_BROWSER 指定".to_string())
}

fn browser_works(path: &Path) -> bool {
    Command::new(path)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn load_png(path: &Path, label: &str) -> Result<RgbaImage, String> {
    image::open(path)
        .map(|image| image.to_rgba8())
        .map_err(|error| format!("读取 {label} PNG {} 失败: {error}", path.display()))
}

fn save_side_by_side(left: &RgbaImage, right: &RgbaImage, output: &Path) -> Result<(), String> {
    const GAP: u32 = 8;
    let width = left.width() + GAP + right.width();
    let height = left.height().max(right.height());
    let mut combined = RgbaImage::from_pixel(width, height, Rgba([32, 35, 42, 255]));
    copy_image(left, &mut combined, 0, 0);
    copy_image(right, &mut combined, left.width() + GAP, 0);
    for y in 0..height {
        for x in left.width()..left.width() + GAP {
            combined.put_pixel(x, y, Rgba([110, 118, 132, 255]));
        }
    }
    combined
        .save(output)
        .map_err(|error| format!("保存并排图 {} 失败: {error}", output.display()))
}

fn copy_image(source: &RgbaImage, target: &mut RgbaImage, offset_x: u32, offset_y: u32) {
    for (x, y, pixel) in source.enumerate_pixels() {
        target.put_pixel(offset_x + x, offset_y + y, *pixel);
    }
}

fn compare_images(
    rust_image: &RgbaImage,
    html_image: &RgbaImage,
    threshold: u8,
    diff_output: &Path,
) -> Result<DiffMetrics, String> {
    let width = rust_image.width().max(html_image.width());
    let height = rust_image.height().max(html_image.height());
    let total_pixels = u64::from(width) * u64::from(height);
    let mut exact_changed_pixels = 0_u64;
    let mut changed_pixels = 0_u64;
    let mut absolute_error_sum = 0_u128;
    let mut squared_error_sum = 0_u128;
    let mut max_channel_error = 0_u8;
    let mut diff = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let rust_pixel = pixel_or_transparent(rust_image, x, y);
            let html_pixel = pixel_or_transparent(html_image, x, y);
            let mut pixel_max = 0_u8;
            let mut pixel_has_exact_change = false;
            for channel in 0..4 {
                let error = rust_pixel[channel].abs_diff(html_pixel[channel]);
                pixel_max = pixel_max.max(error);
                max_channel_error = max_channel_error.max(error);
                absolute_error_sum += u128::from(error);
                squared_error_sum += u128::from(error) * u128::from(error);
                pixel_has_exact_change |= error != 0;
            }
            if pixel_has_exact_change {
                exact_changed_pixels += 1;
            }
            if pixel_max > threshold {
                changed_pixels += 1;
                diff.put_pixel(
                    x,
                    y,
                    Rgba([80_u8.saturating_add(pixel_max.saturating_mul(2) / 3), 0, 0, 255]),
                );
            } else {
                let gray = ((u16::from(rust_pixel[0])
                    + u16::from(rust_pixel[1])
                    + u16::from(rust_pixel[2]))
                    / 3
                    / 4) as u8;
                diff.put_pixel(x, y, Rgba([gray, gray, gray, 255]));
            }
        }
    }

    diff.save(diff_output)
        .map_err(|error| format!("保存差异图 {} 失败: {error}", diff_output.display()))?;
    let channel_samples = total_pixels.saturating_mul(4).max(1);
    let total_pixels_nonzero = total_pixels.max(1);
    Ok(DiffMetrics {
        compared_width: width,
        compared_height: height,
        total_pixels,
        exact_changed_pixels,
        exact_changed_ratio: exact_changed_pixels as f64 / total_pixels_nonzero as f64,
        changed_pixels,
        changed_ratio: changed_pixels as f64 / total_pixels_nonzero as f64,
        mean_absolute_error: absolute_error_sum as f64 / channel_samples as f64,
        root_mean_square_error: (squared_error_sum as f64 / channel_samples as f64).sqrt(),
        max_channel_error,
        dimensions_match: rust_image.dimensions() == html_image.dimensions(),
    })
}

fn pixel_or_transparent(image: &RgbaImage, x: u32, y: u32) -> Rgba<u8> {
    if x < image.width() && y < image.height() {
        *image.get_pixel(x, y)
    } else {
        Rgba([0, 0, 0, 0])
    }
}

fn summarize(pages: &[PageReport]) -> SummaryReport {
    let mut compared_pages = 0_usize;
    let mut failed_pages = 0_usize;
    let mut dimension_mismatches = 0_usize;
    let mut total_pixels = 0_u64;
    let mut changed_pixels = 0_u64;
    let mut weighted_absolute_error = 0.0_f64;
    let mut max_channel_error = 0_u8;

    for page in pages {
        if let Some(metrics) = &page.difference {
            compared_pages += 1;
            dimension_mismatches += usize::from(!metrics.dimensions_match);
            total_pixels += metrics.total_pixels;
            changed_pixels += metrics.changed_pixels;
            weighted_absolute_error += metrics.mean_absolute_error * metrics.total_pixels as f64;
            max_channel_error = max_channel_error.max(metrics.max_channel_error);
        } else {
            failed_pages += 1;
        }
    }

    SummaryReport {
        requested_pages: pages.len(),
        compared_pages,
        failed_pages,
        dimension_mismatches,
        total_pixels,
        changed_pixels,
        changed_ratio: if total_pixels == 0 {
            0.0
        } else {
            changed_pixels as f64 / total_pixels as f64
        },
        mean_absolute_error: if total_pixels == 0 {
            0.0
        } else {
            weighted_absolute_error / total_pixels as f64
        },
        max_channel_error,
    }
}

fn route_output_dir(root: &Path, route: &str) -> PathBuf {
    let mut output = root.to_path_buf();
    for component in Path::new(route).components() {
        if let Component::Normal(part) = component {
            output.push(part);
        }
    }
    output
}

fn relative_report_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|error| format!("无法读取当前目录: {error}"))
    }
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn serve_connection(mut stream: TcpStream, root: &Path) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| error.to_string())?;
    // 读到完整请求头为止（单次 read 可能只拿到部分字节）
    let mut buffer = Vec::with_capacity(8192);
    let mut chunk = [0_u8; 4096];
    loop {
        if buffer.windows(4).any(|w| w == b"\r\n\r\n") || buffer.len() > 64 * 1024 {
            break;
        }
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buffer.extend_from_slice(&chunk[..n]),
            Err(ref error) if error.kind() == ErrorKind::WouldBlock || error.kind() == ErrorKind::Interrupted => {
                thread::sleep(Duration::from_millis(2));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    let request = String::from_utf8_lossy(&buffer);
    let first_line = request.lines().next().unwrap_or_default();
    let mut fields = first_line.split_whitespace();
    let method = fields.next().unwrap_or_default();
    let request_path = fields.next().unwrap_or("/");
    if method != "GET" && method != "HEAD" {
        return write_http_response(&mut stream, 405, "text/plain; charset=utf-8", b"method not allowed", method == "HEAD");
    }

    // 外壳页：内嵌精确 375x667 iframe，建立独立视口承载真实页面。
    if let Some(encoded_route) = request_path
        .split(['?', '#'])
        .next()
        .and_then(|path| path.strip_prefix("/__mini_shell__/"))
    {
        let Some(route) = percent_decode(encoded_route).and_then(|r| safe_url_path(&r)) else {
            return write_http_response(&mut stream, 400, "text/plain; charset=utf-8", b"bad route", method == "HEAD");
        };
        let route = route.to_string_lossy().replace('\\', "/");
        let iframe_src = html_attribute_escape(&format!("/{route}.html?__mini_compare=1"));
        let shell = format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><style>\
             *{{margin:0;padding:0;box-sizing:border-box}}html,body{{width:{shell}px;height:{vh}px;\
             overflow:hidden;background:#f5f6f8}}\
             iframe{{position:absolute;top:0;left:0;width:{vw}px;height:{vh}px;border:0;\
             margin:0;display:block;background:#f5f6f8;color-scheme:light}}\
             </style></head><body>\
             <iframe src=\"{iframe_src}\" scrolling=\"no\" frameborder=\"0\"></iframe>\
             </body></html>",
            shell = CAPTURE_SHELL_WIDTH, vw = VIEWPORT_WIDTH, vh = VIEWPORT_HEIGHT, iframe_src = iframe_src,
        );
        return write_http_response(&mut stream, 200, "text/html; charset=utf-8", shell.as_bytes(), method == "HEAD");
    }

    let is_capture = request_path
        .split(['?', '#'])
        .nth(1)
        .map(|query| query.split('&').any(|pair| pair == "__mini_compare=1"))
        .unwrap_or(false);

    let Some(relative_path) = safe_url_path(request_path) else {
        return write_http_response(&mut stream, 400, "text/plain; charset=utf-8", b"bad request", method == "HEAD");
    };
    let mut file_path = root.join(relative_path);
    if file_path.is_dir() {
        file_path.push("index.html");
    }
    if !file_path.starts_with(root) {
        return write_http_response(&mut stream, 403, "text/plain; charset=utf-8", b"forbidden", method == "HEAD");
    }

    // 对页面 HTML 的对比抓取请求：注入覆盖样式，把 375px 的 #app 钉在左上角、
    // 关掉外壳滚动，避免 Chrome 最小窗口宽度导致的水平居中偏移。
    let is_html = file_path.extension().and_then(|e| e.to_str()) == Some("html");
    if is_capture && is_html {
        return match fs::read_to_string(&file_path) {
            Ok(html) => {
                let injected = inject_capture_override(&html);
                write_http_response(
                    &mut stream,
                    200,
                    "text/html; charset=utf-8",
                    injected.as_bytes(),
                    method == "HEAD",
                )
            }
            Err(error) if error.kind() == ErrorKind::NotFound => write_http_response(
                &mut stream,
                404,
                "text/plain; charset=utf-8",
                b"not found",
                method == "HEAD",
            ),
            Err(error) => Err(error.to_string()),
        };
    }

    match fs::read(&file_path) {
        Ok(body) => write_http_response(
            &mut stream,
            200,
            content_type(&file_path),
            &body,
            method == "HEAD",
        ),
        Err(error) if error.kind() == ErrorKind::NotFound => write_http_response(
            &mut stream,
            404,
            "text/plain; charset=utf-8",
            b"not found",
            method == "HEAD",
        ),
        Err(error) => Err(error.to_string()),
    }
}

/// 注入对比抓取用的覆盖样式并禁用页面脚本。
///
/// - 覆盖样式：固定视口宽度、左上角对齐、关闭外壳滚动，让浏览器渲染范围与原生
///   375x667 画布严格一致。
/// - 禁用脚本：编译产物的静态首屏 HTML 已由同一份 data 快照完整渲染并带全部 CSS 类，
///   与原生渲染器的静态渲染语义一致。runtime.js 的响应式重渲染只服务交互，对静态
///   截图无意义，且其异步执行会与截图时机竞争（偶发丢样式）。禁用后截图完全确定。
/// - 例外：页面含 `<canvas>` 时保留脚本。canvas 的画面**只能**由 JS 画出来，
///   静态 HTML 里是一块空白，禁用脚本等于拿「空画布」当参考基准 —— 那样原生端
///   把 canvas 画对了反而会让差异变大，这项对比就失去意义了。
fn inject_capture_override(html: &str) -> String {
    let has_canvas = html.to_ascii_lowercase().contains("<canvas");
    // 把 <script ...> 改成惰性类型，浏览器不再执行任何页面脚本。
    let disabled = if has_canvas {
        html.to_string()
    } else {
        replace_case_insensitive(html, "<script", "<script type=\"application/x-mini-disabled\" ")
    };
    let override_style = format!(
        "<style id=\"__mini_compare_override\">\
         html,body{{margin:0!important;padding:0!important;width:{VIEWPORT_WIDTH}px!important;\
         min-width:{VIEWPORT_WIDTH}px!important;background:#f5f6f8!important;overflow:hidden!important;}}\
         #app{{margin:0!important;left:0!important;right:auto!important;}}\
         *{{scrollbar-width:none!important;}}::-webkit-scrollbar{{display:none!important;}}\
         </style>"
    );
    if let Some(index) = disabled.to_ascii_lowercase().find("</head>") {
        let mut result = String::with_capacity(disabled.len() + override_style.len());
        result.push_str(&disabled[..index]);
        result.push_str(&override_style);
        result.push_str(&disabled[index..]);
        result
    } else {
        format!("{override_style}{disabled}")
    }
}

fn html_attribute_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// 大小写不敏感地替换子串（仅用于禁用 <script> 标签）。
fn replace_case_insensitive(haystack: &str, needle: &str, replacement: &str) -> String {
    let lower_haystack = haystack.to_ascii_lowercase();
    let lower_needle = needle.to_ascii_lowercase();
    let mut result = String::with_capacity(haystack.len());
    let mut cursor = 0;
    while let Some(found) = lower_haystack[cursor..].find(&lower_needle) {
        let start = cursor + found;
        result.push_str(&haystack[cursor..start]);
        result.push_str(replacement);
        cursor = start + needle.len();
    }
    result.push_str(&haystack[cursor..]);
    result
}

fn safe_url_path(request_path: &str) -> Option<PathBuf> {
    let encoded = request_path.split(['?', '#']).next().unwrap_or("/");
    let decoded = percent_decode(encoded)?;
    let mut relative = PathBuf::new();
    for component in Path::new(decoded.trim_start_matches('/')).components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }
    if relative.as_os_str().is_empty() {
        relative.push("index.html");
    }
    Some(relative)
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_value(*bytes.get(index + 1)?)?;
            let low = hex_value(*bytes.get(index + 2)?)?;
            decoded.push(high * 16 + low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> Result<(), String> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .and_then(|_| if head_only { Ok(()) } else { stream.write_all(body) })
        .map_err(|error| error.to_string())
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()).unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => "application/octet-stream",
    }
}
