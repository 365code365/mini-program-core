//! 应用配置结构体

use serde::Deserialize;

/// app.json 配置结构
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub pages: Vec<String>,
    #[serde(default)]
    pub window: WindowConfig,
    #[serde(default)]
    pub tab_bar: Option<TabBarConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowConfig {
    #[serde(default = "default_nav_title")]
    pub navigation_bar_title_text: String,
    #[serde(default = "default_nav_bg")]
    pub navigation_bar_background_color: String,
    #[serde(default)]
    pub navigation_bar_text_style: String,
    #[serde(default = "default_bg")]
    pub background_color: String,
}

fn default_nav_title() -> String { "Mini App".to_string() }
fn default_nav_bg() -> String { "#000000".to_string() }
fn default_bg() -> String { "#FFFFFF".to_string() }

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TabBarConfig {
    #[serde(default)]
    pub custom: bool,
    #[serde(default = "default_tab_color")]
    pub color: String,
    #[serde(default = "default_tab_selected")]
    pub selected_color: String,
    #[serde(default = "default_tab_bg")]
    pub background_color: String,
    #[serde(default)]
    pub list: Vec<TabBarItem>,
}

fn default_tab_color() -> String { "#999999".to_string() }
fn default_tab_selected() -> String { "#007AFF".to_string() }
fn default_tab_bg() -> String { "#FFFFFF".to_string() }

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TabBarItem {
    pub page_path: String,
    pub text: String,
    #[serde(default)]
    pub icon_path: String,
    #[serde(default)]
    pub selected_icon_path: String,
}
