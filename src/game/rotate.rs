//! 引擎内旋转（game-doc/04 G6.4，GT-O2）：横屏包嵌在宿主转不了方向的地方（竖屏锁定的页面里），
//! 创建选项 `rotateInEngine: true` 时引擎自己转：
//! - 游戏看到的窗口是横的（宿主给的宽高对调），getSystemInfo / createCanvas 都按横屏；
//! - 输出帧整帧转 90°：`landscape` / `landscapeRight`（home 键在右 = 设备逆时针转过去）内容**顺时针**转，
//!   游戏的上边贴设备的右边；`landscapeLeft` 反过来；
//! - 触点反变换回游戏坐标。
//!
//! 坐标（连续坐标，宿主竖屏宽 Wd = 游戏高 Hg）：
//! - 顺时针：设备 (xd, yd) = (Wd − y, x)，所以游戏 (x, y) = (yd, Wd − xd)；像素 (x, y) → (Wd − 1 − y, x)
//! - 逆时针：设备 (xd, yd) = (y, Hd − x)，所以游戏 (x, y) = (Hd − yd, xd)；像素 (x, y) → (y, Hd − 1 − x)

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    Clockwise,
    CounterClockwise,
}

impl Rotation {
    /// 宿主坐标（竖屏，宽 `wd` 高 `hd`）→ 游戏坐标
    pub fn to_game(self, x: f32, y: f32, wd: f32, hd: f32) -> (f32, f32) {
        match self {
            Rotation::Clockwise => (y, wd - x),
            Rotation::CounterClockwise => (hd - y, x),
        }
    }

    /// 把 `w × h` 的 RGBA 转进 `out`（`h × w`）
    pub fn rotate_rgba(self, src: &[u8], w: u32, h: u32, out: &mut Vec<u8>) {
        let (w, h) = (w as usize, h as usize);
        out.resize(w * h * 4, 0);
        // 按输出行写（顺序写目的地，读跳着读）：输出宽 = h
        for oy in 0..w {
            for ox in 0..h {
                let (x, y) = match self {
                    // 输出 (ox, oy) = (Wd − 1 − y, x) → x = oy, y = h − 1 − ox
                    Rotation::Clockwise => (oy, h - 1 - ox),
                    // 输出 (ox, oy) = (y, w − 1 − x) → x = w − 1 − oy, y = ox
                    Rotation::CounterClockwise => (w - 1 - oy, ox),
                };
                let s = (y * w + x) * 4;
                let d = (oy * h + ox) * 4;
                out[d..d + 4].copy_from_slice(&src[s..s + 4]);
            }
        }
    }
}

/// 选项 + 包方向 + 宿主尺寸 → 要不要转、往哪转。宿主本来就给了横屏尺寸（它能转）就不转
pub fn decide(rotate_in_engine: bool, orientation: super::config::Orientation, width: u32, height: u32) -> Option<Rotation> {
    use super::config::Orientation::*;
    if !rotate_in_engine || width > height {
        return None;
    }
    match orientation {
        Portrait => None,
        Landscape | LandscapeRight => Some(Rotation::Clockwise),
        LandscapeLeft => Some(Rotation::CounterClockwise),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 像素旋转与触点反变换一致() {
        // 3×2 的图，每个像素的 R 是它的序号
        let src: Vec<u8> = (0..6u8).flat_map(|i| [i, 0, 0, 255]).collect();
        for r in [Rotation::Clockwise, Rotation::CounterClockwise] {
            let mut out = Vec::new();
            r.rotate_rgba(&src, 3, 2, &mut out);
            // 输出 2×3：每个输出像素中心按 to_game 反变换回去，应当落在源里同一个像素
            for oy in 0..3 {
                for ox in 0..2 {
                    let (gx, gy) = r.to_game(ox as f32 + 0.5, oy as f32 + 0.5, 2.0, 3.0);
                    let want = (gy as usize) * 3 + gx as usize;
                    assert_eq!(out[(oy * 2 + ox) * 4] as usize, want, "{r:?} ({ox},{oy})");
                }
            }
        }
        // 顺时针：游戏左上 (0,0) 到设备右上
        let mut out = Vec::new();
        Rotation::Clockwise.rotate_rgba(&src, 3, 2, &mut out);
        assert_eq!(out[4], 0, "设备 (1,0) 是游戏 (0,0)");
    }
}
