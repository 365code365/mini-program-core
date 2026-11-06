//! 颜色模块

/// RGBA 颜色
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
            a: 255,
        }
    }

    /// 预混合 alpha
    pub fn premultiply(&self) -> Self {
        let a = self.a as f32 / 255.0;
        Self {
            r: (self.r as f32 * a) as u8,
            g: (self.g as f32 * a) as u8,
            b: (self.b as f32 * a) as u8,
            a: self.a,
        }
    }

    /// Alpha 混合 (使用整数运算优化)
    #[inline]
    pub fn blend(&self, dst: &Color) -> Color {
        // 如果源完全透明，返回目标色
        if self.a == 0 { return *dst; }
        // 目标完全透明：下面的通用公式算出来就是源色本身（out_a = src_a，颜色 = src·src_a / src_a），
        // 省掉三次按 out_a 的除法。透明底的离屏画布（小游戏的界面层、2D 的 UI 画布）大面积都是这种：
        // 萌宠小岛开一次菜单，铺一块半透明面板就是几百万次，iPhone 上一次 fill 4ms
        if dst.a == 0 { return *self; }
        
        // 针对目标是完全不透明的常见情况（如背景）进行优化
        if dst.a == 255 {
            if self.a == 255 { return *self; }
            
            let alpha = self.a as u32;
            let inv_alpha = 255 - alpha;
            
            return Color {
                r: ((self.r as u32 * alpha + dst.r as u32 * inv_alpha) / 255) as u8,
                g: ((self.g as u32 * alpha + dst.g as u32 * inv_alpha) / 255) as u8,
                b: ((self.b as u32 * alpha + dst.b as u32 * inv_alpha) / 255) as u8,
                a: 255,
            };
        }

        // 通用混合模式 (支持半透明目标)
        let src_a = self.a as u32;
        let dst_a = dst.a as u32;
        let inv_src_a = 255 - src_a;
        
        // out_a = src_a + dst_a * (1 - src_a)
        let out_a = src_a + (dst_a * inv_src_a) / 255;
        
        if out_a == 0 { return Color::TRANSPARENT; }
        
        // 结果颜色计算需要除以 out_a
        // r = (src.r * src_a + dst.r * dst_a * inv_src_a) / out_a
        let dst_factor = (dst_a * inv_src_a) / 255;
        
        Color {
            r: ((self.r as u32 * src_a + dst.r as u32 * dst_factor) / out_a) as u8,
            g: ((self.g as u32 * src_a + dst.g as u32 * dst_factor) / out_a) as u8,
            b: ((self.b as u32 * src_a + dst.b as u32 * dst_factor) / out_a) as u8,
            a: out_a as u8,
        }
    }

    // 预定义颜色
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const TRANSPARENT: Color = Color::new(0, 0, 0, 0);
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}
