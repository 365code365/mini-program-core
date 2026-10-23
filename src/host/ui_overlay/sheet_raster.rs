//! 操作菜单面板「画一次、挪着用」。
//!
//! 入场 / 退场 0.3s 里面板内容（白底、文字、分隔线、按压底）一个像素不变，只是整块上下挪。
//! 从前每一帧都整块重画（文字重新光栅化、白底整块填、整块上传 GPU）：一加 PLY110 上 GPU 直通帧里
//! CPU 合成 3~10ms，弹出时屏上 11~17% 的 vsync 重复上一张（SurfaceFlinger `--latency` 量的）。
//!
//! 现在面板状态（选项、按压、尺寸）变了才画一次，存成以「画的那一刻的顶边」为原点的一张图；
//! 之后每帧按**整物理像素**的位移贴上去（CPU 参照路径 [`SheetRaster::blit`]），GPU 把同一张图当纹理
//! 在着色器里挪（`compositor::gpu`）—— 两边贴的是同一张图，逐字节一致。
//!
//! 只有顶边圆角那几行与底下的页面有关（角上抗锯齿要与压暗后的页面混合）：这几行每帧照常在 CPU 上
//! 用 `fill_rrect_aa_rows` 混（同一套算式、整像素平移），面板内容在这几行的像素再盖上去。
//! 判断「哪些像素与底下有关」是实测的：同一个圆角白底分别画在黑底和白底上，两次不一样的就是。
//!
//! 取舍：面板按整像素一档挪（dpr 3 上 1/3 逻辑像素），子像素位置定格在画的那一帧。看不出来，
//! 而且画的那一帧与贴的那一帧位置相同时（静止的面板、gallery 的 `20_action_sheet`）与直接画逐字节相同。
//!
//! ## 画这一张要快（弹出的第一帧就在画它）
//! 面板按**绝对坐标**画在一块整屏大小的画布上（与直接画同一组坐标，才逐字节相同），缓存直接留着这块画布、
//! 只认第 `t0` 行往下那一截。从前另有一块整屏草稿：第一次用时 `resize` 把 10MB 逐个写零，画完再 `to_vec`
//! 拷出那一截 —— 一加 PLY110 上弹出第一帧光画这一张就 7.5~10ms（一个 90Hz 周期 11.1ms）。
//! 现在画布用 `vec![0; n]`（calloc，没碰过的页不占内存也不用写），面板上面那几行从来不碰；
//! 换一张时把旧缓存的画布接着用（按一下选项就要重画一次）。
use super::draw::fill_rrect_aa_rows;

/// 面板缓存：画布里从第 `t0` 行（画的那一刻面板顶边所在的整像素行）到屏幕底的那一截
pub struct SheetRaster {
    /// 内容变了就换一个（GPU 据此决定要不要重新上传）。进程里唯一（见 `action_sheet::NEXT_RASTER`）
    pub id: u64,
    pub key: RasterKey,
    pub width: u32,
    /// 画的那一刻的顶边（逻辑像素）与它所在的整像素行
    top: f32,
    t0: i32,
    /// 存了多少行（`h - t0`）
    pub rows: u32,
    /// 前多少行里有与底下有关的像素（圆角）：这几行每帧在 CPU 上混
    pub corner_rows: u32,
    /// 整屏大小的画布（0x00RRGGBB，文字像素高字节 0xFF，与直接画一样）：只有第 `t0` 行往下是面板，
    /// 上面那几行没写过（见文件头「画这一张要快」）
    canvas: Vec<u32>,
    /// 前 `corner_rows` 行里哪些像素是面板内容（照抄），其余由圆角白底每帧现混
    content: Vec<bool>,
    /// 画白底用的参数（物理像素）：每帧混圆角时原样再画一次
    rect: (f32, f32, f32, f32),
    radius: f32,
}

/// 决定面板长什么样的那些东西：变了才重画
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RasterKey {
    pub sheet: u64,
    pub pressed: Option<i32>,
    pub size: (u32, u32),
    pub sf_bits: u32,
    pub has_text: bool,
}

/// 面板这一帧贴在哪：顶边整像素行 + 从第几行起由图（GPU 纹理）提供
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub top_row: i32,
    pub corner_rows: u32,
    pub rows: u32,
}

impl SheetRaster {
    /// 画一张缓存。`draw(buf)` 按顶边 `top`（逻辑像素）把整个面板画进 `w × h` 的缓冲（与直接画同一个函数）；
    /// `rect` / `radius` 是其中圆角白底的参数（物理像素）。`canvas` 是上一张缓存还回来的画布
    /// （[`into_canvas`](Self::into_canvas)），没有就给空的
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        id: u64,
        key: RasterKey,
        (w, h): (u32, u32),
        sf: f32,
        top: f32,
        rect: (f32, f32, f32, f32),
        radius: f32,
        mut canvas: Vec<u32>,
        draw: impl FnOnce(&mut [u32]),
    ) -> Self {
        let t0 = (top * sf).floor() as i32;
        let start = t0.clamp(0, h as i32) as u32;
        let rows = h - start;
        let n = (w * h) as usize;
        let wu = w as usize;
        if canvas.len() != n {
            // calloc：整屏 10MB 只有面板那一截会被碰到（`resize` 会把整块逐个写一遍零）
            canvas = vec![0u32; n];
        }
        let lo = start as usize * wu;
        // ① 圆角白底单独画在黑底、白底上：两次不一样的像素与底下有关。
        //    先在画布的这几行上做（之后这几行要画面板），与从前在另一块草稿上做是同一组坐标、同一个结果
        let k = ((radius.ceil() as u32) + 2).min(rows);
        let band = (start, start + k);
        let (a, b) = (lo, (start + k) as usize * wu);
        canvas[a..b].fill(0);
        fill_rrect_aa_rows(&mut canvas, w, h, rect, radius, 0xFFFFFF, 1.0, [true, true, false, false], band);
        let on_black = canvas[a..b].to_vec();
        canvas[a..b].fill(0xFFFFFF);
        fill_rrect_aa_rows(&mut canvas, w, h, rect, radius, 0xFFFFFF, 1.0, [true, true, false, false], band);
        let depends: Vec<bool> = canvas[a..b].iter().zip(&on_black).map(|(on_white, on_black)| on_white != on_black).collect();
        // ② 面板本体
        canvas[lo..].fill(0);
        draw(&mut canvas);
        // ③ 与底下有关的像素：面板内容盖过它（按压底那种整块填色）就照抄内容，否则每帧现混
        let mut corner_rows = 0u32;
        let mut content = vec![true; (k as usize) * wu];
        for (i, keep) in content.iter_mut().enumerate() {
            if depends[i] {
                *keep = canvas[lo + i] != on_black[i];
                if !*keep {
                    corner_rows = corner_rows.max(i as u32 / w + 1);
                }
            }
        }
        content.truncate((corner_rows * w) as usize);
        Self { id, key, width: w, top, t0: start as i32, rows, corner_rows, canvas, content, rect, radius }
    }

    /// 缓存的像素（`width × rows`，第 0 行是画的那一刻的顶边行）
    pub fn pixels(&self) -> &[u32] {
        &self.canvas[self.t0 as usize * self.width as usize..]
    }

    /// 换下来的缓存把画布还回来，下一张接着画在上面
    pub fn into_canvas(self) -> Vec<u32> {
        self.canvas
    }

    /// 顶边移到 `top`（逻辑像素）时贴在哪：按画的那一刻的位置整像素平移
    pub fn place(&self, top: f32, sf: f32) -> Placement {
        let shift = ((top - self.top) * sf).round() as i32;
        Placement { top_row: self.t0 + shift, corner_rows: self.corner_rows, rows: self.rows }
    }

    /// 贴进缓冲（行宽 `w`、高 `h`），只写 `rows` 那几行 [a, b)。圆角那几行先把白底混在
    /// （已经压暗的）底图上，再盖面板内容；其余整行照抄
    pub fn blit(&self, buf: &mut [u32], w: u32, h: u32, p: Placement, rows: (u32, u32)) {
        if w != self.width {
            return;
        }
        let shift = (p.top_row - self.t0) as f32;
        let wu = w as usize;
        let lo = p.top_row.max(rows.0 as i32).max(0);
        let hi = (p.top_row + self.rows as i32).min(rows.1 as i32).min(h as i32);
        if hi <= lo {
            return;
        }
        let corner_end = (p.top_row + self.corner_rows as i32).min(hi);
        if corner_end > lo {
            let r = (self.rect.0, self.rect.1 + shift, self.rect.2, self.rect.3);
            fill_rrect_aa_rows(buf, w, h, r, self.radius, 0xFFFFFF, 1.0, [true, true, false, false], (lo as u32, corner_end as u32));
        }
        let pixels = self.pixels();
        for y in lo..hi {
            let ly = (y - p.top_row) as usize;
            let dst = &mut buf[y as usize * wu..(y as usize + 1) * wu];
            let src = &pixels[ly * wu..(ly + 1) * wu];
            if (ly as u32) < self.corner_rows {
                let keep = &self.content[ly * wu..(ly + 1) * wu];
                for x in 0..wu {
                    if keep[x] {
                        dst[x] = src[x];
                    }
                }
            } else {
                dst.copy_from_slice(src);
            }
        }
    }
}
