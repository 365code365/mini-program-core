//! Scroll-view 离屏缓存
//! 
//! 实现类似微信小程序的离屏渲染优化：
//! - 将 scroll-view 的完整内容渲染到离屏 Canvas
//! - 滚动时直接从缓存复制可见区域，避免重新渲染
//! - 只有内容变化时才更新缓存

use crate::Canvas;
use std::collections::HashMap;

/// 单个 scroll-view 的缓存
pub struct ScrollViewCache {
    /// 离屏 Canvas（存储完整内容）
    pub canvas: Canvas,
    /// 内容宽度
    pub content_width: u32,
    /// 内容高度
    pub content_height: u32,
    /// 视口宽度
    pub viewport_width: u32,
    /// 视口高度
    pub viewport_height: u32,
    /// 内容版本号（用于判断是否需要更新）
    pub version: u64,
    /// 是否需要重新渲染
    dirty: bool,
}

impl ScrollViewCache {
    /// 创建新的缓存
    pub fn new(content_width: u32, content_height: u32, viewport_width: u32, viewport_height: u32) -> Self {
        Self {
            canvas: Canvas::new(content_width.max(1), content_height.max(1)),
            content_width,
            content_height,
            viewport_width,
            viewport_height,
            version: 0,
            dirty: true,
        }
    }
    
    /// 检查并更新缓存尺寸
    pub fn update_size(&mut self, content_width: u32, content_height: u32, viewport_width: u32, viewport_height: u32) -> bool {
        let size_changed = self.content_width != content_width || self.content_height != content_height;
        
        if size_changed {
            self.content_width = content_width;
            self.content_height = content_height;
            self.canvas = Canvas::new(content_width.max(1), content_height.max(1));
            self.dirty = true;
        }
        
        self.viewport_width = viewport_width;
        self.viewport_height = viewport_height;
        
        size_changed
    }
    
    /// 标记为脏（需要重新渲染）
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    
    /// 标记为干净（已渲染）
    pub fn mark_clean(&mut self) {
        self.dirty = false;
        self.version += 1;
    }
    
    /// 是否需要重新渲染
    pub fn needs_render(&self) -> bool {
        self.dirty
    }
    
    /// 获取可变 Canvas 引用（用于渲染）
    pub fn canvas_mut(&mut self) -> &mut Canvas {
        &mut self.canvas
    }
    
    /// 将缓存的可见区域复制到目标 Canvas
    /// 
    /// - target: 目标 Canvas
    /// - scroll_offset: 滚动偏移（逻辑像素）
    /// - dest_x, dest_y: 目标位置（物理像素）
    /// - scale_factor: 缩放因子
    /// 按**行**成片拷贝，不逐像素。
    ///
    /// 一屏是 750x1334 ≈ 一百万像素，逐像素 `set_pixel` 要为每个像素重做边界比较
    /// 与裁剪矩形的浮点截断；Skyline 那边滑动是纯合成操作，我们这一笔白花的开销
    /// 直接表现为「拖动时掉帧、发抖」。`Canvas::blend_pixels` 每行只判一次边界，
    /// 整行不透明时退化成 memcpy。
    pub fn blit_to(
        &self,
        target: &mut Canvas,
        scroll_offset: f32,
        dest_x: f32,
        dest_y: f32,
        scale_factor: f32,
    ) {
        let src_pixels = self.canvas.pixels();
        let src_width = self.canvas.width() as usize;
        let src_height = self.canvas.height();
        if src_width == 0 || src_height == 0 {
            return;
        }

        // 源区域（物理像素）
        let src_y_start = (scroll_offset * scale_factor).max(0.0) as u32;
        if src_y_start >= src_height {
            return;
        }
        let visible_height = (self.viewport_height as f32 * scale_factor) as u32;
        let src_y_end = (src_y_start + visible_height).min(src_height);
        if src_y_end <= src_y_start {
            return;
        }

        let from = src_y_start as usize * src_width;
        let to = src_y_end as usize * src_width;
        target.blend_pixels(dest_x as i32, dest_y as i32, &src_pixels[from..to], src_width);
    }
}

/// Scroll-view 缓存管理器
pub struct ScrollCacheManager {
    /// 缓存映射：scroll-view ID -> 缓存
    caches: HashMap<String, ScrollViewCache>,
    /// 全局版本号（页面数据变化时递增）
    global_version: u64,
}

impl ScrollCacheManager {
    pub fn new() -> Self {
        Self {
            caches: HashMap::new(),
            global_version: 0,
        }
    }
    
    /// 获取或创建缓存
    pub fn get_or_create(
        &mut self,
        id: &str,
        content_width: u32,
        content_height: u32,
        viewport_width: u32,
        viewport_height: u32,
    ) -> &mut ScrollViewCache {
        if !self.caches.contains_key(id) {
            let cache = ScrollViewCache::new(content_width, content_height, viewport_width, viewport_height);
            self.caches.insert(id.to_string(), cache);
        } else {
            // 更新尺寸
            if let Some(cache) = self.caches.get_mut(id) {
                cache.update_size(content_width, content_height, viewport_width, viewport_height);
            }
        }
        
        self.caches.get_mut(id).unwrap()
    }
    
    /// 获取缓存（只读）
    pub fn get(&self, id: &str) -> Option<&ScrollViewCache> {
        self.caches.get(id)
    }
    
    /// 获取缓存（可变）
    pub fn get_mut(&mut self, id: &str) -> Option<&mut ScrollViewCache> {
        self.caches.get_mut(id)
    }
    
    /// 标记所有缓存为脏
    pub fn mark_all_dirty(&mut self) {
        self.global_version += 1;
        for cache in self.caches.values_mut() {
            cache.mark_dirty();
        }
    }
    
    /// 清除所有缓存
    pub fn clear(&mut self) {
        self.caches.clear();
        self.global_version = 0;
    }
    
    /// 移除指定缓存
    pub fn remove(&mut self, id: &str) {
        self.caches.remove(id);
    }
    
    /// 获取全局版本号
    pub fn global_version(&self) -> u64 {
        self.global_version
    }
}

impl Default for ScrollCacheManager {
    fn default() -> Self {
        Self::new()
    }
}
