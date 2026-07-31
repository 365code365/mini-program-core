//! 渐变：`createLinearGradient` / `createRadialGradient` 的取色实现
//!
//! `canvas` 模块的一片。**纯搬迁**，一行逻辑没改。
use super::*;

/// 线性渐变
#[derive(Clone)]
pub struct LinearGradient {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    stops: Vec<(f32, Color)>,
}

impl LinearGradient {
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { x0, y0, x1, y1, stops: Vec::new() }
    }
    
    /// 添加颜色停止点
    pub fn add_color_stop(&mut self, offset: f32, color: &str) {
        if let Some(c) = parse_color_str(color) {
            self.stops.push((offset.clamp(0.0, 1.0), c));
            self.stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        }
    }
    
    /// 获取指定位置的颜色
    pub fn get_color_at(&self, x: f32, y: f32) -> Color {
        if self.stops.is_empty() {
            return Color::TRANSPARENT;
        }
        if self.stops.len() == 1 {
            return self.stops[0].1;
        }
        
        // 计算点在渐变线上的投影位置
        let dx = self.x1 - self.x0;
        let dy = self.y1 - self.y0;
        let len_sq = dx * dx + dy * dy;
        if len_sq == 0.0 {
            return self.stops[0].1;
        }
        
        let t = ((x - self.x0) * dx + (y - self.y0) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);
        
        self.interpolate_color(t)
    }
    
    fn interpolate_color(&self, t: f32) -> Color {
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        if t >= self.stops.last().unwrap().0 {
            return self.stops.last().unwrap().1;
        }
        
        for i in 0..self.stops.len() - 1 {
            let (t0, c0) = &self.stops[i];
            let (t1, c1) = &self.stops[i + 1];
            if t >= *t0 && t <= *t1 {
                let ratio = (t - t0) / (t1 - t0);
                return Color::new(
                    (c0.r as f32 + (c1.r as f32 - c0.r as f32) * ratio) as u8,
                    (c0.g as f32 + (c1.g as f32 - c0.g as f32) * ratio) as u8,
                    (c0.b as f32 + (c1.b as f32 - c0.b as f32) * ratio) as u8,
                    (c0.a as f32 + (c1.a as f32 - c0.a as f32) * ratio) as u8,
                );
            }
        }
        self.stops[0].1
    }
}


/// 径向渐变
#[derive(Clone)]
pub struct RadialGradient {
    /// 内圆圆心。**目前不参与求值**：实现按「同心圆」处理，只用外圆的圆心与半径
    /// （`createRadialGradient` 的实际用法几乎都是同心的）。留着两个字段是为了
    /// 接口与 canvas 规范一致，等真要支持偏心渐变时不用改调用方。
    #[allow(dead_code)]
    x0: f32,
    #[allow(dead_code)]
    y0: f32,
    r0: f32,
    x1: f32,
    y1: f32,
    r1: f32,
    stops: Vec<(f32, Color)>,
}

impl RadialGradient {
    pub fn new(x0: f32, y0: f32, r0: f32, x1: f32, y1: f32, r1: f32) -> Self {
        Self { x0, y0, r0, x1, y1, r1, stops: Vec::new() }
    }
    
    /// 添加颜色停止点
    pub fn add_color_stop(&mut self, offset: f32, color: &str) {
        if let Some(c) = parse_color_str(color) {
            self.stops.push((offset.clamp(0.0, 1.0), c));
            self.stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        }
    }
    
    /// 获取指定位置的颜色
    pub fn get_color_at(&self, x: f32, y: f32) -> Color {
        if self.stops.is_empty() {
            return Color::TRANSPARENT;
        }
        
        // 简化实现：使用到中心点的距离
        let dx = x - self.x1;
        let dy = y - self.y1;
        let dist = (dx * dx + dy * dy).sqrt();
        let t = if self.r1 > self.r0 {
            ((dist - self.r0) / (self.r1 - self.r0)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        
        self.interpolate_color(t)
    }
    
    fn interpolate_color(&self, t: f32) -> Color {
        if self.stops.is_empty() {
            return Color::TRANSPARENT;
        }
        if self.stops.len() == 1 {
            return self.stops[0].1;
        }
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        if t >= self.stops.last().unwrap().0 {
            return self.stops.last().unwrap().1;
        }
        
        for i in 0..self.stops.len() - 1 {
            let (t0, c0) = &self.stops[i];
            let (t1, c1) = &self.stops[i + 1];
            if t >= *t0 && t <= *t1 {
                let ratio = (t - t0) / (t1 - t0);
                return Color::new(
                    (c0.r as f32 + (c1.r as f32 - c0.r as f32) * ratio) as u8,
                    (c0.g as f32 + (c1.g as f32 - c0.g as f32) * ratio) as u8,
                    (c0.b as f32 + (c1.b as f32 - c0.b as f32) * ratio) as u8,
                    (c0.a as f32 + (c1.a as f32 - c0.a as f32) * ratio) as u8,
                );
            }
        }
        self.stops[0].1
    }
}
