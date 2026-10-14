//! 几何图形模块

/// 2D 点
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance(&self, other: &Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn lerp(&self, other: &Point, t: f32) -> Point {
        Point {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

/// 尺寸
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// 矩形
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub fn from_points(p1: Point, p2: Point) -> Self {
        let x = p1.x.min(p2.x);
        let y = p1.y.min(p2.y);
        let width = (p1.x - p2.x).abs();
        let height = (p1.y - p2.y).abs();
        Self { x, y, width, height }
    }

    pub fn left(&self) -> f32 { self.x }
    pub fn top(&self) -> f32 { self.y }
    pub fn right(&self) -> f32 { self.x + self.width }
    pub fn bottom(&self) -> f32 { self.y + self.height }

    pub fn contains(&self, point: &Point) -> bool {
        point.x >= self.x && point.x <= self.right() &&
        point.y >= self.y && point.y <= self.bottom()
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right() && self.right() > other.x &&
        self.y < other.bottom() && self.bottom() > other.y
    }

    pub fn inset(&self, dx: f32, dy: f32) -> Rect {
        Rect {
            x: self.x + dx,
            y: self.y + dy,
            width: (self.width - 2.0 * dx).max(0.0),
            height: (self.height - 2.0 * dy).max(0.0),
        }
    }
}

/// 整数像素矩形，半开区间 `[x0, x1) × [y0, y1)`，已夹到某块画布内。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IRect {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl IRect {
    /// 一个像素都不覆盖（含负宽高的浮点矩形取整后的结果）
    pub fn is_empty(&self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }
}

/// 浮点矩形覆盖到的整数像素：起点向下取整、终点向上取整，再夹到 `w × h` 画布内。
///
/// `Canvas::clear_area` 按它清屏；记账「这一帧改写了哪块像素」的
/// `compositor::PageCanvas::write_rect`（给 GPU 镜像按脏区上传用）也按它算。
/// 两边用同一个函数，才不会出现「清了第 N 列、记账却只记到第 N-1 列」这种差一像素的
/// 漏记（各自对 `x`、`x + width` 取整的写法稍有不同，结果就可能不一样）。
pub fn pixel_rect(r: Rect, w: u32, h: u32) -> IRect {
    IRect {
        x0: r.x.floor().clamp(0.0, w as f32) as u32,
        x1: r.right().ceil().clamp(0.0, w as f32) as u32,
        y0: r.y.floor().clamp(0.0, h as f32) as u32,
        y1: r.bottom().ceil().clamp(0.0, h as f32) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Canvas, Color};

    /// `clear_area` 改动过的像素必须落在 `pixel_rect` 里：脏区记账按 `pixel_rect` 算，
    /// 清屏多出一列就是一列没人知道的脏像素。
    #[test]
    fn clear_area_writes_only_inside_pixel_rect() {
        let (w, h) = (37u32, 23u32);
        let (base, ink) = (Color::new(1, 2, 3, 4), Color::new(250, 251, 252, 253));
        // 固定种子的 LCG：小数坐标、负坐标、出界、负宽高、零宽高都会出现
        let mut s = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = |lo: f32, hi: f32| {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            lo + (hi - lo) * ((s >> 40) as f32 / (1u64 << 24) as f32)
        };
        let mut touched_any = 0;
        for _ in 0..2000 {
            let r = Rect::new(
                next(-20.0, w as f32 + 20.0),
                next(-20.0, h as f32 + 20.0),
                next(-5.0, 30.0),
                next(-5.0, 20.0),
            );
            let mut c = Canvas::new(w, h);
            c.clear(base);
            c.clear_area(&r, ink);
            let p = pixel_rect(r, w, h);
            for y in 0..h {
                for x in 0..w {
                    if c.get_pixel(x, y) != base {
                        touched_any += 1;
                        assert!(
                            x >= p.x0 && x < p.x1 && y >= p.y0 && y < p.y1,
                            "clear_area({r:?}) 改了 ({x},{y})，不在 pixel_rect {p:?} 内"
                        );
                    }
                }
            }
        }
        // 防空转：随机矩形里总有落在画布内的，一个像素都没改说明测试本身坏了
        assert!(touched_any > 0);
    }

    #[test]
    fn pixel_rect_floors_start_ceils_end_and_clamps() {
        let p = pixel_rect(Rect::new(1.5, 2.2, 3.0, 0.1), 10, 10);
        assert_eq!(p, IRect { x0: 1, y0: 2, x1: 5, y1: 3 });
        let p = pixel_rect(Rect::new(-3.0, 8.5, 100.0, 100.0), 10, 10);
        assert_eq!(p, IRect { x0: 0, y0: 8, x1: 10, y1: 10 });
        assert!(pixel_rect(Rect::new(4.0, 4.0, -1.0, 2.0), 10, 10).is_empty());
        assert!(pixel_rect(Rect::new(4.0, 4.0, 0.0, 0.0), 10, 10).is_empty());
    }
}
