//! 页面画布的 GPU 镜像：环形条带纹理 + 「哪些行在 GPU 上是最新的」记账（设计 §4.3）。
//!
//! 页面画布可以上万行高（3× 屏首页约 10305 行），超过常见的 8192 纹理上限，降级上限只有 2048。
//! 但真正会上屏的只有视口那一段，所以 GPU 只镜像「视口 ± 余量」：纹理是 `texture_2d_array`，
//! 每层 [`SLICE_ROWS`] 行，画布第 r 行放在环形下标 `r % ring_rows`。任何不超过环长的
//! 连续行区间都不会自相覆盖。
//!
//! 记账只有一个区间 `valid = [a, b)`：这些画布行在 GPU 上与 CPU 画布逐字节相同。
//! - 回放写入日志（[`PageCanvas::ops_since`]）：整宽条带写入把那些行从 `valid` 里**减掉**
//!   （不当场上传 —— 动画页面每帧都整带重画，整带上传是几 MB，而真正要采样的只是视口）；
//!   矩形写入只把与 `valid` 相交的部分**当场按矩形上传**（损伤区小，传矩形比传整行省）。
//! - 合成前把本帧要采样的行 `visible \ valid` 从**此刻的** CPU 画布补传上去。
//!
//! 最后一条让 GPU 输出的正确性不依赖「上屏行一定落在已绘制条带内」这条宿主不变量：
//! 宿主哪天漏了重画，GPU 采到的也是此刻 CPU 画布里的内容，和 CPU 上屏结果一样。
use crate::compositor::{color_bytes, DirtyOp, PageCanvas};

/// 环形纹理每层的行数（与着色器里的 SLICE_ROWS 一致）
pub const SLICE_ROWS: u32 = crate::compositor::select::RING_SLICE_ROWS;

/// 半开行区间 `[a, b)`
pub type Rows = (i32, i32);

fn len(r: Rows) -> i32 {
    (r.1 - r.0).max(0)
}

/// `valid` 减去 `[y0, y1)`。中间挖掉一段会裂成两段，只留**较长**的那段（只能记一个区间；
/// 丢掉的那段下次要采样时补传，结果仍然正确，只是多传了一点）
pub fn subtract(valid: Option<Rows>, cut: Rows) -> Option<Rows> {
    let v = valid?;
    if len(cut) == 0 || cut.1 <= v.0 || cut.0 >= v.1 {
        return Some(v);
    }
    let left = (v.0, cut.0.min(v.1));
    let right = (cut.1.max(v.0), v.1);
    let keep = if len(left) >= len(right) { left } else { right };
    (len(keep) > 0).then_some(keep)
}

/// `need \ valid`：最多两段
pub fn missing(valid: Option<Rows>, need: Rows) -> Vec<Rows> {
    if len(need) == 0 {
        return Vec::new();
    }
    let Some(v) = valid else { return vec![need] };
    let mut out = Vec::with_capacity(2);
    let left = (need.0, need.1.min(v.0));
    let right = (need.0.max(v.1), need.1);
    for r in [left, right] {
        if len(r) > 0 {
            out.push(r);
        }
    }
    out
}

/// 补传之后的有效区间：并集连续且不超过环长就取并集，否则只剩 `need`
/// （`need` 本身不超过环长，补传不会覆盖到其中已有效的行）
pub fn merged(valid: Option<Rows>, need: Rows, ring_rows: i32) -> Option<Rows> {
    if len(need) == 0 {
        return valid;
    }
    match valid {
        Some(v) if v.0 <= need.1 && need.0 <= v.1 => {
            let u = (v.0.min(need.0), v.1.max(need.1));
            Some(if len(u) <= ring_rows { u } else { need })
        }
        _ => Some(need),
    }
}

/// 画布 → 环形纹理的镜像
pub struct PageMirror {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub layers: u32,
    pub ring_rows: u32,
    /// 上次看到的写入游标 `(gen, seq)`
    pub cursor: (u64, u64),
    pub valid: Option<Rows>,
}

impl PageMirror {
    pub fn new(device: &wgpu::Device, width: u32, layers: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("页面环形镜像"),
            size: wgpu::Extent3d { width: width.max(1), height: SLICE_ROWS, depth_or_array_layers: layers.max(1) },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        Self { texture, view, width, layers, ring_rows: layers.max(1) * SLICE_ROWS, cursor: (0, 0), valid: None }
    }

    /// 回放 `page` 自上次以来的写入，再把本帧要采样的 `visible` 行补齐。返回上传的字节数
    pub fn sync(&mut self, queue: &wgpu::Queue, page: &PageCanvas, visible: Rows) -> u64 {
        let canvas_h = page.read().height() as i32;
        let mut bytes = 0u64;
        match page.ops_since(self.cursor) {
            None => self.valid = None,
            Some(ops) => {
                for op in ops {
                    match *op {
                        DirtyOp::Band(y0, y1) => {
                            self.valid = subtract(self.valid, (y0.max(0), y1.min(canvas_h)));
                        }
                        DirtyOp::Rect(r) => {
                            if r.is_empty() {
                                continue;
                            }
                            if let Some(v) = self.valid {
                                let (a, b) = ((r.y0 as i32).max(v.0), (r.y1 as i32).min(v.1));
                                if a < b {
                                    bytes += self.upload(queue, page, (a, b), (r.x0, r.x1));
                                }
                            }
                        }
                    }
                }
            }
        }
        self.cursor = page.cursor();
        let need = (visible.0.max(0), visible.1.min(canvas_h));
        if len(need) > self.ring_rows as i32 {
            // 环长按视口算过余量，到不了这里；万一到了只传能装下的那段（下一段会被截掉），并报出来
            crate::compositor::diag::emit(&format!("⚠ GPU 镜像：要采样 {} 行超过环长 {}", len(need), self.ring_rows));
        }
        let need = (need.0, need.1.min(need.0 + self.ring_rows as i32));
        for seg in missing(self.valid, need) {
            bytes += self.upload(queue, page, seg, (0, self.width.min(page.read().width())));
        }
        self.valid = merged(self.valid, need, self.ring_rows as i32);
        bytes
    }

    /// 把画布的 `rows × cols` 传到环里（跨层、跨环尾时按层切开）
    fn upload(&self, queue: &wgpu::Queue, page: &PageCanvas, rows: Rows, cols: (u32, u32)) -> u64 {
        let canvas = page.read();
        let cw = canvas.width();
        let (x0, x1) = (cols.0.min(cw).min(self.width), cols.1.min(cw).min(self.width));
        if x0 >= x1 {
            return 0;
        }
        let bytes = color_bytes(canvas.pixels());
        let mut sent = 0u64;
        let mut r = rows.0.max(0) as u32;
        let end = rows.1.max(0) as u32;
        while r < end {
            let k = r % self.ring_rows;
            let (layer, row_in) = (k / SLICE_ROWS, k % SLICE_ROWS);
            let n = (end - r).min(SLICE_ROWS - row_in);
            let offset = ((r as u64 * cw as u64) + x0 as u64) * 4;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x: x0, y: row_in, z: layer },
                    aspect: wgpu::TextureAspect::All,
                },
                &bytes[offset as usize..],
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(cw * 4), rows_per_image: None },
                wgpu::Extent3d { width: x1 - x0, height: n, depth_or_array_layers: 1 },
            );
            sent += (x1 - x0) as u64 * n as u64 * 4;
            r += n;
        }
        sent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtract_keeps_the_longer_side() {
        assert_eq!(subtract(Some((0, 100)), (40, 50)), Some((50, 100)));
        assert_eq!(subtract(Some((0, 100)), (60, 70)), Some((0, 60)));
        assert_eq!(subtract(Some((0, 100)), (0, 100)), None);
        assert_eq!(subtract(Some((0, 100)), (-5, 30)), Some((30, 100)));
        assert_eq!(subtract(Some((0, 100)), (200, 300)), Some((0, 100)));
        assert_eq!(subtract(None, (0, 10)), None);
        assert_eq!(subtract(Some((0, 100)), (50, 50)), Some((0, 100)));
    }

    #[test]
    fn missing_is_need_minus_valid() {
        assert_eq!(missing(None, (10, 20)), vec![(10, 20)]);
        assert_eq!(missing(Some((0, 100)), (10, 20)), Vec::<Rows>::new());
        assert_eq!(missing(Some((15, 18)), (10, 20)), vec![(10, 15), (18, 20)]);
        assert_eq!(missing(Some((0, 15)), (10, 20)), vec![(15, 20)]);
        assert_eq!(missing(Some((50, 60)), (10, 20)), vec![(10, 20)]);
        assert_eq!(missing(Some((0, 5)), (5, 5)), Vec::<Rows>::new());
    }

    #[test]
    fn merged_unions_only_when_contiguous_and_fits() {
        assert_eq!(merged(Some((0, 100)), (90, 150), 3072), Some((0, 150)));
        assert_eq!(merged(Some((0, 100)), (100, 150), 3072), Some((0, 150)));
        assert_eq!(merged(Some((0, 100)), (200, 250), 3072), Some((200, 250)));
        assert_eq!(merged(Some((0, 3000)), (2900, 3100), 3072), Some((2900, 3100)));
        assert_eq!(merged(None, (5, 9), 3072), Some((5, 9)));
    }

    /// 模拟一段滚动：每帧可见区下移 16 行，偶尔整带重画、矩形写入。
    /// 不变量：valid 永远不超过环长（否则环里会有两行互相覆盖）
    #[test]
    fn valid_never_exceeds_ring() {
        let ring = 3072;
        let mut valid: Option<Rows> = None;
        for f in 0..2000 {
            let top = f * 16;
            let need = (top, top + 1334);
            if f % 37 == 0 {
                valid = subtract(valid, (top - 400, top + 1734));
            }
            valid = merged(valid, need, ring);
            let v = valid.unwrap();
            assert!(len(v) <= ring, "帧 {f}: {v:?}");
            assert!(v.0 <= need.0 && need.1 <= v.1, "可见区必须有效");
        }
    }
}
