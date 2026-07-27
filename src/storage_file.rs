//! `wx.setStorageSync` 的落盘。
//!
//! 微信的 storage 是**跨启动持久**的（同一个小程序下次打开还能读到）。之前这里只有
//! 一个进程内的 HashMap，于是每次启动都是空的 —— 任何「首次进入拉数据存起来、
//! 之后走缓存」的应用都永远停在首次分支，缓存逻辑等于从没被执行过。
//!
//! 存放位置刻意放在 `target/mini-storage/<小程序名>.json`：
//! - 按小程序分文件，互不干扰（微信也是按 appid 隔离）
//! - **不写进 `sample/`** —— 示例小程序是双端一致性的基准输入，不能被运行时产物污染

use std::collections::HashMap;
use std::path::PathBuf;

/// 当前小程序的 storage 文件路径。拿不到小程序目录时返回 None（退化成纯内存）。
fn storage_path() -> Option<PathBuf> {
    let root = crate::assets::app_root()?;
    let name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("mini-storage");
    Some(dir.join(format!("{}.json", name)))
}

/// 读回上次会话存下的键值对
pub fn load() -> HashMap<String, String> {
    let Some(path) = storage_path() else { return HashMap::new() };
    let Ok(text) = std::fs::read_to_string(&path) else { return HashMap::new() };
    match serde_json::from_str::<HashMap<String, String>>(&text) {
        Ok(map) => {
            if std::env::var("MINI_STORAGE_LOG").is_ok() {
                eprintln!("💾 读回 storage {} 项 <- {}", map.len(), path.display());
            }
            map
        }
        // 文件损坏就当空的，不要因为一份缓存把应用拖挂
        Err(_) => HashMap::new(),
    }
}

/// 写回磁盘。失败只警告不报错 —— 缓存写不下去不该让应用崩。
pub fn save(map: &HashMap<String, String>) {
    let Some(path) = storage_path() else { return };
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let Ok(text) = serde_json::to_string(map) else { return };
    if let Err(e) = std::fs::write(&path, text) {
        eprintln!("⚠️  storage 写入失败 {}: {}", path.display(), e);
    } else if std::env::var("MINI_STORAGE_LOG").is_ok() {
        eprintln!("💾 写入 storage {} 项 -> {}", map.len(), path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 三段断言合在一个测试里：`assets::set_app_root` 是**进程级全局**，
    /// 拆成多个 `#[test]` 会因为并行执行互相改写而随机失败
    /// （踩过一次：损坏文件那条读到了另一个用例写的合法 JSON）。
    #[test]
    fn storage_file_path_and_round_trip() {
        // ① 路径按小程序名隔离，且绝不落在 sample/ 里
        crate::assets::set_app_root(PathBuf::from("/tmp/whatever/tea-app"));
        let p = storage_path().expect("应能算出路径");
        assert!(p.ends_with("tea-app.json"), "按小程序名分文件: {:?}", p);
        let s = p.to_string_lossy().to_string();
        assert!(s.contains("mini-storage"), "要放在 mini-storage 目录下: {}", s);
        assert!(!s.contains("/sample/"), "绝不能写进 sample/：{}", s);

        // ② 写进去能读回来
        crate::assets::set_app_root(PathBuf::from("/tmp/whatever/__storage_test_app"));
        let mut m = HashMap::new();
        m.insert("k".to_string(), "{\"a\":1}".to_string());
        save(&m);
        assert_eq!(load().get("k").map(|s| s.as_str()), Some("{\"a\":1}"));
        if let Some(p) = storage_path() {
            let _ = std::fs::remove_file(p);
        }

        // ③ 文件损坏时退化成空，不能让一份坏缓存把应用拖挂
        crate::assets::set_app_root(PathBuf::from("/tmp/whatever/__storage_bad_app"));
        if let Some(p) = storage_path() {
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            let _ = std::fs::write(&p, "{ this is not json");
            assert!(load().is_empty(), "损坏的缓存文件要当成空的");
            let _ = std::fs::remove_file(p);
        }
    }
}
