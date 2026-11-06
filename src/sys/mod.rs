//! 引擎跑在哪台机器上的那部分：可写数据目录、storage 落盘、剪贴板、帧时钟、内存驻留探测。
//! 宿主（桌面窗体 / 移动端 SDK）在建引擎前把目录与时钟交进来，其余模块只经这里读。

/// 引擎的可写数据目录（storage 落盘、图片磁盘缓存），由宿主指定（环境变量 `MINI_DATA_DIR` 优先）
pub mod data_dir;
/// `wx.setStorageSync` 的跨启动落盘
pub mod storage_file;
/// `wx.setClipboardData` / `getClipboardData`（引擎里一份，桌面窗体可同步系统剪贴板）
pub mod clipboard;
/// 帧时钟：这一帧按哪个时刻画（SDK 按宿主 vsync 时间，见模块注释）
pub mod frame_clock;
/// 一块缓冲真正驻留了多少（mincore）
pub mod mem_probe;
