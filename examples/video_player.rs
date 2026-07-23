//! 独立视频播放示例：打开窗口，自动循环播放 doc/videos/video.mp4，
//! 视频帧按显示器刷新率呈现（H.264 解码 + macOS 音频输出）。
//!
//! 运行：cargo run --example video_player
//! 关闭窗口或按 Esc 退出。

use mini_render::renderer::components::{get_or_create_player, get_video_frame, get_video_progress};
use mini_render::{Canvas, Color};

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

const SRC: &str = "doc/videos/video.mp4";
const W: u32 = 360;
const H: u32 = 640;

struct VideoApp {
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    canvas: Canvas,
    frame_interval: Duration,
    last_present: Instant,
    started: Instant,
    frames_shown: u64,
}

impl VideoApp {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            canvas: Canvas::new(W, H),
            frame_interval: Duration::from_micros(16_667),
            last_present: Instant::now(),
            started: Instant::now(),
            frames_shown: 0,
        }
    }
}

impl ApplicationHandler for VideoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let attrs = Window::default_attributes()
            .with_title("Mini Render - 视频播放")
            .with_inner_size(winit::dpi::LogicalSize::new(W, H))
            .with_resizable(false);
        let window = Rc::new(event_loop.create_window(attrs).unwrap());
        let millihertz = window.current_monitor()
            .and_then(|m| m.refresh_rate_millihertz())
            .filter(|&hz| hz > 0)
            .unwrap_or(60_000);
        self.frame_interval = Duration::from_secs_f64(1000.0 / millihertz as f64);
        println!("🖥️  刷新率 {:.0}Hz", millihertz as f64 / 1000.0);

        let ctx = softbuffer::Context::new(window.clone()).unwrap();
        self.surface = Some(softbuffer::Surface::new(&ctx, window.clone()).unwrap());

        // 加载并自动循环播放
        println!("🎬 解码中（首次需要几秒预解码全部帧）...");
        get_or_create_player(SRC, true, true);
        self.window = Some(window);
        self.started = Instant::now();
        if let Some(w) = &self.window { w.request_redraw(); }
    }

    fn new_events(&mut self, _e: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            if let Some(w) = &self.window { w.request_redraw(); }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Escape) {
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                self.canvas.clear(Color::BLACK);
                if let Some((data, fw, fh)) = get_video_frame(SRC) {
                    // 按比例适配窗口（aspectFit）
                    self.canvas.draw_image(&data, fw, fh, 0.0, 0.0, W as f32, H as f32, "aspectFit", 0.0);
                    self.frames_shown += 1;
                }
                if let (Some(window), Some(surface)) = (&self.window, &mut self.surface) {
                    let size = window.inner_size();
                    if size.width > 0 && size.height > 0 {
                        surface.resize(NonZeroU32::new(size.width).unwrap(), NonZeroU32::new(size.height).unwrap()).unwrap();
                        let mut buffer = surface.buffer_mut().unwrap();
                        let px = self.canvas.pixels();
                        let cw = self.canvas.width() as usize;
                        for y in 0..size.height.min(self.canvas.height()) as usize {
                            for x in 0..size.width.min(self.canvas.width()) as usize {
                                let c = &px[y * cw + x];
                                buffer[y * size.width as usize + x] = ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32;
                            }
                        }
                        buffer.present().unwrap();
                    }
                }
                self.last_present = Instant::now();
                // 每秒打印一次实测帧率
                let elapsed = self.started.elapsed().as_secs_f64();
                if self.frames_shown % 60 == 0 {
                    if let Some((cur, dur)) = get_video_progress(SRC) {
                        println!("▶ {:.1}/{:.1}s  平均 {:.0} fps", cur, dur, self.frames_shown as f64 / elapsed.max(0.001));
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // 持续按刷新率出帧（视频一直在播放）
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.last_present + self.frame_interval));
    }
}

fn main() {
    println!("视频播放示例：{}", SRC);
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut VideoApp::new()).unwrap();
}
