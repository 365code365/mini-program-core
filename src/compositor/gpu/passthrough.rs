//! 直通帧的上传：CPU 合成整帧（覆盖层要读目标像素、带浮点，GPU 复刻不了），但只把覆盖层
//! 画过的那几行传上去；其余行着色器照常合成，再按遮罩表压暗（`compositor::footprint`）。
//!
//! 为什么要收窄：一加 PLY110（Adreno 810）上 1080×2354 整帧上传 + 合成，操作菜单入场一帧
//! 9~20ms（`MINI_FPS` 显示「等 surface」为 0，全是这一段），比 CPU 后端（同一场景 6.8ms）还慢。
//! 操作菜单 / picker 面板弹着时只占屏幕下方一截，Modal / Toast 只是屏幕中间一个盒子。
//!
//! 逐字节一致靠的是：行区间外 CPU 写的恰好是「基础帧逐分量查表」（覆盖层自己报的，
//! 见 `Footprint`），基础帧 GPU 本来就与 CPU 逐字节相同，表由 CPU 用同一个算式算好。
//! CPU 那边也只合成这一截（[`cpu::overlay_reach`] 在画之前给出最大范围，`cpu::compose_rows`），
//! 一加上操作菜单一帧的 CPU 合成从 5~8ms 降到面板那三成。`MINI_GPU_VERIFY` 要整帧参照
//! （`last_passthrough`）时 `full_reference` 打开，CPU 改回整帧合成，GPU 照样只取那一截。
use super::pipeline::Tex;
use super::GpuCompositor;
use crate::compositor::{cpu, FrameLayers};

/// 直通帧里取 CPU 结果的行区间 [y0, y1) 与区间外的遮罩表
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PassBand {
    pub y0: u32,
    pub y1: u32,
    pub lut: Option<[u8; 256]>,
}

/// 覆盖层占了这么多行以上就整帧上传：省下的上传不多，还要多同步一遍其余图层
const WHOLE_FRAME_RATIO: f32 = 0.85;

impl GpuCompositor {
    /// CPU 合成这一帧并上传覆盖层那几行。返回（上传字节数, 是否整帧直通）。
    /// 整帧直通时着色器原样输出，调用方不必再同步其余图层
    pub(super) fn sync_pass(&mut self, l: &FrameLayers<'_>) -> (u64, bool) {
        // 与快照一致：输出缓冲初值 0（tabBar 画布比 tabbar_px 矮、导航栏半透明时那几行要从 0 起）。
        // 只清要合成的那几行：整块 10MB 每帧清一遍，一加上光这一步就 1~2ms
        let (w, n) = (l.width as usize, (l.width * l.height) as usize);
        self.pass_scratch.resize(n, 0);
        let t0 = std::time::Instant::now();
        // 覆盖层只占一截：CPU 也只合成那一截（逐帧校验要整帧参照时除外）
        let reach = if self.full_reference { None } else { cpu::overlay_reach(l) };
        self.sheet_frame = None;
        if let Some(layer) = reach.and_then(|_| cpu::sheet_layer(l)) {
            return self.sync_sheet_layer(l, layer, t0);
        }
        let small = |r: (u32, u32)| ((r.1 - r.0) as f32) < l.height as f32 * WHOLE_FRAME_RATIO;
        let buf = &mut self.pass_scratch;
        let whole_frame = |buf: &mut Vec<u32>| {
            buf.fill(0);
            cpu::compose(buf, l)
        };
        let fp = match reach.filter(|(r, _)| small(*r)) {
            Some((win, base)) => {
                // 只给要底图的那几行合成页面、压暗；其余行由不透明面板整行盖掉
                buf[base.0 as usize * w..base.1 as usize * w].fill(0);
                let fp = cpu::compose_rows(buf, l, base);
                // 安全网：画出来的超出了事先算的范围（不该发生）就整帧重来，宁慢不错
                let inside = fp.rows.is_none_or(|(a, b)| a >= win.0 && b <= win.1);
                if fp.full || !inside {
                    whole_frame(buf)
                } else {
                    fp
                }
            }
            None => whole_frame(buf),
        };
        // 逐帧校验时 CPU 是整帧合成的，GPU 照样只取覆盖层那一截：校验的正是「着色器在那一截外
        // 合成 + 查表」与 CPU 整帧的逐字节一致
        self.pass_compose_ms = t0.elapsed().as_secs_f32() * 1000.0;
        let (y0, y1) = fp.rows.unwrap_or((0, 0));
        let whole = fp.full || !small((y0, y1));
        self.pass_band = if whole { PassBand { y0: 0, y1: l.height, lut: None } } else { PassBand { y0, y1, lut: fp.lut } };
        let PassBand { y0, y1, .. } = self.pass_band;
        if y1 <= y0 {
            return (0, whole);
        }
        Tex::ensure(&mut self.pass, &self.gpu.device, "直通帧", l.width, l.height);
        // SAFETY: u32 切片按字节看（小端：B,G,R,0），长度 ×4，生命周期跟随 scratch
        let bytes = unsafe { std::slice::from_raw_parts(self.pass_scratch.as_ptr().cast::<u8>(), n * 4) };
        let sent = self.pass.as_ref().map(|t| t.upload_rows(&self.gpu.queue, bytes, l.width, y0, y1 - y0, l.width)).unwrap_or(0);
        (sent, whole)
    }

    /// 操作菜单是唯一的覆盖层：面板正文从缓存纹理贴（只在缓存换了时上传），CPU 只合成顶边圆角那几行
    /// （页面 + 压暗 + 圆角混白），其余行着色器合成再查表压暗。弹出 / 收起的每一帧只传几十行
    fn sync_sheet_layer(&mut self, l: &FrameLayers<'_>, (top, corner, rows, id): (i32, u32, u32, u64), t0: std::time::Instant) -> (u64, bool) {
        let (w, h) = (l.width as usize, l.height as i32);
        let y0 = top.clamp(0, h) as u32;
        let y1 = (top + corner as i32).clamp(0, h) as u32;
        let buf = &mut self.pass_scratch;
        buf[y0 as usize * w..y1 as usize * w].fill(0);
        crate::host::ui_overlay::action_sheet::set_panel_on_gpu(true);
        let fp = cpu::compose_rows(buf, l, (y0, y1));
        crate::host::ui_overlay::action_sheet::set_panel_on_gpu(false);
        self.pass_compose_ms = t0.elapsed().as_secs_f32() * 1000.0;
        self.pass_band = PassBand { y0, y1, lut: fp.lut };
        let mut sent = 0u64;
        if self.sheet_tex_id != Some(id) {
            let (dev, queue) = (&self.gpu.device, &self.gpu.queue);
            let tex = &mut self.sheet_tex;
            sent += crate::host::ui_overlay::action_sheet::with_raster_pixels(|pw, ph, px| {
                Tex::ensure(tex, dev, "操作菜单面板", pw, ph);
                // SAFETY: u32 切片按字节看（小端：B,G,R,0），与直通帧同一种布局
                let bytes = unsafe { std::slice::from_raw_parts(px.as_ptr().cast::<u8>(), px.len() * 4) };
                tex.as_ref().map(|t| t.upload_rows(queue, bytes, pw, 0, ph, pw)).unwrap_or(0)
            })
            .unwrap_or(0);
            self.sheet_tex_id = Some(id);
        }
        self.sheet_frame = Some((top, y1, (top + rows as i32).clamp(0, h) as u32));
        if y1 > y0 {
            Tex::ensure(&mut self.pass, &self.gpu.device, "直通帧", l.width, l.height);
            let n = w * l.height as usize;
            // SAFETY: 同上
            let bytes = unsafe { std::slice::from_raw_parts(self.pass_scratch.as_ptr().cast::<u8>(), n * 4) };
            sent += self.pass.as_ref().map(|t| t.upload_rows(&self.gpu.queue, bytes, l.width, y0, y1 - y0, l.width)).unwrap_or(0);
        }
        (sent, false)
    }
}
