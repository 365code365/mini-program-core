//! 原生 tabBar 的运行时 API（官方 api/ui/tab-bar/*）：`setTabBarBadge` / `removeTabBarBadge` /
//! `showTabBarRedDot` / `hideTabBarRedDot` / `setTabBarItem` / `setTabBarStyle` / `showTabBar` / `hideTabBar`。
//!
//! 这些只作用于 app.json 声明的**原生** tabBar（`custom: true` 的自定义 tabBar 由页面自己画，
//! 微信里调这些同样不生效，返回 fail）。改动记在线程局部的覆盖表里，绘制时叠到配置上
//! （[`effective`] 换文字 / 颜色，[`mark`] 给角标与红点）；[`take_requests`] 返回真时宿主重画 tabBar。
//! `hideTabBar` 之后当前页按「不是 tabBar 页」排版（视口变高），`showTabBar` 恢复。
//!
//! 角标文字超过 4 个字符时截断成前 3 个 + `…`（与微信一致）。

use super::config::TabBarConfig;
use crate::runtime::MiniApp;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct Overrides {
    badges: HashMap<usize, String>,
    dots: HashSet<usize>,
    texts: HashMap<usize, String>,
    icons: HashMap<usize, (Option<String>, Option<String>)>,
    color: Option<String>,
    selected_color: Option<String>,
    background_color: Option<String>,
    hidden: bool,
}

thread_local! {
    static STATE: RefCell<Overrides> = RefCell::new(Overrides::default());
}

/// 某一项上的标记：角标文字（Some）或红点（None 且 `dot`）
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub badge: Option<String>,
    pub dot: bool,
}

/// `hideTabBar` 之后为真
pub fn hidden() -> bool {
    STATE.with(|s| s.borrow().hidden)
}

/// 小程序重启 / 引擎销毁时清空
pub fn reset() {
    STATE.with(|s| *s.borrow_mut() = Overrides::default());
}

/// 叠上覆盖之后的配置（文字、图标路径、颜色）
pub fn effective(cfg: &TabBarConfig) -> TabBarConfig {
    STATE.with(|s| {
        let s = s.borrow();
        let mut c = cfg.clone();
        for (i, item) in c.list.iter_mut().enumerate() {
            if let Some(t) = s.texts.get(&i) {
                item.text = t.clone();
            }
            if let Some((icon, sel)) = s.icons.get(&i) {
                if let Some(p) = icon {
                    item.icon_path = p.clone();
                }
                if let Some(p) = sel {
                    item.selected_icon_path = p.clone();
                }
            }
        }
        if let Some(v) = &s.color {
            c.color = v.clone();
        }
        if let Some(v) = &s.selected_color {
            c.selected_color = v.clone();
        }
        if let Some(v) = &s.background_color {
            c.background_color = v.clone();
        }
        c
    })
}

/// 第 `i` 项的角标 / 红点（角标优先：微信里设了角标红点就不显示）
pub fn mark(i: usize) -> Option<Mark> {
    STATE.with(|s| {
        let s = s.borrow();
        match (s.badges.get(&i), s.dots.contains(&i)) {
            (Some(b), _) => Some(Mark { badge: Some(b.clone()), dot: false }),
            (None, true) => Some(Mark { badge: None, dot: true }),
            _ => None,
        }
    })
}

fn badge_text(t: &str) -> String {
    if t.chars().count() > 4 {
        let head: String = t.chars().take(3).collect();
        format!("{head}…")
    } else {
        t.to_string()
    }
}

/// 每帧：执行逻辑层的 tabBar 请求，回 success / fail。`native` 是 app.json 的原生 tabBar 配置
/// （没有、或是自定义 tabBar 时全部 fail）。返回是否有改动（要重画 tabBar / 重新排版）
pub fn take_requests(app: &mut MiniApp, native: Option<&TabBarConfig>) -> bool {
    let reqs = app.take_host_requests("tabBar");
    if reqs.is_empty() {
        return false;
    }
    let mut changed = false;
    for req in reqs {
        let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
        let op = req.get("op").and_then(Value::as_str).unwrap_or("").to_string();
        let result = apply(&op, &req, native);
        if result.is_ok() {
            changed = true;
        }
        let err = match &result {
            Ok(()) => String::new(),
            Err(e) => e.clone(),
        };
        let js = format!("__resolveTabBar({id}, {}, {})", result.is_ok(), serde_json::to_string(&err).unwrap_or_default());
        app.eval(&js).ok();
    }
    changed
}

fn apply(op: &str, req: &Value, native: Option<&TabBarConfig>) -> Result<(), String> {
    let Some(cfg) = native.filter(|c| !c.custom && !c.list.is_empty()) else {
        return Err("not TabBar page".into());
    };
    let index = || -> Result<usize, String> {
        let i = req.get("index").and_then(Value::as_u64).ok_or("parameter error: index is required")? as usize;
        if i >= cfg.list.len() {
            return Err(format!("tabbar item not found: index {i}"));
        }
        Ok(i)
    };
    let s = |k: &str| req.get(k).and_then(Value::as_str).map(str::to_string);
    STATE.with(|st| {
        let mut st = st.borrow_mut();
        match op {
            "setTabBarBadge" => {
                let i = index()?;
                st.badges.insert(i, badge_text(&s("text").unwrap_or_default()));
            }
            "removeTabBarBadge" => {
                let i = index()?;
                st.badges.remove(&i);
            }
            "showTabBarRedDot" => {
                let i = index()?;
                st.dots.insert(i);
            }
            "hideTabBarRedDot" => {
                let i = index()?;
                st.dots.remove(&i);
            }
            "setTabBarItem" => {
                let i = index()?;
                if let Some(t) = s("text") {
                    st.texts.insert(i, t);
                }
                let (icon, sel) = (s("iconPath"), s("selectedIconPath"));
                if icon.is_some() || sel.is_some() {
                    st.icons.insert(i, (icon, sel));
                }
            }
            "setTabBarStyle" => {
                if let Some(v) = s("color") {
                    st.color = Some(v);
                }
                if let Some(v) = s("selectedColor") {
                    st.selected_color = Some(v);
                }
                if let Some(v) = s("backgroundColor") {
                    st.background_color = Some(v);
                }
            }
            "showTabBar" => st.hidden = false,
            "hideTabBar" => st.hidden = true,
            other => return Err(format!("unknown op {other}")),
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_text_truncates_past_four_chars() {
        assert_eq!(badge_text("99+"), "99+");
        assert_eq!(badge_text("1234"), "1234");
        assert_eq!(badge_text("12345"), "123…");
    }
}
