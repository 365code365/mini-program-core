//! 覆盖层单独成一层（预乘 RGBA）：WebGL 直接上屏时由 GPU 叠在游戏画面上（game/webgl/window.rs），游戏画面不必读回。
//!
//! ui_overlay 只会往不透明缓冲上混合，不出 alpha。这里黑底、白底各画一遍：source-over 下
//! 黑底结果 = α·C（就是预乘色），白底 − 黑底 = (1 − α)·255，于是 α = 255 − (白 − 黑)。三个通道取平均抵消舍入。
//! 只在覆盖层弹着的那几帧跑（两遍合成尺寸的绘制，min(dpr, 2) 倍）。

use super::overlay::compose_size;
use super::GameEngine;

impl GameEngine {
    /// 覆盖层（含原生按钮）的预乘 RGBA 与它的尺寸（合成尺寸）
    pub fn overlay_layer(&self) -> (u32, u32, Vec<u8>) {
        let (w, h, _) = compose_size(self.width, self.height, self.dpr);
        let n = (w * h) as usize;
        let mut black = Vec::new();
        let mut white = Vec::new();
        self.compose_on(&[0u8, 0, 0, 255].repeat(n), &mut black);
        self.compose_on(&[255u8, 255, 255, 255].repeat(n), &mut white);
        let mut out = vec![0u8; n * 4];
        for ((o, b), wt) in out.chunks_exact_mut(4).zip(black.chunks_exact(4)).zip(white.chunks_exact(4)) {
            let diff = (wt[0] as i32 - b[0] as i32) + (wt[1] as i32 - b[1] as i32) + (wt[2] as i32 - b[2] as i32);
            let a = (255 - (diff + 1) / 3).clamp(0, 255) as u8;
            // 预乘色不能超过 alpha（舍入可能让黑底结果比 α 大 1）
            o.copy_from_slice(&[b[0].min(a), b[1].min(a), b[2].min(a), a]);
        }
        (w, h, out)
    }
}

#[cfg(test)]
mod tests {
    /// 差值求 alpha 的算术：α·C 压黑底 / 白底，反推回来误差不超过 1
    #[test]
    fn 黑白两底反推预乘色与透明度() {
        for (c, a) in [(200u32, 128u32), (17, 179), (255, 255), (0, 0), (90, 30)] {
            let black = (c * a + 127) / 255;
            let white = (c * a + 255 * (255 - a) + 127) / 255;
            let diff = 3 * (white as i32 - black as i32);
            let ra = (255 - (diff + 1) / 3).clamp(0, 255) as u32;
            assert!(ra.abs_diff(a) <= 1, "c={c} a={a} → {ra}");
            assert!(black <= ra + 1);
        }
    }
}
