//! 多个输入框之间换焦点：旧框的光标要消失、占位符要回来；聚焦但还没输入时占位符照常显示（与微信一致）。
//!
//! 背景：兴趣圈发帖页（sample/forum `pages/compose/compose`）先点标题框再点正文框，标题框里还留着一个光标
//! （局部重画只重画了新框）；两个框聚焦时都不画占位符。判据：
//! ① 点完正文框之后，标题框那块与「只点过正文框」的引擎逐像素相同（不留光标、占位符在）；
//! ② 开 / 关局部重画两个引擎同一串点击，标题框那块逐像素相同；
//! ③ 正文框聚焦、值为空：盒子里有占位符的灰字，光标（#07C160）在第一行开头。
use mini_render::compositor::HostEnv;
use mini_render::host::{EngineOptions, MiniEngine};
use std::path::PathBuf;
use std::time::Duration;

const W: u32 = 375;
const DPR: f32 = 2.0;
// 用 SelectorQuery 量出来的（375 宽、dpr 2）：标题框 (26,14,296,50)，正文框 (26,74,323,160)
const TITLE: (f32, f32, f32, f32) = (26.0, 14.0, 296.0, 50.0);
const BODY: (f32, f32, f32, f32) = (26.0, 74.0, 323.0, 160.0);

fn engine(no_damage: bool, tag: &str) -> MiniEngine {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample/forum");
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("focus-visual-{tag}"));
    std::fs::remove_dir_all(&data).ok();
    let env = HostEnv { no_damage, ..HostEnv::default() };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), W, 667, DPR, env, EngineOptions::default())
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(Some("pages/compose/compose")).expect("打开发帖页");
    e
}

fn pump(e: &mut MiniEngine, t: &mut u64, n: u32) {
    for _ in 0..n {
        *t += 16;
        std::thread::sleep(Duration::from_millis(4));
        e.pump(*t);
    }
}

fn tap(e: &mut MiniEngine, t: &mut u64, (x, y): (f32, f32)) {
    e.pointer_down(x, y);
    pump(e, t, 1);
    e.pointer_up(x, y);
    pump(e, t, 3);
}

fn region(e: &mut MiniEngine, (x, y, w, h): (f32, f32, f32, f32)) -> Vec<[u8; 3]> {
    let px = e.rgba().to_vec();
    let pw = e.pixel_size().0 as usize;
    let mut out = Vec::new();
    for yy in (y * DPR) as usize..((y + h) * DPR) as usize {
        for xx in (x * DPR) as usize..((x + w) * DPR) as usize {
            let i = (yy * pw + xx) * 4;
            out.push([px[i], px[i + 1], px[i + 2]]);
        }
    }
    out
}

fn center((x, y, w, h): (f32, f32, f32, f32)) -> (f32, f32) {
    (x + w / 2.0, y + h / 2.0)
}

fn is_caret(p: &[u8; 3]) -> bool {
    p[0] < 40 && p[1] > 160 && p[2] < 120
}

#[test]
fn 换焦点之后旧框不留光标_占位符回来_聚焦空框照样显示占位符() {
    // 先点标题再点正文
    let (mut a, mut ta) = (engine(false, "a"), 1000u64);
    pump(&mut a, &mut ta, 5);
    tap(&mut a, &mut ta, center(TITLE));
    pump(&mut a, &mut ta, 3);
    tap(&mut a, &mut ta, (BODY.0 + 40.0, BODY.1 + 20.0));
    pump(&mut a, &mut ta, 3);
    // 只点正文
    let (mut c, mut tc) = (engine(false, "c"), 1000u64);
    pump(&mut c, &mut tc, 5);
    tap(&mut c, &mut tc, (BODY.0 + 40.0, BODY.1 + 20.0));
    pump(&mut c, &mut tc, 3);
    let (ra, rc) = (region(&mut a, TITLE), region(&mut c, TITLE));
    assert!(!ra.iter().any(is_caret), "标题框失焦后还留着光标");
    if ra != rc {
        let tw = (TITLE.2 * DPR) as usize;
        let d: Vec<usize> = (0..ra.len()).filter(|&i| ra[i] != rc[i]).collect();
        let (xs, ys): (Vec<usize>, Vec<usize>) = d.iter().map(|i| (i % tw, i / tw)).unzip();
        panic!(
            "标题框失焦后与没点过它的不一样（光标 / 占位符残留）：{} 个像素，x {}..{} y {}..{}；样本 {:?} vs {:?}",
            d.len(), xs.iter().min().unwrap(), xs.iter().max().unwrap(), ys.iter().min().unwrap(), ys.iter().max().unwrap(), ra[d[0]], rc[d[0]]
        );
    }
    // 关掉局部重画的参照引擎：同一串点击，标题框那块相同
    let (mut b, mut tb) = (engine(true, "b"), 1000u64);
    pump(&mut b, &mut tb, 5);
    tap(&mut b, &mut tb, center(TITLE));
    pump(&mut b, &mut tb, 3);
    tap(&mut b, &mut tb, (BODY.0 + 40.0, BODY.1 + 20.0));
    pump(&mut b, &mut tb, 3);
    assert!(region(&mut b, TITLE) == ra, "局部重画与整帧重画的标题框不一样");
    // 正文框聚焦、值为空：占位符的灰字在（不是纯白），光标在左上
    let body = region(&mut a, BODY);
    // 占位符是 `.ph { color: #c0c4cc }`：浅灰偏蓝，抗锯齿后在 (150..240) 之间、蓝分量最大
    let grey = body.iter().filter(|p| p[0] > 150 && p[0] < 240 && p[2] >= p[0] && !is_caret(p)).count();
    assert!(grey > 200, "聚焦的空正文框没画占位符（灰色像素 {grey}）");
    let bw = (BODY.2 * DPR) as usize;
    let caret_cols: Vec<usize> = body.iter().enumerate().filter(|(_, p)| is_caret(p)).map(|(i, _)| i % bw).collect();
    // 光标按 500ms 闪：这一拍可能是灭的，亮的话必须在开头
    if let Some(&mx) = caret_cols.iter().max() {
        assert!(mx < 40, "光标该在第一行开头（占位符前面），实际在第 {mx} 列");
    }
}

/// 不经过点击的失焦（`wx.hideKeyboard`、页面把 `focus` 设回 false）：光标同样要消失。
/// 焦点无论从哪条路换掉，旧框都要重画（engine_press.rs `sync_focus_visual` 每帧比对）
#[test]
fn 程序失焦_光标消失_占位符回来() {
    let (mut a, mut ta) = (engine(false, "h-a"), 1000u64);
    pump(&mut a, &mut ta, 5);
    let before = region(&mut a, TITLE);
    tap(&mut a, &mut ta, center(TITLE));
    // 等到光标亮的那一拍（500ms 闪一次）
    let mut lit = false;
    for _ in 0..80 {
        pump(&mut a, &mut ta, 1);
        if region(&mut a, TITLE).iter().any(is_caret) {
            lit = true;
            break;
        }
    }
    assert!(lit, "聚焦后标题框里一直没出现光标");
    a.eval("wx.hideKeyboard()").unwrap();
    pump(&mut a, &mut ta, 4);
    let after = region(&mut a, TITLE);
    assert!(!after.iter().any(is_caret), "wx.hideKeyboard 之后标题框还留着光标");
    assert!(after == before, "失焦之后标题框与聚焦前不一样");
}
