use super::*;

/// GPU 直通帧先按 `reach` 只合成那一截：入场 / 退场任何时刻画出来的都不能超出它，
/// 而且分窗画出来的那一截要与整帧画的逐字节相同（遮罩只压了窗口里的行）
#[test]
fn 面板总在_reach_之内_分窗与整帧逐字节相同() {
    let (w, h, sf) = (90u32, 500u32, 1.5f32);
    let base: Vec<u32> = (0..w * h).map(|i| i.wrapping_mul(2_654_435_761) & 0xFF_FFFF).collect();
    for (ago, closing) in [(0, false), (40, false), (120, false), (500, false), (60, true), (250, true)] {
        open_at(ago, closing);
        let (rows, base_rows) = reach(w, h, sf, None);
        let at = FROZEN.with(|f| f.get()).expect("reach 定住了这一刻");
        let win = rows.rows.expect("弹着就有范围");
        let b = base_rows.rows.unwrap_or((win.0, win.0));
        assert!(b.0 == win.0 && b.1 <= win.1, "{ago}ms：底图行 {b:?} 应在范围 {win:?} 的开头");
        if win.1 - win.0 > 100 {
            assert!(b.1 - b.0 < 40, "{ago}ms：白底整行盖住的那些行不该要底图 {b:?} / {win:?}");
        }
        // 分窗：只给顶边那几行合成底图、压暗，其余行靠白底整行盖掉；未合成的行先填一个怪色
        let mut part: Vec<u32> = base.iter().enumerate().map(|(i, &p)| if (i as u32 / w) >= b.0 && (i as u32 / w) < b.1 { p } else { 0x12_3456 }).collect();
        let fp = render(&mut part, w, h, sf, None, b);
        assert_eq!(fp.rows, Some(win), "{ago}ms：同一刻画出来的范围就是 reach 给的");
        // 整帧（同一刻）
        FROZEN.with(|f| f.set(Some(at)));
        let mut full = base.clone();
        render(&mut full, w, h, sf, None, (0, h));
        let (s, e) = ((win.0 * w) as usize, (win.1 * w) as usize);
        assert!(part[s..e] == full[s..e], "{ago}ms：分窗与整帧在覆盖层范围里不一致");
    }
    clear();
}

/// 面板缓存的编号进程里唯一：GPU 自检在预热线程上画过一张，合成器搬到引擎线程后，真弹出来的那张
/// 不能与它撞号（撞了 GPU 就不传纹理，屏上贴的是自检那张：白板没字、底下一截黑）
#[test]
fn 面板缓存编号跨线程不撞() {
    let built = || {
        open_at(900, false);
        reach(90, 500, 1.5, None);
        let id = gpu_layer().map(|l| l.3);
        clear();
        id.expect("弹着就有面板缓存")
    };
    let other = std::thread::spawn(built).join().expect("另一条线程");
    let here = built();
    assert_ne!(other, here, "两条线程上的面板缓存撞号");
}

/// 面板缓存贴出来的与直接画的逐字节相同（画与贴在同一个位置时：静止的面板、gallery）。
/// 按着第一项（没有 alertText）时按压底盖过圆角 —— 缓存要认出那些像素是内容，不能当圆角现混。
///
/// 各例之间不清缓存：同一尺寸的下一张接着画在上一张的画布上（有 / 没有 alertText，面板顶边不在同一行），
/// 上一张留下的像素不能漏进来
#[test]
fn 缓存贴出来与直接画逐字节相同() {
    RASTER.with(|r| *r.borrow_mut() = None);
    let cases = [(3.0f32, "提示", Some(1)), (3.0, "", Some(-1)), (3.0, "提示", None), (2.0, "", Some(0)), (2.55, "", None)];
    for (i, (dpr, alert, pressed)) in cases.into_iter().enumerate() {
        let (w, h) = (360u32 * dpr as u32, 700u32 * dpr as u32);
        let base: Vec<u32> = (0..w * h).map(|i| i.wrapping_mul(2_654_435_761) & 0xFF_FFFF).collect();
        let t = Instant::now().checked_sub(std::time::Duration::from_millis(900)).unwrap();
        let st = ActionSheet {
            id: 7 + i as u64,
            alert: alert.into(),
            items: vec!["拍照".into(), "从相册选择".into()],
            color: Color::BLACK,
            opened: t,
            closing: None,
            pressed,
            armed: true,
            settled: false,
        };
        with(|s| *s = Some(st.clone()));
        let mut got = base.clone();
        render(&mut got, w, h, dpr, None, (0, h));
        // 直接画（从前的画法）：同一个遮罩表、同一个顶边
        let mut want = base.clone();
        Footprint::apply_lut(&mut want, w, (0, h), &footprint::lut_trunc(1.0 - 0.6), 0);
        let top = h as f32 / dpr - sheet_height(&st);
        draw_panel(&mut want, w, h, dpr, None, &st, top);
        let bad = got.iter().zip(&want).position(|(a, b)| a != b);
        assert!(bad.is_none(), "dpr {dpr} alert {alert:?} pressed {pressed:?}：第一个不同 @{:?}", bad.map(|i| (i as u32 % w, i as u32 / w)));
    }
    clear();
}
