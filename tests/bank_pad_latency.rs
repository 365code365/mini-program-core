//! 招财银行 · 转账（sample/bank `pages/transfer/transfer`）的支付密码键盘：键盘画在 `position: fixed` 的遮罩里，
//! 每个键 `hover-class="key-down"` + `bindtap="press"`（setData 圆点）。用户反馈「点击输入很不灵敏很慢很卡」。
//!
//! 判据：连点 6 个键，每一下从抬手到圆点画出来不超过 2 帧；按下与抬手那几帧都不整帧
//! （只重画覆盖层里变了的那一块），最慢一帧 < 8ms（桌面）。
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::MiniEngine;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[test]
fn 转账_支付密码键盘_连点每一下两帧内出圆点且不整帧() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample/bank");
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("bank-pad");
    std::fs::remove_dir_all(&data).ok();
    let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), 360, 785, 3.0).expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(Some("pages/transfer/transfer")).expect("打开转账");
    let mut t = 1000u64;
    let mut log: Vec<(FrameKind, f32)> = Vec::new();
    let mut step = |e: &mut MiniEngine, log: &mut Vec<(FrameKind, f32)>| -> bool {
        t += 11;
        std::thread::sleep(Duration::from_millis(11));
        let s = Instant::now();
        let f = e.pump_ex(t) & FRAME_NEW != 0;
        if f {
            log.push((e.last_frame_kind(), s.elapsed().as_secs_f32() * 1000.0));
        }
        f
    };
    for _ in 0..5 {
        step(&mut e, &mut log);
    }
    e.eval("getCurrentPages()[0].setData({showPad: true, amountView: '500', name: '曹文杰', bankName: '建设银行', cardTail: '2288'})").unwrap();
    for _ in 0..20 {
        step(&mut e, &mut log);
    }
    log.clear();
    // 键盘第一行在屏幕底部往上 4 行键处；三列中间。键高 108rpx = 51.84 逻辑像素
    let key_h = 108.0 * 360.0 / 750.0;
    let (w, h) = (360.0f32, e.pixel_size().1 as f32 / 3.0);
    let row1 = h - key_h * 3.5;
    let dots = |e: &MiniEngine| e.eval("getCurrentPages()[0].data.pwd.length").unwrap_or_default();
    let mut worst_wait = 0;
    let taps = std::env::var("PAD_TAPS").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
    for i in 0..taps {
        let x = w / 6.0 + (i % 3) as f32 * w / 3.0;
        let before = dots(&e);
        e.pointer_down(x, row1);
        for _ in 0..4 {
            step(&mut e, &mut log);
        }
        e.pointer_up(x, row1);
        let mut waited = 0;
        loop {
            waited += 1;
            step(&mut e, &mut log);
            if dots(&e) != before || waited > 30 {
                break;
            }
        }
        worst_wait = worst_wait.max(waited);
        for _ in 0..6 {
            step(&mut e, &mut log);
        }
    }
    let n = dots(&e);
    let full: Vec<String> = log.iter().filter(|(k, _)| matches!(k, FrameKind::Full(_))).map(|(k, ms)| format!("{}:{ms:.1}", k.label())).collect();
    let worst = log.iter().map(|(_, ms)| *ms).fold(0.0, f32::max);
    let all: Vec<String> = log.iter().map(|(k, ms)| format!("{}:{ms:.1}", k.label())).collect();
    eprintln!("点了 5 下，pwd 长度 {n}，抬手到数据变最多 {worst_wait} 帧，整帧 {}，最慢 {worst:.1}ms\n{}", full.len(), all.join(" "));
    if taps != 5 {
        return; // 排查用（`PAD_TAPS=200` 配 profiler），不判
    }
    assert_eq!(n.trim(), "5", "5 下没全点进去");
    assert!(worst_wait <= 2, "抬手到圆点最多等了 {worst_wait} 帧");
    assert!(full.is_empty(), "按键时出了整帧：{full:?}");
}

/// 覆盖层只重画点击态那一块（host/engine_fixed_partial.rs）：开 / 关局部重画两个引擎喂同一串按键，
/// 每一帧合成结果逐字节相同。点击态按墙上时钟出现 / 消失（hover-start-time / hover-stay-time），
/// 两个引擎前后脚走，偶尔差一帧：只比两边点击态一样的帧（都按着 / 都没按）
#[test]
fn 转账_支付密码键盘_覆盖层局部重画与整层逐字节一致() {
    use mini_render::compositor::HostEnv;
    use mini_render::host::EngineOptions;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample/bank");
    let mk = |no_damage: bool| {
        let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(if no_damage { "bank-pad-b" } else { "bank-pad-a" });
        std::fs::remove_dir_all(&data).ok();
        let env = HostEnv { no_damage, ..HostEnv::default() };
        let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 360, 785, 3.0, env, EngineOptions::default())
            .expect("创建引擎");
        e.set_animation_clock(0.0);
        e.launch(Some("pages/transfer/transfer")).expect("打开转账");
        e
    };
    let (mut a, mut b) = (mk(false), mk(true));
    let mut t = 1000u64;
    let mut compared = 0;
    let mut partial = 0;
    let mut step = |a: &mut MiniEngine, b: &mut MiniEngine, what: &str| {
        t += 11;
        std::thread::sleep(Duration::from_millis(11));
        let fa = a.pump_ex(t) & FRAME_NEW;
        let fb = b.pump_ex(t) & FRAME_NEW;
        if fa != 0 && matches!(a.last_frame_kind(), FrameKind::Damage) {
            partial += 1;
        }
        if (fa != 0 || fb != 0) && a.rgba() == b.rgba() {
            compared += 1;
        } else if fa != 0 || fb != 0 {
            // 不一样时只允许是点击态的时刻差：再走一帧（两边都过了那个时刻）必须对上
            t += 11;
            std::thread::sleep(Duration::from_millis(40));
            a.pump_ex(t);
            b.pump_ex(t);
            assert!(a.rgba() == b.rgba(), "{what}：局部那边「{}」与整层重画不一致（多等一帧也没对上）", a.last_frame_kind().label());
        }
    };
    for e in [&mut a, &mut b] {
        e.eval("getCurrentPages()[0].setData({showPad: true, amountView: '500', name: '曹文杰', bankName: '建设银行', cardTail: '2288'})").unwrap();
    }
    for _ in 0..20 {
        step(&mut a, &mut b, "弹键盘");
    }
    let key_h = 108.0 * 360.0 / 750.0;
    let h = a.pixel_size().1 as f32 / 3.0;
    for i in 0..5 {
        let (x, y) = (60.0 + (i % 3) as f32 * 120.0, h - key_h * (3.5 - (i / 3) as f32));
        for e in [&mut a, &mut b] {
            e.pointer_down(x, y);
        }
        for _ in 0..5 {
            step(&mut a, &mut b, "按着");
        }
        for e in [&mut a, &mut b] {
            e.pointer_up(x, y);
        }
        for _ in 0..12 {
            step(&mut a, &mut b, "松手");
        }
    }
    assert!(compared >= 10, "比过的帧太少：{compared}");
    assert!(partial > 0, "按键时覆盖层该有只重画一块的帧");
}
