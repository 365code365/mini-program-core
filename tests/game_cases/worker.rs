//! GT-W（Worker，game-doc/05 §3）：夹具 sample-game/g-worker。Worker 是真线程，带墙上时钟的只做日志 / 分布断言（06 §45）。
use super::*;
use mini_render::game::GameEngine;
use std::time::{Duration, Instant};

fn w_ev(h: &GameHarness, cmd: &str) -> Vec<serde_json::Value> {
    h.events("w").into_iter().filter(|e| e["cmd"] == cmd).collect()
}

/// 跑到出现 `cmd` 的回复为止（墙上时钟上限 `secs`），返回期间最慢一次 pump 的耗时
fn wait(h: &mut GameHarness, cmd: &str, secs: f64) -> Duration {
    let t0 = Instant::now();
    let mut worst = Duration::ZERO;
    while w_ev(h, cmd).is_empty() {
        assert!(t0.elapsed().as_secs_f64() < secs, "{secs}s 内没等到 Worker 的 {cmd}：{:?}", h.events("w"));
        let p = Instant::now();
        h.vsyncs(1);
        worst = worst.max(p.elapsed());
        std::thread::sleep(Duration::from_millis(2));
    }
    worst
}

/// W1 / W5 / 错误：Worker 里 1.5s 死算期间主线程照常出帧（每次 pump 都很快）；Worker 环境里没有 wx；
/// Worker 里 require 按 Worker 目录；定时器能用；没接住的异常 → onError；Worker 的 console 进宿主日志
#[test]
fn w1_w5_真线程与环境() {
    let _g = serial();
    let base = GameEngine::live_worker_threads();
    let mut h = GameHarness::open("g-worker", Opts::default());
    h.engine.eval("go('badPath')").unwrap();
    h.vsyncs(1);
    assert!(h.events("w")[0]["msg"].as_str().unwrap().contains("workers"), "不在 workers 目录：{:?}", h.events("w"));
    h.engine.eval("go('create')").unwrap();
    wait(&mut h, "ready", 5.0);
    assert_eq!(GameEngine::live_worker_threads(), base + 1);
    h.engine.eval("go('env')").unwrap();
    wait(&mut h, "env", 5.0);
    let env = &w_ev(&h, "env")[0];
    assert_eq!((env["wx"].as_str(), env["worker"].as_str(), env["setTimeout"].as_str(), env["util"].as_str()),
        (Some("undefined"), Some("object"), Some("function"), Some("util-in-worker")), "{env}");
    h.engine.eval("go('timer')").unwrap();
    wait(&mut h, "timer", 5.0);
    h.engine.eval("go('log')").unwrap();
    let t0 = Instant::now();
    while !h.has("T:EV workerLog") {
        assert!(t0.elapsed().as_secs() < 5, "Worker 的 console 没进日志");
        h.vsyncs(1);
    }
    let f0: f64 = h.engine.eval("frames()").unwrap().parse().unwrap();
    h.engine.eval("go('busy', 1500)").unwrap();
    let t_busy = Instant::now();
    let worst = wait(&mut h, "busyDone", 10.0);
    let f1: f64 = h.engine.eval("frames()").unwrap().parse().unwrap();
    eprintln!("W1：Worker 死算 {:?}，期间 rAF {} 帧，最慢一次 pump {worst:?}", t_busy.elapsed(), f1 - f0);
    assert!(t_busy.elapsed().as_millis() >= 1400, "Worker 真的算了 1.5s");
    assert!(worst < Duration::from_millis(50), "Worker 死算时主线程一次 pump 最慢 {worst:?}");
    assert!(f1 - f0 > 100.0, "死算期间 rAF 照常跑（{} 帧）", f1 - f0);
    h.engine.eval("go('throw')").unwrap();
    wait(&mut h, "onError", 5.0);
    assert!(w_ev(&h, "onError")[0]["message"].as_str().unwrap().contains("故意抛错"));
    h.engine.eval("go('fn')").unwrap();
    h.vsyncs(1);
    assert_eq!(w_ev(&h, "fnErr")[0]["name"], "DataCloneError");
    h.assert_no_fail();
}

/// W2：16MB ArrayBuffer + 各种类型往返：内容相同、transfer 之后源 byteLength == 0、视图与 buffer 的共享关系、循环引用都还原
#[test]
fn w2_结构化克隆_16mb_transfer() {
    let _g = serial();
    let mut h = GameHarness::open("g-worker", Opts::default());
    h.engine.eval("go('create')").unwrap();
    wait(&mut h, "ready", 5.0);
    h.engine.eval("go('echo', { mb: 16, transfer: true, back: true })").unwrap();
    wait(&mut h, "afterTransfer", 20.0);
    let sent = &w_ev(&h, "sent")[0];
    assert_eq!(sent["lenAfter"], 0, "transfer 之后源 buffer detach");
    let e = &w_ev(&h, "echoed")[0];
    let n = 16 * 1024 * 1024;
    assert_eq!((e["info"]["len"].as_u64(), &e["info"]["sum"], e["info"]["sameBuf"].as_bool()), (Some(n), &sent["sum"], Some(true)), "Worker 收到的：{e}");
    let b = &e["back"];
    assert_eq!((b["len"].as_u64(), &b["sum"]), (Some(n), &sent["sum"]), "回到主线程的内容相同");
    for k in ["nan", "negZero", "undef", "sameBuf", "cyc"] {
        assert_eq!(b[k], true, "{k}：{b}");
    }
    assert_eq!((b["big"].as_str(), b["date"].as_f64(), b["map"].as_str(), b["set"].as_bool(), b["re"].as_str(), b["viewType"].as_str()),
        (Some("12345678901234567890"), Some(1.7e12), Some("v"), Some(true), Some("gi"), Some("Float32Array")), "{b}");
    assert_eq!(w_ev(&h, "afterTransfer")[0]["len"], 0, "Worker 那边 transfer 回来之后也 detach");
}

/// W3 / W4 / 被结束：再建一个先停掉旧的；terminate 之后线程退出（计数回原值）；testOnProcessKilled → onProcessKilled
#[test]
fn w3_w4_终止与第二个() {
    let _g = serial();
    let base = GameEngine::live_worker_threads();
    let mut h = GameHarness::open("g-worker", Opts::default());
    h.engine.eval("go('create', 'a')").unwrap();
    wait(&mut h, "ready", 5.0);
    h.engine.eval("go('busy', 3000)").unwrap(); // 旧的在死算：terminate 也要能停下来
    h.vsyncs(1);
    h.engine.eval("go('create', 'b')").unwrap();
    let t0 = Instant::now();
    while h.events("w").iter().filter(|e| e["cmd"] == "ready").count() < 2 {
        assert!(t0.elapsed().as_secs() < 5);
        h.vsyncs(1);
    }
    assert!(h.log.iter().any(|l| l.contains("同时只能有一个 Worker")));
    let t0 = Instant::now();
    while GameEngine::live_worker_threads() > base + 1 {
        assert!(t0.elapsed().as_secs() < 3, "死算中的旧 Worker 没停下来");
        std::thread::sleep(Duration::from_millis(10));
    }
    h.engine.eval("go('kill')").unwrap();
    wait(&mut h, "killed", 5.0);
    assert_eq!(w_ev(&h, "killed")[0]["tag"], "b");
    h.engine.eval("go('create', 'c')").unwrap();
    wait(&mut h, "ready", 5.0);
    h.engine.eval("go('terminate')").unwrap();
    let t0 = Instant::now();
    while GameEngine::live_worker_threads() > base {
        assert!(t0.elapsed().as_secs() < 3, "terminate 之后线程没退出（{} 个）", GameEngine::live_worker_threads());
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!h.events("w").iter().any(|e| e["tag"] == "a" && e["cmd"] == "busyDone"), "旧 Worker 被停掉，不该再有回复");
}
