//! GPU 对拍（设计 §13.2；v2 修订 R1）：固定种子生成图层配置，离屏合成后与 `cpu::compose`
//! **逐字节**比较。没有可用适配器时打印「GPU 未测」并跳过 —— 决定性判据是 `tools/gpu-parity.sh`，
//! 它在没测到时不会返回 0。
use super::{cpu_rgba, first_mismatch, open, GpuCompositor, OpenOptions};
use crate::compositor::select::{SelectInput, GPU_COMPILED};
use crate::compositor::{cpu, BackendPref, DeviceInfo, FrameLayers, LayerCanvas, OverlayRefs, PageCanvas, Platform, PrefSource, PullState};
use crate::{Canvas, Color, Rect};

fn open_for_test(w: u32, h: u32) -> Option<GpuCompositor> {
    open_with_fault(w, h, None)
}

fn open_with_fault(w: u32, h: u32, fault: Option<crate::compositor::FaultSpec>) -> Option<GpuCompositor> {
    let device = DeviceInfo::default();
    let select = SelectInput {
        pref: BackendPref::Gpu,
        source: PrefSource::Cli,
        platform: Platform::current(),
        device: &device,
        gpu_compiled: GPU_COMPILED,
        has_surface: true,
    };
    let opts = OpenOptions { select, fault, log: false, width: w, height: h, canvas_w: w, margin_px: 400, use_verdict: false };
    match open(opts, None) {
        Ok((c, s)) => {
            eprintln!("GPU 对拍用 {}", s.adapter.as_deref().unwrap_or("?"));
            Some(c)
        }
        Err(s) => {
            eprintln!("⚠ GPU 未测：{}", s.reason);
            None
        }
    }
}

struct Rng(u64);
impl Rng {
    fn u(&mut self, n: u32) -> u32 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as u32) % n.max(1)
    }
    fn byte(&mut self) -> u8 {
        self.u(256) as u8
    }
}

fn noise(c: &mut Canvas, rng: &mut Rng, alpha: impl Fn(&mut Rng) -> u8) {
    for p in c.pixels_mut() {
        *p = Color::new(rng.byte(), rng.byte(), rng.byte(), alpha(rng));
    }
}

fn check(comp: &mut GpuCompositor, l: &FrameLayers<'_>, what: &str) {
    comp.compose_offscreen(l, true).unwrap_or_else(|e| panic!("{what}：GPU 合成失败 {e}"));
    let gpu = comp.read_back().unwrap();
    let mut buf = vec![0u32; (l.width * l.height) as usize];
    cpu::compose(&mut buf, l);
    if let Some((x, y, c, g)) = first_mismatch(&cpu_rgba(&buf), &gpu, l.width) {
        panic!("{what}：@{x},{y} CPU #{c:06x} / GPU #{g:06x}");
    }
}

#[test]
fn random_layer_configs_match_cpu_byte_for_byte() {
    let (w, h) = (120u32, 200u32);
    let Some(mut comp) = open_for_test(w, h) else { return };
    let mut rng = Rng(7);
    for case in 0..50 {
        // 画布：比输出窄 / 等宽，比视口矮 / 跨环形层边界
        let cw = if rng.u(4) == 0 { w - 1 - rng.u(30) } else { w };
        let ch = match rng.u(3) {
            0 => 1 + rng.u(h),
            1 => 1000 + rng.u(1200),
            _ => 2049,
        };
        let mut page = Canvas::new(cw, ch);
        noise(&mut page, &mut rng, |r| r.byte());
        let page = PageCanvas::new(page);
        let tab_h = 10 + rng.u(30);
        let mut tab = Canvas::new(if rng.u(3) == 0 { w / 2 } else { w }, tab_h + rng.u(2) * 5);
        noise(&mut tab, &mut rng, |_| 255);
        let tab = LayerCanvas::new(tab);
        let mut fixed = Canvas::new(w, h);
        noise(&mut fixed, &mut rng, |r| [0, 255, r.byte()][r.u(3) as usize]);
        let fixed = LayerCanvas::new(fixed);
        let has_tabbar = rng.u(2) == 0;
        let scroll_px = match rng.u(4) {
            0 => -(rng.u(60) as i32),
            1 => ch as i32 - (rng.u(h) as i32),
            2 => 1020 + rng.u(10) as i32,
            _ => rng.u(ch) as i32,
        };
        let rows = rng.u(h);
        let l = FrameLayers {
            width: w,
            height: h,
            scale: 2.0,
            page: &page,
            scroll_px,
            page_bg: rng.u(0xFF_FFFF),
            has_tabbar,
            tabbar: (rng.u(5) != 0).then_some(&tab),
            tabbar_px: if has_tabbar { tab_h } else { 0 },
            fixed: (rng.u(3) != 0).then_some(&fixed),
            fixed_rows: (rng.u(4) != 0).then(|| (rows, (rows + rng.u(h)).min(h + 5))),
            pull: (rng.u(3) == 0).then(|| PullState { gap_logical: rng.u(80) as f32, refreshing: rng.u(2) == 0, progress: 0.7, phase: 0.25 }),
            overlays: OverlayRefs::none(),
            edge_back: None,
        };
        check(&mut comp, &l, &format!("配置 #{case}"));
    }
}

/// 同一个合成器连续 200 步：整带写 / 矩形写 / 滚动 / 换画布 / 跳过若干帧不合成。
/// 守的是增量镜像（MirrorRows）—— 静态快照只比最后一帧，测不到中途的记账错误（v2 修订 R1）
#[test]
fn incremental_mirror_200_steps_match_cpu() {
    let (w, h) = (96u32, 160u32);
    let Some(mut comp) = open_for_test(w, h) else { return };
    let mut rng = Rng(42);
    let mut base = Canvas::new(w, 2600);
    noise(&mut base, &mut rng, |r| r.byte());
    let mut page = PageCanvas::new(base);
    let mut scroll = 0i32;
    for step in 0..200 {
        match rng.u(6) {
            0 => {
                let y0 = scroll - 40 + rng.u(80) as i32;
                let y1 = y0 + 50 + rng.u(400) as i32;
                let c = page.write_band(y0, y1);
                for y in y0.max(0)..y1.min(c.height() as i32) {
                    for x in 0..w {
                        c.set_pixel(x as i32, y, Color::new(step as u8, y as u8, x as u8, 255));
                    }
                }
            }
            1 => {
                let r = Rect::new(rng.u(w) as f32 + 0.3, (scroll + rng.u(h) as i32) as f32 + 0.6, 1.0 + rng.u(40) as f32, 1.0 + rng.u(60) as f32);
                let ir = crate::pixel_rect(r, w, page.read().height());
                let c = page.write_rect(r);
                for y in ir.y0..ir.y1 {
                    for x in ir.x0..ir.x1 {
                        c.set_pixel(x as i32, y as i32, Color::new(x as u8, step as u8, y as u8, 255));
                    }
                }
            }
            2 => {
                let mut fresh = Canvas::new(w, 1500 + rng.u(1500));
                noise(&mut fresh, &mut rng, |r| r.byte());
                page.replace(fresh);
            }
            _ => scroll = (scroll + rng.u(120) as i32 - 40).clamp(-30, page.read().height() as i32),
        }
        // 偶尔跳过几帧不合成：日志里会攒下多次写入（超过 32 次时整体作废）
        if rng.u(5) == 0 {
            continue;
        }
        let l = FrameLayers {
            width: w,
            height: h,
            scale: 2.0,
            page: &page,
            scroll_px: scroll,
            page_bg: 0x334455,
            has_tabbar: false,
            tabbar: None,
            tabbar_px: 0,
            fixed: None,
            fixed_rows: None,
            pull: None,
            overlays: OverlayRefs::none(),
            edge_back: None,
        };
        check(&mut comp, &l, &format!("第 {step} 步"));
    }
}

/// 防空转：注入 `tint`（着色器让 R 通道 +1）时比较必须报不一致。
/// 否则上面两条「全绿」可能只是比较链路没接上（例如读回的其实是 CPU 结果）
#[test]
fn tint_fault_is_detected() {
    use crate::compositor::FaultSpec;
    let (w, h) = (64u32, 64u32);
    // 自检期间 tint 不生效，所以带着 tint 也能建出来
    let Some(mut comp) = open_with_fault(w, h, Some(FaultSpec::Tint)) else { return };
    let mut c = Canvas::new(w, 300);
    c.clear(Color::new(10, 20, 30, 255));
    let page = PageCanvas::new(c);
    let l = FrameLayers {
        width: w, height: h, scale: 2.0, page: &page, scroll_px: 0, page_bg: 0, has_tabbar: false,
        tabbar: None, tabbar_px: 0, fixed: None, fixed_rows: None, pull: None,
        overlays: OverlayRefs::none(), edge_back: None,
    };
    comp.compose_offscreen(&l, true).unwrap();
    let gpu = comp.read_back().unwrap();
    let mut buf = vec![0u32; (w * h) as usize];
    cpu::compose(&mut buf, &l);
    let m = first_mismatch(&cpu_rgba(&buf), &gpu, w).expect("tint 注入后必须不一致");
    assert_eq!((m.2, m.3), (0x0a141e, 0x0b141e), "差的应该恰好是 R 通道 +1");
}

/// 设备丢失注入：第 N 帧之后 compose 必须返回 DeviceLost（宿主据此当帧回退 CPU）
#[test]
fn lost_fault_surfaces_as_device_lost() {
    use crate::compositor::FaultSpec;
    let (w, h) = (32u32, 32u32);
    let Some(mut comp) = open_with_fault(w, h, Some(FaultSpec::Lost(2))) else { return };
    let page = PageCanvas::new(Canvas::new(w, 64));
    let l = FrameLayers {
        width: w, height: h, scale: 2.0, page: &page, scroll_px: 0, page_bg: 0, has_tabbar: false,
        tabbar: None, tabbar_px: 0, fixed: None, fixed_rows: None, pull: None,
        overlays: OverlayRefs::none(), edge_back: None,
    };
    assert!(comp.compose_offscreen(&l, true).is_ok(), "第 1 帧正常");
    let second = comp.compose_offscreen(&l, true);
    let third = comp.compose_offscreen(&l, true);
    let lost = [second, third].into_iter().any(|r| matches!(r, Err(super::GpuError::DeviceLost(_))));
    assert!(lost, "注入 lost@2 后必须报设备丢失");
}
