//! 本机起一份商店服务（tools/mini-store/server.py，带 /api/demo/ 演示数据接口），给拉网络数据的示例
//! （光影放映厅、云听音乐、拾光图库、跳转中心）跑场景测试：不连公网服务器，数据与服务器上同一份代码生成。
//!
//! 一个测试进程起一次（[`base`]）。测试进程退出时 static 的 `Drop` 不会跑，所以服务套一层看门狗：
//! 它读自己的标准输入，测试进程一退出管道就断（EOF），看门狗杀掉服务再退出 —— 不会留下孤儿进程。
use std::io::Write;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

struct Server {
    _child: Child,
    /// 拿着不放：放掉（或测试进程退出）看门狗就收到 EOF、杀掉服务
    _stdin: std::process::ChildStdin,
    base: String,
}

const WATCHDOG: &str = "import subprocess,sys\np=subprocess.Popen(sys.argv[1:])\ntry:\n    sys.stdin.read()\nfinally:\n    p.kill()\n";

static SERVER: OnceLock<Mutex<Server>> = OnceLock::new();

/// 演示接口的地址（`http://127.0.0.1:<端口>`）。没有 python3 时 panic（测试环境要有）
pub fn base() -> String {
    SERVER.get_or_init(|| Mutex::new(start())).lock().unwrap_or_else(|p| p.into_inner()).base.clone()
}

fn start() -> Server {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = repo.join("tools/mini-store/server.py");
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("demo-store-{}", std::process::id()));
    std::fs::remove_dir_all(&data).ok();
    std::fs::create_dir_all(&data).unwrap();
    // 服务启动要有管理账户（serve 先读 config.json）；演示接口本身是公开的
    let mut sp = Command::new("python3")
        .arg(&script)
        .args(["set-password", "admin"])
        .env("MINI_STORE_DATA", &data)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("要有 python3");
    sp.stdin.take().unwrap().write_all(b"test-password-1234\n").unwrap();
    assert!(sp.wait().unwrap().success(), "set-password 失败");
    let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let mut child = Command::new("python3")
        .args(["-c", WATCHDOG, "python3"])
        .arg(&script)
        .arg("serve")
        .env("MINI_STORE_DATA", &data)
        .env("MINI_STORE_PORT", port.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdin = child.stdin.take().unwrap();
    let t0 = Instant::now();
    while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(t0.elapsed() < Duration::from_secs(10), "本机商店服务 10 秒没起来");
        std::thread::sleep(Duration::from_millis(50));
    }
    Server { _child: child, _stdin: stdin, base: format!("http://127.0.0.1:{port}") }
}
