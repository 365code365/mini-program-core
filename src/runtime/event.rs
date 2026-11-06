//! 宿主推给逻辑层的 App 生命周期事件（切前后台）。触摸、点击、页面生命周期不走这里：
//! 触摸链路的事件对象由渲染层拼好直接 `__dispatchEvent`（host/touch.rs），页面生命周期由页面栈直接调 `__dispatchPage`。

/// App 级生命周期
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// 回到前台：`App.onShow`
    AppShow,
    /// 退到后台：`App.onHide`
    AppHide,
}
