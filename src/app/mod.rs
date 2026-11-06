//! 小程序包本身：在哪（目录定位）、是什么（小程序还是小游戏）、资源根、自定义组件、「关于小程序」。
//! 与运行时（`runtime/`）、宿主（`host/`）无关，桌面、SDK、H5 编译器都用。

/// 「关于小程序」的信息（胶囊「•••」展示的内容；桌面、SDK、H5 共用）
pub mod about;
/// 示例小程序目录的定位（裸名字 / `sample/xxx` / 任意路径都能解析）
pub mod dir;
/// 包种类探测：小程序（app.json）还是小游戏（game.json），两者都有 / 都没有报错
pub mod profile;
/// 小程序资源根目录（图片等相对路径解析）
pub mod assets;
/// 页面 json 的 `usingComponents`：自定义组件三件套加载与样式作用域
pub mod using_components;
