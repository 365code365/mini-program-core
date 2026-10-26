//! 包种类探测：一个目录是小程序还是小游戏（game-doc/02 §1）。
//!
//! 从前只认 `app.json`，没有它时 `page_loader::load_app_json` **静默**套一份默认配置 —— 一个小游戏包
//! （只有 `game.json` + `game.js`）会被当成一个空白小程序打开，`game.js` 一行都不执行，屏幕上只有默认导航栏，
//! 没有任何报错。所以入口处（创建引擎、桌面窗体）先过这里：认不出就报错，不猜。
//!
//! 两个都有也报错而不是择一：微信里不存在这种包，择一会让打包错误静默通过。

use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppProfile {
    MiniProgram,
    MiniGame,
}

/// 探测失败的原因（文案就是给开发者看的那一行）
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileError {
    /// 目录下 `app.json` 与 `game.json` 都没有
    Neither(String),
    /// 两个都有
    Both(String),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileError::Neither(d) => write!(f, "不是小程序也不是小游戏：{d} 下没有 app.json / game.json"),
            ProfileError::Both(d) => write!(f, "{d} 下同时有 app.json 与 game.json：一个包只能是小程序或小游戏之一"),
        }
    }
}

impl std::error::Error for ProfileError {}

/// 有 `game.json` → 小游戏；有 `app.json` → 小程序；都有 / 都没有 → 报错
pub fn detect(dir: &Path) -> Result<AppProfile, ProfileError> {
    let app = dir.join("app.json").is_file();
    let game = dir.join("game.json").is_file();
    match (app, game) {
        (true, false) => Ok(AppProfile::MiniProgram),
        (false, true) => Ok(AppProfile::MiniGame),
        (true, true) => Err(ProfileError::Both(dir.display().to_string())),
        (false, false) => Err(ProfileError::Neither(dir.display().to_string())),
    }
}

/// 只接受小程序的入口（`MiniEngine`、桌面窗体的小程序模式）用：小游戏给出明确的「尚未支持 / 走游戏入口」
pub fn require_mini_program(dir: &Path) -> Result<(), String> {
    match detect(dir) {
        Ok(AppProfile::MiniProgram) => Ok(()),
        Ok(AppProfile::MiniGame) => Err(format!("{} 是小游戏（有 game.json），不能按小程序打开", dir.display())),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str, files: &[&str]) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("mr-profile-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        for f in files {
            std::fs::write(d.join(f), "{}").unwrap();
        }
        d
    }

    #[test]
    fn 四种组合() {
        let a = dir("a", &["app.json"]);
        let g = dir("g", &["game.json"]);
        let b = dir("b", &["app.json", "game.json"]);
        let n = dir("n", &[]);
        assert_eq!(detect(&a), Ok(AppProfile::MiniProgram));
        assert_eq!(detect(&g), Ok(AppProfile::MiniGame));
        assert!(matches!(detect(&b), Err(ProfileError::Both(_))));
        assert!(matches!(detect(&n), Err(ProfileError::Neither(_))));
        assert!(detect(&n).unwrap_err().to_string().contains("不是小程序也不是小游戏"));
        assert!(require_mini_program(&g).unwrap_err().contains("是小游戏"));
        for d in [a, g, b, n] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    #[test]
    fn 目录名叫_game_json_的不算() {
        // game.json 是目录（不是文件）时不认：is_file 而不是 exists
        let d = dir("dirjson", &["app.json"]);
        std::fs::create_dir_all(d.join("game.json")).unwrap();
        assert_eq!(detect(&d), Ok(AppProfile::MiniProgram));
        let _ = std::fs::remove_dir_all(d);
    }
}
