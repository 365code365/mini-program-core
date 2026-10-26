//! `game.json`（官方「小游戏配置」，game-doc/02 §1）。
//!
//! 只认官方字段；未知字段不报错（微信也不报），但记下来，`MINI_GAME_LOG=1` 时列出 ——
//! 拼错一个字段（`deviceOrientaion`）在微信里同样是静默无效，这里至少能看到。
//! 字段类型错（比如 `subpackages` 给了对象）报错并指出字段：微信开发者工具编译时也会拦下这种包。

use serde_json::Value;

/// `deviceOrientation` / `desktopDeviceOrientation` 的合法值
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Portrait,
    /// home 键在右侧的横屏；未锁定时随手机旋转
    Landscape,
    LandscapeLeft,
    LandscapeRight,
}

impl Orientation {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "portrait" => Self::Portrait,
            "landscape" => Self::Landscape,
            "landscapeLeft" => Self::LandscapeLeft,
            "landscapeRight" => Self::LandscapeRight,
            _ => return None,
        })
    }
    pub fn is_landscape(self) -> bool {
        self != Self::Portrait
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Portrait => "portrait",
            Self::Landscape => "landscape",
            Self::LandscapeLeft => "landscapeLeft",
            Self::LandscapeRight => "landscapeRight",
        }
    }
}

/// 一个分包。`root` 可以是目录（`stage1/`）或单个 js 文件（`one.js`）
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subpackage {
    pub name: String,
    pub root: String,
    pub independent: bool,
}

/// `plugins` 里的一项（引擎插件等，game-doc/11 B-7）
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plugin {
    pub name: String,
    pub provider: String,
    pub version: String,
    /// 包内兜底副本（Cocos「引擎分离」允许本地兜底）
    pub path: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameConfig {
    pub device_orientation: Orientation,
    pub desktop_device_orientation: Orientation,
    pub show_status_bar: bool,
    /// 毫秒；缺省 60000（官方默认）
    pub timeout_request: u64,
    pub timeout_connect_socket: u64,
    pub timeout_upload_file: u64,
    pub timeout_download_file: u64,
    /// Worker 根目录
    pub workers: Option<String>,
    pub subpackages: Vec<Subpackage>,
    pub open_data_context: Option<String>,
    pub plugins: Vec<Plugin>,
    pub resizable: bool,
    /// 原文（`__wxConfig` 一类运行时元数据、以后的新字段从这里取）
    pub raw: Value,
    /// 不认识的顶层字段（只用于诊断）
    pub unknown_fields: Vec<String>,
    /// 解析时的警告（非法枚举值回落之类），由调用方打出
    pub warnings: Vec<String>,
}

const KNOWN: &[&str] = &[
    "deviceOrientation", "desktopDeviceOrientation", "showStatusBar", "networkTimeout", "workers",
    "subpackages", "subPackages", "parallelPreloadSubpackages", "navigateToMiniProgramAppIdList", "permission",
    "openDataContext", "resizable", "displayMode", "disableSetUserStorageFromMiniProgram", "lockStepOptions",
    "iOSHighPerformance", "iOSHighPerformance+", "plugins",
];

const DEFAULT_TIMEOUT_MS: u64 = 60_000;

impl GameConfig {
    /// 解析 `game.json` 的文本。空文件按 `{}`（微信允许 game.json 只写 `{}`）
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        let raw: Value = if text.is_empty() {
            Value::Object(Default::default())
        } else {
            serde_json::from_str(text).map_err(|e| format!("game.json 不是合法的 JSON：{e}"))?
        };
        let obj = raw.as_object().ok_or("game.json 顶层必须是对象")?;
        let mut warnings = Vec::new();

        let orient = |key: &str, warnings: &mut Vec<String>| -> Result<Option<Orientation>, String> {
            match obj.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::String(s)) => match Orientation::parse(s) {
                    Some(o) => Ok(Some(o)),
                    None => {
                        warnings.push(format!("game.json {key} 的值 {s:?} 不合法，按 portrait 处理"));
                        Ok(Some(Orientation::Portrait))
                    }
                },
                Some(_) => Err(format!("game.json {key} 应当是字符串")),
            }
        };
        let device_orientation = orient("deviceOrientation", &mut warnings)?.unwrap_or(Orientation::Portrait);
        // 官方：desktopDeviceOrientation 未填时取 deviceOrientation 的值
        let desktop_device_orientation = orient("desktopDeviceOrientation", &mut warnings)?.unwrap_or(device_orientation);

        let bool_field = |key: &str| -> Result<bool, String> {
            match obj.get(key) {
                None | Some(Value::Null) => Ok(false),
                Some(Value::Bool(b)) => Ok(*b),
                Some(_) => Err(format!("game.json {key} 应当是布尔值")),
            }
        };
        let show_status_bar = bool_field("showStatusBar")?;
        let resizable = bool_field("resizable")?;

        let nt = match obj.get("networkTimeout") {
            None | Some(Value::Null) => serde_json::Map::new(),
            Some(Value::Object(m)) => m.clone(),
            Some(_) => return Err("game.json networkTimeout 应当是对象".into()),
        };
        let timeout = |key: &str| -> Result<u64, String> {
            match nt.get(key) {
                None | Some(Value::Null) => Ok(DEFAULT_TIMEOUT_MS),
                Some(v) => v
                    .as_f64()
                    .filter(|n| n.is_finite() && *n >= 0.0)
                    .map(|n| n as u64)
                    .ok_or_else(|| format!("game.json networkTimeout.{key} 应当是非负数字")),
            }
        };

        // workers：字符串，或 `{ path, isSubpackage }`
        let workers = match obj.get("workers") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(Value::Object(m)) => Some(
                m.get("path").and_then(|v| v.as_str()).ok_or("game.json workers.path 应当是字符串")?.to_string(),
            ),
            Some(_) => return Err("game.json workers 应当是字符串或对象".into()),
        };

        // subpackages 与 subPackages 两种写法官方都认；两个都写时合并（微信开发者工具的行为未核实，取并集最不意外）
        let mut subpackages = Vec::new();
        for key in ["subpackages", "subPackages"] {
            match obj.get(key) {
                None | Some(Value::Null) => {}
                Some(Value::Array(list)) => {
                    for (i, it) in list.iter().enumerate() {
                        let name = it.get("name").and_then(|v| v.as_str());
                        let root = it.get("root").and_then(|v| v.as_str()).ok_or(format!("game.json {key}[{i}].root 必填"))?;
                        subpackages.push(Subpackage {
                            name: name.unwrap_or(root).to_string(),
                            root: root.to_string(),
                            independent: it.get("independent").and_then(|v| v.as_bool()).unwrap_or(false),
                        });
                    }
                }
                Some(_) => return Err(format!("game.json {key} 应当是数组")),
            }
        }

        let open_data_context = match obj.get("openDataContext") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => return Err("game.json openDataContext 应当是字符串".into()),
        };

        let mut plugins = Vec::new();
        match obj.get("plugins") {
            None | Some(Value::Null) => {}
            Some(Value::Object(m)) => {
                for (name, p) in m {
                    let s = |k: &str| p.get(k).and_then(|v| v.as_str()).map(str::to_string);
                    plugins.push(Plugin {
                        name: name.clone(),
                        provider: s("provider").ok_or(format!("game.json plugins.{name}.provider 必填"))?,
                        version: s("version").unwrap_or_default(),
                        path: s("path"),
                    });
                }
            }
            Some(_) => return Err("game.json plugins 应当是对象".into()),
        }

        let unknown_fields = obj.keys().filter(|k| !KNOWN.contains(&k.as_str())).cloned().collect();
        Ok(Self {
            device_orientation,
            desktop_device_orientation,
            show_status_bar,
            timeout_request: timeout("request")?,
            timeout_connect_socket: timeout("connectSocket")?,
            timeout_upload_file: timeout("uploadFile")?,
            timeout_download_file: timeout("downloadFile")?,
            workers,
            subpackages,
            open_data_context,
            plugins,
            resizable,
            raw,
            unknown_fields,
            warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 空文件与默认值() {
        for t in ["", "{}", "  \n"] {
            let c = GameConfig::parse(t).unwrap();
            assert_eq!(c.device_orientation, Orientation::Portrait);
            assert_eq!(c.desktop_device_orientation, Orientation::Portrait);
            assert!(!c.show_status_bar);
            assert_eq!(c.timeout_request, 60_000);
            assert!(c.subpackages.is_empty() && c.plugins.is_empty() && c.warnings.is_empty());
        }
    }

    #[test]
    fn 方向_非法值回落_portrait_并警告_桌面方向缺省跟随() {
        let c = GameConfig::parse(r#"{"deviceOrientation":"sideways"}"#).unwrap();
        assert_eq!(c.device_orientation, Orientation::Portrait);
        assert_eq!(c.warnings.len(), 1);
        let c = GameConfig::parse(r#"{"deviceOrientation":"landscape"}"#).unwrap();
        assert_eq!(c.desktop_device_orientation, Orientation::Landscape);
        let c = GameConfig::parse(r#"{"deviceOrientation":"landscape","desktopDeviceOrientation":"portrait"}"#).unwrap();
        assert_eq!(c.desktop_device_orientation, Orientation::Portrait);
        assert!(GameConfig::parse(r#"{"deviceOrientation":1}"#).is_err());
    }

    #[test]
    fn 分包两种写法与单文件_root() {
        let c = GameConfig::parse(
            r#"{"subpackages":[{"name":"stage1","root":"stage1/"},{"root":"one.js"}],
                "subPackages":[{"name":"solo","root":"solo/","independent":true}]}"#,
        )
        .unwrap();
        assert_eq!(c.subpackages.len(), 3);
        assert_eq!(c.subpackages[1].name, "one.js", "没写 name 时以 root 为名");
        assert!(c.subpackages[2].independent);
        assert!(GameConfig::parse(r#"{"subpackages":{}}"#).unwrap_err().contains("subpackages"));
        assert!(GameConfig::parse(r#"{"subpackages":[{"name":"x"}]}"#).unwrap_err().contains("root"));
    }

    #[test]
    fn 超时_workers_插件_未知字段() {
        let c = GameConfig::parse(
            r#"{"networkTimeout":{"request":500},"workers":{"path":"workers","isSubpackage":true},
                "plugins":{"cocos":{"provider":"wx7095f7fa398a2f30","version":"3.8.0","path":"cocos"}},
                "deviceOrientaion":"landscape"}"#,
        )
        .unwrap();
        assert_eq!(c.timeout_request, 500);
        assert_eq!(c.timeout_download_file, 60_000);
        assert_eq!(c.workers.as_deref(), Some("workers"));
        assert_eq!(c.plugins[0].provider, "wx7095f7fa398a2f30");
        assert_eq!(c.plugins[0].path.as_deref(), Some("cocos"));
        assert_eq!(c.unknown_fields, vec!["deviceOrientaion".to_string()], "拼错的字段要能被列出来");
        assert!(GameConfig::parse(r#"{"networkTimeout":{"request":-1}}"#).is_err());
        assert!(GameConfig::parse("[]").is_err());
        assert!(GameConfig::parse("{oops").unwrap_err().contains("不是合法的 JSON"));
    }
}
