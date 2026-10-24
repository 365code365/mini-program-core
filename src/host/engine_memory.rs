//! 引擎的内存分项：每块大缓冲分配了多少、真正驻留多少（`mem_probe::resident`）。
//!
//! 「内存大」先看这一张表再动手：字体、画布、合成缓冲、侧滑截图、留着的页面、scroll-view 离屏缓存、
//! 图片缓存、JS 堆各占多少。宿主 / 示例用 [`MiniEngine::memory_breakdown`]，`MINI_MEM_LOG=1` 时每次进后台打一行
use super::engine::MiniEngine;
use crate::mem_probe::resident;

/// 上次还页时留下的行区间（页面画布：已绘制条带；覆盖层：有非零字节的行）与当时画布的代数 / 高度
#[derive(Default)]
pub struct Kept {
    page: Option<(u64, u32, u32)>,
    fixed: Option<(u64, u32, u32)>,
}

/// 区间变化不到这么多行就不再还（madvise 本身几十微秒，别每帧调）
const SLACK_ROWS: u32 = 256;

impl MiniEngine {
    /// 页面画布：已绘制条带以外的行内容本来就不算数（读之前一定先清再画，engine_frame.rs `render_scoped`），
    /// 那些页还给系统。长页面滑过一遍，从前整页都留在内存里（「综合场景」页 30MB），现在只留条带那段。
    /// 覆盖层：全是 0 的行（弹窗关掉之后整屏的空行）还给系统，再读还是 0，内容不变
    pub(super) fn release_idle_rows(&mut self, fixed_nonzero: Option<Option<(u32, u32)>>) {
        if let Some((b0, b1)) = self.drawn_band {
            let gen = self.canvas.gen();
            let keep = (b0.floor().max(0.0) as u32, b1.ceil() as u32);
            let moved = match self.mem_kept.page {
                Some((g, k0, k1)) => g != gen || k0.abs_diff(keep.0) > SLACK_ROWS || k1.abs_diff(keep.1) > SLACK_ROWS,
                None => true,
            };
            if moved {
                self.canvas.release_outside(keep.0, keep.1);
                self.mem_kept.page = Some((gen, keep.0, keep.1));
            }
        }
        if let Some(nz) = fixed_nonzero {
            let gen = self.fixed_canvas.gen();
            let h = self.fixed_canvas.read().height();
            // 一行非零字节都没有：整层还掉
            let keep = nz.map(|(a, b)| (a, b + 1)).unwrap_or((0, 0));
            let _ = h;
            // 非零的行比上次留的少了一截（弹窗关了、悬浮条缩了）才还；只是在里面变就不动
            let changed = match self.mem_kept.fixed {
                Some((g, k0, k1)) => g != gen && (keep.0 > k0 + SLACK_ROWS / 4 || keep.1 + SLACK_ROWS / 4 < k1),
                None => true,
            };
            if changed {
                self.fixed_canvas.release_zero_rows_outside(keep.0, keep.1);
                self.mem_kept.fixed = Some((gen, keep.0, keep.1));
            } else if let Some(k) = self.mem_kept.fixed.as_mut() {
                // 区间变大了：记下更大的那段，下次缩回来时才还
                *k = (gen, k.1.min(keep.0), k.2.max(keep.1));
            }
        }
    }
}

/// 一项：名字、分配的字节、驻留的字节
#[derive(Debug, Clone)]
pub struct MemItem {
    pub name: &'static str,
    pub alloc: usize,
    pub resident: usize,
}

impl MiniEngine {
    pub fn memory_breakdown(&self) -> Vec<MemItem> {
        let mut out = Vec::new();
        let mut add = |name: &'static str, alloc: usize, res: usize| out.push(MemItem { name, alloc, resident: res });
        let page = self.canvas.read().pixels();
        add("页面画布", std::mem::size_of_val(page), resident(page));
        let fixed = self.fixed_canvas.read().pixels();
        add("覆盖层画布", std::mem::size_of_val(fixed), resident(fixed));
        let tab = self.tabbar_canvas.read().pixels();
        add("tabBar 画布", std::mem::size_of_val(tab), resident(tab));
        if let Some(px) = self.nav_canvas_pixels() {
            add("导航栏画布", std::mem::size_of_val(px), resident(px));
        }
        add("合成缓冲", self.compose.capacity() * 4, resident(&self.compose[..]));
        add("RGBA 读出缓冲", self.rgba.capacity(), resident(&self.rgba[..]));
        let shots: (usize, usize) = self.back_shots.iter().fold((0, 0), |a, s| (a.0 + s.bytes(), a.1 + resident(&s.pixels[..]) + s.packed.as_ref().map_or(0, |b| b.len())));
        add("侧滑返回截图", shots.0, shots.1);
        let mut sv = (0, 0);
        let mut renderers: Vec<&crate::renderer::WxmlRenderer> = self.renderer.iter().chain(self.tabbar_renderer.iter()).collect();
        let kept: Vec<&crate::renderer::WxmlRenderer> = self
            .keep
            .saved
            .iter()
            .filter_map(|v| v.renderer.as_ref())
            .chain(self.keep.tabs.values().filter_map(|(_, v)| v.renderer.as_ref()))
            .collect();
        let mut kept_sv = (0, 0);
        for r in &kept {
            for b in r.offscreen_buffers() {
                kept_sv.0 += std::mem::size_of_val(b);
                kept_sv.1 += resident(b);
            }
        }
        for r in renderers.drain(..) {
            for b in r.offscreen_buffers() {
                sv.0 += std::mem::size_of_val(b);
                sv.1 += resident(b);
            }
        }
        add("scroll-view 离屏缓存", sv.0, sv.1);
        add("留着的页面的离屏缓存", kept_sv.0, kept_sv.1);
        let (img, anim) = crate::renderer::components::image_cache_bytes();
        add("解码图片", img, img);
        add("动图帧", anim, anim);
        let (scaled, mip) = crate::canvas::scaled_image_cache_bytes();
        add("图片缩放 / mip 缓存", scaled + mip, scaled + mip);
        if let Some(t) = crate::text::shared_fonts() {
            let m = t.font_memory();
            add("字体（几何 + 表）", m.geometry + m.tables, m.geometry + m.tables);
            add("字形位图缓存", m.bitmaps.1, m.bitmaps.1);
        }
        let g = crate::renderer::components::gradient::row_cache::bytes();
        add("渐变行缓存", g, g);
        let (js, _) = self.app.js_memory();
        add("JS 堆（QuickJS）", js, js);
        out
    }

    /// 一行摘要（按驻留量从大到小）
    pub fn memory_report_line(&self) -> String {
        let mut v = self.memory_breakdown();
        v.sort_by(|a, b| b.resident.cmp(&a.resident));
        let mb = |b: usize| b as f32 / 1048576.0;
        let total: usize = v.iter().map(|i| i.resident).sum();
        let parts: Vec<String> = v.iter().filter(|i| i.alloc > 0).map(|i| format!("{} {:.1}/{:.1}", i.name, mb(i.resident), mb(i.alloc))).collect();
        format!("内存（驻留/分配 MB）合计 {:.1}：{}", mb(total), parts.join("，"))
    }
}
