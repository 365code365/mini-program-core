//! Skyline swiper 的 `layout-type` 与 `transformer-type`：按项相对当前位置的偏移 `d`
//! （单位：项；负数 = 已经划过去的、正数 = 后面的）给每一项一个变换与不透明度。
//!
//! 变换交给已有的 CSS transform 合成路径（项的 `style.transform` / `opacity`，以项中心为原点），
//! 平移量是**在原本排布位置之上**再加的（物理像素）。没写 `layout-type`（或写 `normal`）时不走这里，
//! 与从前的绘制一字不差。
//!
//! 近似（没有 3D 合成）：`threeD` 用横向（纵向 swiper 是纵向）缩放模拟绕轴翻转，`accordion`
//! 同样按主轴压扁；`tinder` 的卡片随拖动旋转、后面的卡片按层级缩小露出。

use super::super::base::{RenderNode, Transform};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    StackLeft,
    StackRight,
    Tinder,
    ScaleAndFade,
    Accordion,
    ThreeD,
    ZoomIn,
    ZoomOut,
    DeepthPage,
}

pub fn kind(node: &RenderNode) -> Option<Kind> {
    let lt = node.attrs.get("layout-type").map(|s| s.trim()).unwrap_or("");
    Some(match lt {
        "stackLeft" => Kind::StackLeft,
        "stackRight" => Kind::StackRight,
        "tinder" => Kind::Tinder,
        "transformer" => match node.attrs.get("transformer-type").map(|s| s.trim()).unwrap_or("scaleAndFade") {
            "accordion" => Kind::Accordion,
            "threeD" => Kind::ThreeD,
            "zoomIn" => Kind::ZoomIn,
            "zoomOut" => Kind::ZoomOut,
            "deepthPage" | "depthPage" => Kind::DeepthPage,
            _ => Kind::ScaleAndFade,
        },
        _ => return None,
    })
}

/// 要画的项：(真实下标, 虚拟下标)。堆叠类要把后面几张也画出来（它们不在原本的可见范围里）
pub fn items(pos: f32, total: usize, circular: bool) -> Vec<(usize, i64)> {
    if total == 0 {
        return Vec::new();
    }
    let base = pos.floor() as i64;
    (base - 1..=base + 3)
        .filter_map(|v| {
            if circular {
                Some((v.rem_euclid(total as i64) as usize, v))
            } else if v >= 0 && (v as usize) < total {
                Some((v as usize, v))
            } else {
                None
            }
        })
        .collect()
}

/// 一项的效果
#[derive(Clone, Copy, Debug)]
pub struct Fx {
    pub transform: Transform,
    pub opacity: f32,
    /// 画的先后（小的先画、在下面）
    pub z: i32,
}

/// `d` = 虚拟下标 − 当前位置；`item` = 一项沿主轴的物理像素。None：这一项不画
pub fn fx(kind: Kind, d: f32, item: f32, vertical: bool) -> Option<Fx> {
    let mut t = Transform::new();
    let mut opacity = 1.0;
    let ad = d.abs();
    // 主轴上的附加平移
    let mut shift = 0.0;
    let mut z = -(ad * 100.0) as i32;
    match kind {
        Kind::StackLeft | Kind::StackRight | Kind::Tinder => {
            if d > 3.0 || d < -1.0 {
                return None;
            }
            if d > 0.0 {
                // 后面的卡片原地叠着（抵消原本的排布位移），按层级缩小、错开一点
                let dir = if kind == Kind::StackRight { -1.0 } else { 1.0 };
                shift = -d * item + dir * d * item * 0.06;
                let s = 1.0 - 0.06 * d;
                t.scale_x = s;
                t.scale_y = s;
                opacity = (1.0 - 0.2 * (d - 1.0).max(0.0)).max(0.0);
            } else if kind == Kind::Tinder {
                // 当前卡片被拖走：跟手平移并旋转
                t.rotate = d * 15.0;
                opacity = (1.0 + d).max(0.0);
            }
            z = -(d * 100.0) as i32;
        }
        Kind::ScaleAndFade => {
            let s = (1.0 - 0.2 * ad).max(0.8);
            t.scale_x = s;
            t.scale_y = s;
            opacity = (1.0 - 0.5 * ad).max(0.0);
        }
        Kind::Accordion | Kind::ThreeD => {
            let s = if kind == Kind::ThreeD { (ad.min(1.0) * std::f32::consts::FRAC_PI_2).cos() } else { (1.0 - ad).max(0.0) };
            if vertical { t.scale_y = s } else { t.scale_x = s }
            if kind == Kind::ThreeD {
                let k = 1.0 - 0.1 * ad.min(1.0);
                if vertical { t.scale_x = k } else { t.scale_y = k }
            }
            // 压扁时贴着相邻项的那条边，而不是以自己中心收缩
            shift = -d.signum() * item * (1.0 - s) / 2.0;
        }
        Kind::ZoomIn => {
            let s = (1.0 - 0.4 * ad).max(0.6);
            t.scale_x = s;
            t.scale_y = s;
            opacity = (1.0 - 0.6 * ad).max(0.0);
        }
        Kind::ZoomOut => {
            let s = 1.0 + 0.3 * ad;
            t.scale_x = s;
            t.scale_y = s;
            opacity = (1.0 - ad).max(0.0);
        }
        Kind::DeepthPage => {
            if d > 0.0 {
                // 后一页在原地从深处浮上来
                shift = -d * item;
                let s = 0.75 + 0.25 * (1.0 - d.min(1.0));
                t.scale_x = s;
                t.scale_y = s;
                opacity = (1.0 - d).max(0.0);
                z = -100 - (d * 100.0) as i32;
            }
        }
    }
    if opacity <= 0.001 {
        return None;
    }
    if vertical { t.translate_y = shift } else { t.translate_x = shift }
    Some(Fx { transform: t, opacity, z })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_item_is_untouched_and_neighbours_fade() {
        let f = fx(Kind::ScaleAndFade, 0.0, 300.0, false).unwrap();
        assert_eq!((f.transform.scale_x, f.opacity), (1.0, 1.0));
        let n = fx(Kind::ScaleAndFade, 1.0, 300.0, false).unwrap();
        assert!(n.transform.scale_x < 1.0 && n.opacity < 1.0);
        // 堆叠：后一张原地叠在下面（抵消一整项的排布位移），且先画
        let s = fx(Kind::StackLeft, 1.0, 300.0, false).unwrap();
        assert!(s.transform.translate_x < -250.0 && s.z < f.z);
        assert!(fx(Kind::StackLeft, 4.0, 300.0, false).is_none());
    }

    #[test]
    fn stack_draws_cards_behind() {
        assert_eq!(items(1.0, 5, false).iter().map(|x| x.0).collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);
        assert_eq!(items(0.0, 3, true).iter().map(|x| x.0).collect::<Vec<_>>(), vec![2, 0, 1, 2, 0]);
    }
}
