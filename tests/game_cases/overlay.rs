//! GT-U1（系统覆盖层）：Toast / Loading / Modal / ActionSheet 盖在游戏上、拦触摸、按钮回调值。夹具 sample-game/g-overlays。
use super::*;

const GREEN: [u8; 3] = [0x20, 0x80, 0x40];

fn px(h: &mut GameHarness, x: u32, y: u32) -> [u8; 3] {
    let (w, _, p) = h.engine.frame_rgba();
    let i = ((y * w + x) * 4) as usize;
    [p[i], p[i + 1], p[i + 2]]
}

/// 输出帧里接近白色（≥ 0xF0）的像素包围盒（输出像素）
fn white_bbox(h: &mut GameHarness, y_from: u32) -> Option<(u32, u32, u32, u32)> {
    let (w, hh, p) = h.engine.frame_rgba();
    let mut b: Option<(u32, u32, u32, u32)> = None;
    for y in y_from..hh {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            if p[i] >= 0xF0 && p[i + 1] >= 0xF0 && p[i + 2] >= 0xF0 {
                b = Some(match b { None => (x, y, x, y), Some((a, c, d, e)) => (a.min(x), c.min(y), d.max(x), e.max(y)) });
            }
        }
    }
    b
}

fn tap(h: &mut GameHarness, x: f32, y: f32) {
    let t = h.now_us();
    h.engine.touch(0, 7, x, y, 1.0, t);
    h.engine.touch(2, 7, x, y, 0.0, t);
    h.vsyncs(2);
}

fn ui(h: &GameHarness, tag: &str) -> Vec<serde_json::Value> {
    h.events("ui").into_iter().filter(|e| e["tag"] == tag).collect()
}

#[test]
fn u1_覆盖层盖在游戏上_拦触摸_回调值() {
    let _g = serial();
    let mut h = GameHarness::open("g-overlays", Opts::default());
    h.vsyncs(2);
    assert_eq!(h.engine.frame_rgba().0, 375, "没有覆盖层：输出就是主画布（逻辑尺寸）");
    assert_eq!(px(&mut h, 187, 333), GREEN);
    let plain = h.engine.frame_rgba().2.to_vec();

    // Toast：盖在中间、不挡触摸；到时收起，回到原样
    h.engine.eval("go('toast')").unwrap();
    h.vsyncs(2);
    assert_eq!(h.engine.frame_rgba().0, 750, "覆盖层弹着：按 min(dpr, 2) 合成");
    if std::env::var("U1_DUMP").is_ok() {
        let (w, hh, p) = h.engine.frame_rgba();
        image::RgbaImage::from_raw(w, hh, p.to_vec()).unwrap().save("/tmp/u1_toast.png").unwrap();
    }
    assert_ne!(px(&mut h, 375, 500), GREEN, "提示框（偏上，与微信一致）");
    assert_eq!(px(&mut h, 10, 10), GREEN, "提示框不带遮罩：四周还是游戏");
    tap(&mut h, 50.0, 50.0);
    assert_eq!(h.events("gameTouch").len(), 1, "不带 mask 的 Toast 不挡触摸");
    h.vsyncs(70);
    assert_eq!(h.engine.frame_rgba().2, &plain[..], "收起之后与没弹过逐字节相同");

    // Loading（mask）：挡触摸；hideLoading 之后放行
    h.engine.eval("go('loading')").unwrap();
    h.vsyncs(2);
    tap(&mut h, 50.0, 50.0);
    assert_eq!(h.events("gameTouch").len(), 1, "带 mask 的 Loading 挡触摸");
    h.engine.eval("go('hideLoading')").unwrap();
    h.vsyncs(2);
    tap(&mut h, 50.0, 50.0);
    assert_eq!(h.events("gameTouch").len(), 2);

    // Modal：遮罩压暗、点遮罩不回调也不进游戏；点「确定」回 confirm:true
    h.engine.eval("go('modal')").unwrap();
    h.vsyncs(2);
    let g = px(&mut h, 10, 10);
    assert!(g[1] < GREEN[1], "遮罩压暗了游戏：{g:?}");
    let (x0, y0, x1, y1) = white_bbox(&mut h, 0).expect("弹窗白底");
    tap(&mut h, 20.0, 20.0);
    assert!(ui(&h, "modal").is_empty() && h.events("gameTouch").len() == 2, "点遮罩：不回调、不进游戏");
    // 输出是 2 倍：确定按钮在白框右下角
    tap(&mut h, (x1 as f32 - 30.0) / 2.0, (y1 as f32 - 20.0) / 2.0);
    let m = ui(&h, "modal");
    assert_eq!(m.len(), 1, "{:?}", h.events("ui"));
    assert_eq!((m[0]["r"]["confirm"].as_bool(), m[0]["r"]["cancel"].as_bool()), (Some(true), Some(false)), "{m:?}");
    assert!(x1 - x0 > 400 && y1 > y0, "弹窗大小合理：{:?}", (x0, y0, x1, y1));
    h.vsyncs(2);
    assert_eq!(h.engine.frame_rgba().0, 375, "弹窗关了");

    // ActionSheet：滑入走完之后点第二项 → tapIndex 1；点遮罩 → fail cancel
    h.engine.eval("go('sheet')").unwrap();
    h.vsyncs(40);
    let (_, top, _, _) = white_bbox(&mut h, 667).expect("菜单白底（下半屏）");
    // 菜单从上往下：每项 56 逻辑像素
    tap(&mut h, 187.0, top as f32 / 2.0 + 56.0 + 28.0);
    h.vsyncs(40);
    let s = ui(&h, "sheet");
    assert_eq!((s.len(), s[0]["r"]["tapIndex"].as_u64()), (1, Some(1)), "{s:?}");
    h.engine.eval("go('sheet')").unwrap();
    h.vsyncs(40);
    tap(&mut h, 187.0, 100.0);
    h.vsyncs(40);
    let s = ui(&h, "sheet");
    assert_eq!((s.len(), s[1]["ok"].as_bool()), (2, Some(false)), "点遮罩 = 取消：{s:?}");
    assert!(s[1]["r"]["errMsg"].as_str().unwrap().contains("cancel"));
    assert_eq!(h.events("gameTouch").len(), 2, "菜单弹着时的触摸一个都没进游戏");
    assert_eq!(h.engine.frame_rgba().2, &plain[..], "都收起之后与没弹过逐字节相同");
}
