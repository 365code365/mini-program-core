//! 小程序启动器：扫 `sample/`、列出来、点一下**交给 `mini-app-window` 跑**。
//!
//! ## 它为什么不自己跑小程序
//! 从前它是**第三个宿主**：自己加载页面、自己建渲染器、自己一套事件循环与上屏，
//! 与 `src/bin/window.rs`（桌面窗体）和 `host::MiniEngine`（移动端 SDK）并列。
//! 代价不是多几百行代码，而是**功能永远跟不上**：那份实现只画首页、没有页面栈、
//! 没有手势仲裁、没有 picker 面板、没有侧滑返回、没有局部重绘 —— 从启动器里点进去
//! 看到的东西和真正跑起来不是一回事，用它验证问题会得到错的结论。
//!
//! 现在它只做「选哪个」这一件事：点启动就 `spawn` 一个 `mini-app-window <目录>`
//! 子进程，跑的是与真机链路完全相同的那份宿主。启动器窗口留着，可以接着开别的。
//! 命令行里更快的等价物是 `./run.sh`（同一套扫描规则）。

use std::fs;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use softbuffer::Surface;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use mini_render::text::TextRenderer;
use mini_render::ui::ScrollController;
use mini_render::{Canvas, Color, Paint, Rect};

const WINDOW_WIDTH: u32 = 375;
const WINDOW_HEIGHT: u32 = 667;
/// 列表项与标题栏的尺寸（逻辑像素），渲染与命中共用同一组常量
const HEADER_H: f32 = 88.0;
const ITEM_H: f32 = 80.0;
const PADDING: f32 = 16.0;
const BTN_W: f32 = 60.0;

/// 小程序信息
#[derive(Clone, Debug)]
struct MiniAppInfo {
    name: String,
    path: PathBuf,
    description: String,
}

struct LauncherApp {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    canvas: Canvas,
    text_renderer: Option<std::sync::Arc<TextRenderer>>,
    mini_apps: Vec<MiniAppInfo>,
    scale_factor: f32,
    mouse_pos: (f32, f32),
    list_scroll: ScrollController,
    last_frame: Instant,
    click_start_pos: (f32, f32),
    click_start_time: Instant,
    frame_interval: std::time::Duration,
}

impl LauncherApp {
    fn new() -> Self {
        let canvas = Canvas::new(WINDOW_WIDTH * 2, WINDOW_HEIGHT * 2);
        let text_renderer = mini_render::text::shared_fonts();
        let mini_apps = scan_sample_directory();
        let list_content_height = HEADER_H + PADDING + mini_apps.len() as f32 * (ITEM_H + PADDING);
        let now = Instant::now();
        Self {
            window: None,
            surface: None,
            canvas,
            text_renderer,
            mini_apps,
            scale_factor: 2.0,
            mouse_pos: (0.0, 0.0),
            list_scroll: ScrollController::new(list_content_height, WINDOW_HEIGHT as f32),
            last_frame: now,
            click_start_pos: (0.0, 0.0),
            click_start_time: now,
            frame_interval: std::time::Duration::from_micros(16_667),
        }
    }

    /// 需要连续出帧吗（只有列表的惯性滚动会要）
    fn is_active(&self) -> bool {
        self.list_scroll.is_animating() || self.list_scroll.is_dragging
    }

    fn render_list(&mut self) {
        self.canvas.clear(Color::from_hex(0xF5F5F5));
        let sf = self.scale_factor;
        let scroll_offset = self.list_scroll.get_position();

        // 标题栏
        let header_height = HEADER_H * sf;
        let header_paint = Paint::new().with_color(Color::from_hex(0xFF6B35));
        self.canvas.draw_rect(
            &Rect::new(0.0, 0.0, WINDOW_WIDTH as f32 * sf, header_height),
            &header_paint,
        );
        if let Some(tr) = &self.text_renderer {
            let title_paint = Paint::new().with_color(Color::WHITE);
            tr.draw_text(&mut self.canvas, "小程序启动器", 20.0 * sf, 46.0 * sf, 18.0 * sf, &title_paint);
            let sub_paint = Paint::new().with_color(Color::new(255, 255, 255, 200));
            tr.draw_text(&mut self.canvas, "点「启动」在新窗口里跑", 20.0 * sf, 70.0 * sf, 12.0 * sf, &sub_paint);
        }

        let item_height = ITEM_H * sf;
        let padding = PADDING * sf;
        let start_y = header_height + padding - scroll_offset * sf;

        for (i, app) in self.mini_apps.iter().enumerate() {
            let y = start_y + i as f32 * (item_height + padding);
            if y + item_height < header_height || y > WINDOW_HEIGHT as f32 * sf {
                continue; // 视口外不画
            }
            let card_paint = Paint::new().with_color(Color::WHITE);
            self.canvas.draw_rect(
                &Rect::new(padding, y, (WINDOW_WIDTH as f32 - 32.0) * sf, item_height),
                &card_paint,
            );
            let icon_size = 48.0 * sf;
            let icon_x = padding + 16.0 * sf;
            let icon_y = y + (item_height - icon_size) / 2.0;
            let icon_paint = Paint::new().with_color(Color::from_hex(0xFF6B35));
            self.canvas.draw_rect(&Rect::new(icon_x, icon_y, icon_size, icon_size), &icon_paint);

            if let Some(tr) = &self.text_renderer {
                let text_x = icon_x + icon_size + 16.0 * sf;
                let name_paint = Paint::new().with_color(Color::from_hex(0x333333));
                tr.draw_text(&mut self.canvas, &app.name, text_x, y + 30.0 * sf, 16.0 * sf, &name_paint);
                let desc_paint = Paint::new().with_color(Color::from_hex(0x999999));
                tr.draw_text(&mut self.canvas, &app.description, text_x, y + 55.0 * sf, 12.0 * sf, &desc_paint);
            }

            let btn_width = BTN_W * sf;
            let btn_height = 32.0 * sf;
            let btn_x = (WINDOW_WIDTH as f32 - 32.0 - 16.0) * sf - btn_width;
            let btn_y = y + (item_height - btn_height) / 2.0;
            let btn_paint = Paint::new().with_color(Color::from_hex(0x07C160));
            self.canvas.draw_rect(&Rect::new(btn_x, btn_y, btn_width, btn_height), &btn_paint);
            if let Some(tr) = &self.text_renderer {
                let btn_text_paint = Paint::new().with_color(Color::WHITE);
                tr.draw_text(&mut self.canvas, "启动", btn_x + 12.0 * sf, btn_y + 22.0 * sf, 14.0 * sf, &btn_text_paint);
            }
        }

        if self.mini_apps.is_empty() {
            if let Some(tr) = &self.text_renderer {
                let hint = Paint::new().with_color(Color::from_hex(0x999999));
                tr.draw_text(&mut self.canvas, "sample 目录下没有找到小程序", 60.0 * sf, 300.0 * sf, 14.0 * sf, &hint);
                tr.draw_text(&mut self.canvas, "一层子目录里有 app.json 就算一个", 46.0 * sf, 330.0 * sf, 14.0 * sf, &hint);
            }
        }
    }

    /// 点击列表：命中「启动」按钮就开一个窗体进程
    fn handle_list_click(&mut self, x: f32, y: f32) -> bool {
        let (lx, ly) = (x / self.scale_factor, y / self.scale_factor);
        if ly < HEADER_H {
            return false;
        }
        let start_y = HEADER_H + PADDING - self.list_scroll.get_position();
        let btn_x = WINDOW_WIDTH as f32 - 32.0 - 16.0 - BTN_W;
        for (i, app) in self.mini_apps.iter().enumerate() {
            let item_y = start_y + i as f32 * (ITEM_H + PADDING);
            let hit_row = ly >= item_y && ly < item_y + ITEM_H;
            if hit_row && lx >= btn_x && lx < btn_x + BTN_W {
                spawn_window(&app.path, &app.name);
                return true;
            }
        }
        false
    }
}

/// 开一个 `mini-app-window <目录>` 子进程。
///
/// 与启动器同目录取二进制：`cargo build` 出来的两个可执行文件本来就在一起，
/// 这样开发期不用配 PATH。找不到就打印一条能直接复制的命令 —— 比静默失败好。
fn spawn_window(app_path: &Path, name: &str) {
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("mini-app-window")));
    match exe {
        Some(bin) if bin.exists() => {
            match std::process::Command::new(&bin).arg(app_path).spawn() {
                Ok(child) => println!("🚀 {name} -> {} (pid {})", app_path.display(), child.id()),
                Err(e) => eprintln!("❌ 启动失败: {e}；手动跑：{} {}", bin.display(), app_path.display()),
            }
        }
        _ => eprintln!(
            "❌ 找不到 mini-app-window（先 cargo build --release）；手动跑：\n   ./target/release/mini-app-window {}",
            app_path.display()
        ),
    }
}

impl ApplicationHandler for LauncherApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Mini Program Launcher")
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_resizable(false);
        let window = Rc::new(event_loop.create_window(attrs).unwrap());
        // 按显示器刷新率设定帧间隔（支持高刷屏）
        let millihertz = window
            .current_monitor()
            .and_then(|m| m.refresh_rate_millihertz())
            .filter(|&hz| hz > 0)
            .unwrap_or(60_000);
        self.frame_interval = std::time::Duration::from_secs_f64(1000.0 / millihertz as f64);
        let context = softbuffer::Context::new(window.clone()).unwrap();
        let surface = Surface::new(&context, window.clone()).unwrap();
        self.window = Some(window);
        self.surface = Some(surface);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame).as_secs_f32();
                self.last_frame = now;
                self.list_scroll.update(dt);
                self.render_list();

                if let (Some(window), Some(surface)) = (&self.window, &mut self.surface) {
                    let size = window.inner_size();
                    surface
                        .resize(
                            NonZeroU32::new(size.width).unwrap(),
                            NonZeroU32::new(size.height).unwrap(),
                        )
                        .unwrap();
                    let mut buffer = surface.buffer_mut().unwrap();
                    // 列表的滚动已经在渲染时算进坐标，这里是直白的整屏拷贝
                    let pixels = self.canvas.pixels();
                    let cw = self.canvas.width() as usize;
                    let ch = self.canvas.height() as usize;
                    for y in 0..size.height.min(ch as u32) as usize {
                        for x in 0..size.width.min(cw as u32) as usize {
                            let (src, dst) = (y * cw + x, y * size.width as usize + x);
                            if src < pixels.len() && dst < buffer.len() {
                                let c = &pixels[src];
                                buffer[dst] =
                                    ((c.r as u32) << 16) | ((c.g as u32) << 8) | (c.b as u32);
                            }
                        }
                    }
                    buffer.present().unwrap();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x as f32, position.y as f32);
                self.mouse_pos = (x, y);
                if self.list_scroll.is_dragging {
                    let ts = now_ms();
                    self.list_scroll.update_drag(y / self.scale_factor, ts);
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
            }

            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                let (x, y) = self.mouse_pos;
                match state {
                    ElementState::Pressed => {
                        self.click_start_pos = (x, y);
                        self.click_start_time = Instant::now();
                        self.list_scroll.begin_drag(y / self.scale_factor, now_ms());
                    }
                    ElementState::Released => {
                        self.list_scroll.end_drag();
                        // 位移小、时间短才算点击（否则是一次滑动）
                        let moved = (x - self.click_start_pos.0).abs() + (y - self.click_start_pos.1).abs();
                        let quick = self.click_start_time.elapsed().as_millis() < 400;
                        if moved < 10.0 * self.scale_factor && quick {
                            self.handle_list_click(x, y);
                        }
                        if let Some(w) = &self.window {
                            w.request_redraw();
                        }
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                self.list_scroll.handle_scroll(dy, false);
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor as f32;
                self.canvas = Canvas::new(
                    (WINDOW_WIDTH as f32 * self.scale_factor) as u32,
                    (WINDOW_HEIGHT as f32 * self.scale_factor) as u32,
                );
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }

            _ => {}
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        if let winit::event::StartCause::ResumeTimeReached { .. } = cause {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
    }

    /// 有惯性滚动时按刷新率安排下一帧，空闲时休眠（CPU 归零）
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.is_active() {
            event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + self.frame_interval));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 扫 `sample/`：**一层子目录里有 `app.json` 就算一个小程序**
/// （与 `run.sh` / `mini_render::app_dir` 同一套规则，`_archive` 这类归档不会被算进来）
fn scan_sample_directory() -> Vec<MiniAppInfo> {
    let sample_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample");
    let mut apps = Vec::new();
    if let Ok(entries) = fs::read_dir(&sample_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(info) = parse_app_info(&path) {
                    apps.push(info);
                }
            }
        }
    }
    apps.sort_by(|a, b| a.name.cmp(&b.name));
    apps
}

/// 读一个目录的 `app.json`，取名字与页数
fn parse_app_info(path: &Path) -> Option<MiniAppInfo> {
    let app_json_path = path.join("app.json");
    if !app_json_path.exists() {
        return None;
    }
    let content = fs::read_to_string(&app_json_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let name = path.file_name()?.to_string_lossy().to_string();
    let pages_count = json.get("pages").and_then(|p| p.as_array()).map(|a| a.len()).unwrap_or(0);
    Some(MiniAppInfo {
        name,
        path: path.to_path_buf(),
        description: format!("{pages_count} 个页面"),
    })
}

fn main() {
    println!("🚀 Mini Program Launcher");
    println!("========================");
    println!("扫描 sample 目录下的小程序...\n");

    let event_loop = EventLoop::new().unwrap();
    // 默认空闲休眠；惯性滚动期间由 about_to_wait 切到按刷新率出帧
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = LauncherApp::new();
    println!("找到 {} 个小程序：", app.mini_apps.len());
    for a in &app.mini_apps {
        println!("  · {:<14} {}", a.name, a.description);
    }
    println!("\n点「启动」会开一个 mini-app-window 窗口（与真机同一条链路）。");
    println!("命令行等价物：./run.sh <名字>\n");
    event_loop.run_app(&mut app).unwrap();
}
