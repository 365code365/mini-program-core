//! 像素出口：把合成缓冲交给宿主。
//!
//! 合成缓冲是 `u32 = 0x00RRGGBB`（与桌面 softbuffer 同一种布局），小端内存里正好是
//! B G R X —— 所以 BGRA 出口只要把 alpha 补成 FF，RGBA 出口要交换 R/B。
//!
//! ## 为什么要能直写平台缓冲
//! 从前每帧是「合成 → 整帧转 RGBA → `mr_app_pixels` 再整帧拷给宿主 → 宿主再拷进
//! Bitmap / CGImage」。3× 屏一帧 12MB，实测转换 + 拷贝两步 p50 合计约 1~1.8ms（M3），
//! 手机更慢。`pixels_into` 一步写进宿主给的缓冲（CVPixelBuffer / ANativeWindow /
//! Bitmap 的像素内存），中间那份 RGBA 也只在有人要时才转（[`MiniEngine::rgba`]）。
//!
//! 平台要的格式不同：Flutter 在 iOS/macOS 的外部纹理只认 32BGRA（或 NV12），
//! Android 的 Bitmap ARGB_8888 / ANativeWindow RGBA_8888 内存序是 RGBA。
use super::engine::MiniEngine;

/// 输出像素格式（每像素 4 字节，alpha 恒为 255）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    /// 内存序 R G B A
    Rgba8 = 0,
    /// 内存序 B G R A
    Bgra8 = 1,
}

impl PixelFormat {
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Rgba8),
            1 => Some(Self::Bgra8),
            _ => None,
        }
    }
}

/// 把一行 `0x00RRGGBB` 写成目标格式
#[inline]
fn write_row(dst: &mut [u8], src: &[u32], format: PixelFormat) {
    match format {
        PixelFormat::Bgra8 => {
            for (d, px) in dst.chunks_exact_mut(4).zip(src) {
                d.copy_from_slice(&(px | 0xFF00_0000).to_le_bytes());
            }
        }
        PixelFormat::Rgba8 => {
            for (d, px) in dst.chunks_exact_mut(4).zip(src) {
                d.copy_from_slice(&[(px >> 16) as u8, (px >> 8) as u8, *px as u8, 255]);
            }
        }
    }
}

impl MiniEngine {
    /// 当前帧的 RGBA8 像素（宽 = 逻辑宽 × dpr，紧排）。按需从合成缓冲转换。
    pub fn rgba(&mut self) -> &[u8] {
        #[cfg(feature = "gpu")]
        self.sync_cpu_frame();
        let n = self.compose.len();
        if self.rgba.len() != n * 4 {
            self.rgba = vec![255u8; n * 4];
            self.rgba_stale = true;
        }
        if self.rgba_stale {
            let (pw, _) = self.pixel_size();
            let pw = (pw as usize).max(1);
            let Self { rgba, compose, .. } = self;
            for (dst, src) in rgba.chunks_exact_mut(pw * 4).zip(compose.chunks_exact(pw)) {
                write_row(dst, src, PixelFormat::Rgba8);
            }
            self.rgba_stale = false;
        }
        &self.rgba
    }

    /// 把当前帧写进宿主的缓冲：`stride` 是每行字节数（≥ 宽 × 4，平台缓冲常有行尾填充）。
    ///
    /// 返回写到的最远字节（`stride × (高 − 1) + 宽 × 4`）；还没出过帧、`stride` 太小、
    /// `dst` 不够长时返回 0 且一个字节都不写（宿主算错尺寸时不能越界）。
    pub fn write_pixels(&mut self, dst: &mut [u8], stride: usize, format: PixelFormat) -> usize {
        // GPU 上屏的帧没做 CPU 合成：先补（engine_gpu.rs）
        #[cfg(feature = "gpu")]
        self.sync_cpu_frame();
        let (pw, ph) = self.pixel_size();
        let (pw, ph) = (pw as usize, ph as usize);
        let row = pw * 4;
        if pw == 0 || ph == 0 || self.compose.len() != pw * ph || stride < row {
            return 0;
        }
        let need = stride * (ph - 1) + row;
        if dst.len() < need {
            return 0;
        }
        for (y, src) in self.compose.chunks_exact(pw).enumerate() {
            let off = y * stride;
            write_row(&mut dst[off..off + row], src, format);
        }
        need
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 行内格式与旧的整帧转换一致() {
        let src = [0x00112233u32, 0x00FFFFFF, 0x00000000];
        let mut rgba = [0u8; 12];
        let mut bgra = [0u8; 12];
        write_row(&mut rgba, &src, PixelFormat::Rgba8);
        write_row(&mut bgra, &src, PixelFormat::Bgra8);
        assert_eq!(&rgba[..4], &[0x11, 0x22, 0x33, 255]);
        assert_eq!(&bgra[..4], &[0x33, 0x22, 0x11, 255]);
        assert_eq!(&rgba[4..8], &[255, 255, 255, 255]);
        assert_eq!(&bgra[8..], &[0, 0, 0, 255]);
    }

    #[test]
    fn 格式码() {
        assert_eq!(PixelFormat::from_code(0), Some(PixelFormat::Rgba8));
        assert_eq!(PixelFormat::from_code(1), Some(PixelFormat::Bgra8));
        assert_eq!(PixelFormat::from_code(7), None);
    }
}
