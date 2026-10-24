//! `<image>` 的九种「不缩放、只裁剪」mode：`top` / `bottom` / `center` / `left` / `right` /
//! `top left` / `top right` / `bottom left` / `bottom right`（官方 component/image 的「裁剪模式」）。
//!
//! 原图按 1 图片像素 = 1 逻辑像素画（设备像素再乘 dpr，同 CSS `object-fit: none`），贴到框的那条边 / 那个角，
//! 框外的部分裁掉。从前这九种都落进 scaleToFill，整张拉伸成框的大小。

/// 裁剪模式下原图画到哪（设备像素的目标矩形 `(x, y, w, h)`，可能比框大，画的时候裁到框）。
/// 不是裁剪模式返回 `None`（走原来的缩放路径）
pub fn crop_rect(mode: &str, img_w: u32, img_h: u32, x: f32, y: f32, w: f32, h: f32, sf: f32) -> Option<(f32, f32, f32, f32)> {
    // (水平对齐, 竖直对齐)：0 左 / 上，0.5 居中，1 右 / 下
    let (ax, ay) = match mode {
        "top" => (0.5, 0.0),
        "bottom" => (0.5, 1.0),
        "center" => (0.5, 0.5),
        "left" => (0.0, 0.5),
        "right" => (1.0, 0.5),
        "top left" => (0.0, 0.0),
        "top right" => (1.0, 0.0),
        "bottom left" => (0.0, 1.0),
        "bottom right" => (1.0, 1.0),
        _ => return None,
    };
    if img_w == 0 || img_h == 0 {
        return None;
    }
    let (dw, dh) = (img_w as f32 * sf, img_h as f32 * sf);
    // 取整到设备像素：贴边的那一侧正好对齐框边，不出现半像素的模糊边
    let dx = (x + (w - dw) * ax).round();
    let dy = (y + (h - dh) * ay).round();
    Some((dx, dy, dw, dh))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 贴边与居中() {
        // 100×50 的图、框 60×60（dpr 1）：top 水平居中、上边对齐
        assert_eq!(crop_rect("top", 100, 50, 0.0, 0.0, 60.0, 60.0, 1.0), Some((-20.0, 0.0, 100.0, 50.0)));
        assert_eq!(crop_rect("bottom right", 100, 50, 0.0, 0.0, 60.0, 60.0, 1.0), Some((-40.0, 10.0, 100.0, 50.0)));
        assert_eq!(crop_rect("center", 10, 10, 0.0, 0.0, 30.0, 30.0, 2.0), Some((5.0, 5.0, 20.0, 20.0)));
        assert_eq!(crop_rect("aspectFit", 10, 10, 0.0, 0.0, 30.0, 30.0, 2.0), None);
    }
}
