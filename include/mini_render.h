/**
 * Mini Render Engine - C API
 * 类似 Skia 的轻量级渲染引擎
 */

#ifndef MINI_RENDER_H
#define MINI_RENDER_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// 画布句柄
typedef struct Canvas Canvas;

// 路径句柄
typedef struct Path Path;

// 画笔样式
typedef enum {
    MR_STYLE_FILL = 0,
    MR_STYLE_STROKE = 1,
    MR_STYLE_FILL_AND_STROKE = 2
} MRPaintStyle;

// ============ Canvas API ============

// 创建画布
Canvas* mr_canvas_new(uint32_t width, uint32_t height);

// 销毁画布
void mr_canvas_free(Canvas* canvas);

// 清空画布
void mr_canvas_clear(Canvas* canvas, uint8_t r, uint8_t g, uint8_t b, uint8_t a);

// 获取画布尺寸
uint32_t mr_canvas_width(const Canvas* canvas);
uint32_t mr_canvas_height(const Canvas* canvas);

// 绘制矩形
void mr_canvas_draw_rect(
    Canvas* canvas,
    float x, float y, float width, float height,
    uint8_t r, uint8_t g, uint8_t b, uint8_t a,
    uint8_t style, float stroke_width
);

// 绘制圆形
void mr_canvas_draw_circle(
    Canvas* canvas,
    float cx, float cy, float radius,
    uint8_t r, uint8_t g, uint8_t b, uint8_t a,
    uint8_t style, float stroke_width
);

// 绘制线段
void mr_canvas_draw_line(
    Canvas* canvas,
    float x0, float y0, float x1, float y1,
    uint8_t r, uint8_t g, uint8_t b, uint8_t a,
    float stroke_width
);

// 绘制路径
void mr_canvas_draw_path(
    Canvas* canvas,
    const Path* path,
    uint8_t r, uint8_t g, uint8_t b, uint8_t a,
    uint8_t style, float stroke_width
);

// 获取像素数据
size_t mr_canvas_get_pixels(const Canvas* canvas, uint8_t* out, size_t len);

// 保存为 PNG
bool mr_canvas_save_png(const Canvas* canvas, const char* path);

// ============ Path API ============

// 创建路径
Path* mr_path_new(void);

// 销毁路径
void mr_path_free(Path* path);

// 移动到
void mr_path_move_to(Path* path, float x, float y);

// 画线到
void mr_path_line_to(Path* path, float x, float y);

// 二次贝塞尔曲线
void mr_path_quad_to(Path* path, float cx, float cy, float x, float y);

// 三次贝塞尔曲线
void mr_path_cubic_to(Path* path, float c1x, float c1y, float c2x, float c2y, float x, float y);

// 闭合路径
void mr_path_close(Path* path);

// 添加圆角矩形
void mr_path_add_round_rect(Path* path, float x, float y, float w, float h, float radius);

// 添加椭圆
void mr_path_add_oval(Path* path, float cx, float cy, float rx, float ry);


// ============ App API：跑一个小程序（移动端 SDK 用的就是这套） ============
//
// 线程约定：所有 mr_app_* 必须在**同一个线程**调用（逻辑层是 QuickJS，非线程安全）。
// 坐标一律是**逻辑坐标**（iOS 的 pt / Android 的 dp，左上原点）；
// 像素缓冲尺寸 = 逻辑尺寸 x dpr，用 mr_app_pixel_size 问。
//
// 最小用法：
//   MRApp* app = mr_app_create(dir, sandbox, 375, 667, 3.0f);
//   mr_app_launch(app, NULL);                  // NULL = app.json 首页
//   if (mr_app_pump(app, now_ms)) mr_app_pixels(app, buf, len);   // RGBA8
//   mr_app_pointer_down/move/up(app, x, y);    // 必须送完整三段
//   if (!mr_app_back(app)) { /* 页面栈空了，关掉小程序 */ }
//   mr_app_destroy(app);

typedef struct MRApp MRApp;

// 创建实例；失败返回 NULL。
// data_dir 是宿主沙盒里可写的目录：storage 与图片磁盘缓存落在这里，
// 传 NULL 会让这两样**静默失效**（写不进也不报错）。
MRApp* mr_app_create(const char* app_dir, const char* data_dir,
                     uint32_t width, uint32_t height, float dpr);
void   mr_app_destroy(MRApp* app);

// 打开页面（route 为 NULL 时用 app.json 的首页）；成功返回 1
int    mr_app_launch(MRApp* app, const char* route);
// 深链跳页，如 "/pages/detail/detail?id=1"；成功返回 1
int    mr_app_navigate(MRApp* app, const char* url);

// 跑一帧。返回 1 表示有新画面该上屏，0 表示这帧没变化（可以不刷）。
// now_ms 用宿主的单调时钟；传 0 让引擎自己取。
int    mr_app_pump(MRApp* app, uint64_t now_ms);
void   mr_app_pixel_size(MRApp* app, uint32_t* out_w, uint32_t* out_h);
// 拷出 RGBA8；len 不足时返回 0 且不写
size_t mr_app_pixels(MRApp* app, uint8_t* out, size_t len);

// 输入：必须送完整三段，不要在宿主侧自己合成"点击"
void   mr_app_pointer_down(MRApp* app, float x, float y);
void   mr_app_pointer_move(MRApp* app, float x, float y);
// 带事件时间的移动：t_ms 与 mr_app_pump 的 now_ms 同一个单调时钟（Android MotionEvent.getEventTime()、
// iOS UITouch.timestamp*1000）。页面拖动按它重采样，消掉慢拖时的一顿一顿；钟对不上时退回逐事件
void   mr_app_pointer_move_at(MRApp* app, float x, float y, double t_ms);
void   mr_app_pointer_up(MRApp* app, float x, float y);
void   mr_app_pointer_cancel(MRApp* app);
/* 第二根手指（0 按下 / 1 移动 / 2 抬起 / 其它 取消）：只有图片预览认它（双指缩放），页面仍是单指 */
void   mr_app_pointer2(MRApp* app, int phase, float x, float y);
// 每根指针一次的触摸（与 mr_game_touch 同形）：phase 0 下 / 1 移 / 2 抬 / 3 取消，id 宿主指针 id，坐标逻辑像素，
// t_ms 事件时刻（与 mr_app_pump 同一个钟，<=0 = 不知道）。第一根映射到 pointer_*、第二根到 pointer2、第三根起丢掉。
// 与上面的 pointer_* 二选一，不要混着调
void   mr_app_touch(MRApp* app, int phase, int id, float x, float y, float force, double t_ms);
/* 手机朝向（0 竖 / 90 左侧朝下 / -90 右侧朝下）：全屏视频没指定 direction 时跟着转 */
void   mr_app_set_device_orientation(MRApp* app, int deg);
void   mr_app_wheel(MRApp* app, float delta_y, int precise);
void   mr_app_text_input(MRApp* app, const char* utf8);
// enter / newline / backspace / delete / left / right / up / down / home / end / shift+<方向|home|end> /
// word-left / word-right / doc-start / doc-end / delete-word-back / selectall / copy / cut / undo / redo / escape
// 回车也可以作为 mr_app_text_input("\n") 送：textarea 换行（confirm-type=return 时），input 触发 confirm
void   mr_app_key(MRApp* app, const char* name);

// 返回 0 表示页面栈只剩一页，宿主该关掉小程序
int    mr_app_back(MRApp* app);
uint32_t mr_app_page_depth(MRApp* app);

// 键盘顶起画面（输入框 adjust-position）：导航栏以下整体上移 logical 逻辑像素（0 = 收回），引擎自己缓动、换算触摸坐标
void   mr_app_set_keyboard_shift(MRApp* app, float logical);
void   mr_app_on_show(MRApp* app);   // 进前台
void   mr_app_on_hide(MRApp* app);   // 进后台（之后应停止 pump）
void   mr_app_trim_memory(MRApp* app); // 系统内存告警时调用

const char* mr_app_last_error(MRApp* app); // 无错误时返回 NULL
const char* mr_version(void);


// ============ 渲染后端（CPU 参考实现 + GPU 可选） ============
//
// 只增不改：上面的 mr_app_* 签名与语义一个都没变（mr_app_pump 仍只返回 0 / 1）。
// 这些符号**有没有 gpu feature 都导出**；不调它们的宿主永远是 CPU，与以前逐字节一致。
//
// 现状：没编进 gpu feature 时 attach 返回 1、原因串含「未编译」。编进来了（桌面默认、
// build-mobile.sh 的 Android 与 iOS 默认）：偏好 auto / gpu 且门槛与启动自检都过了，
// attach 返回 2，之后由引擎直接画到 surface 上（pump_ex 带 MR_FRAME_PRESENTED）。偏好 cpu
// （默认）时根本不碰 surface。运行中设备丢失 / surface 丢失当帧回退 CPU，带
// MR_FRAME_BACKEND_CHANGED，且本实例之后不再试 GPU。
// CPU 下合成那四位（NEW / PRESENTED / BACKEND_CHANGED / PENDING）只会出现 0 或 MR_FRAME_NEW；
// 另有两位与后端无关的提示位 MR_FRAME_WANTS_MORE / MR_FRAME_EVENTS（见下面「嵌入宿主」）。
// GPU 上屏的帧也能取像素（mr_app_pixels* 会补一次同图层的 CPU 合成，与屏上逐字节相同），
// 只是要多花一次 CPU 合成的时间 —— 每帧都取就失去了 GPU 的意义。
//
// 挂了 surface 之后的用法：
//   int f = mr_app_pump_ex(app, now_ms);
//   if (f & MR_FRAME_BACKEND_CHANGED) { /* 按 mr_app_backend(app) 切换显示路径 */ }
//   if ((f & MR_FRAME_NEW) && !(f & MR_FRAME_PRESENTED)) mr_app_pixels(app, buf, len);

// 后端偏好 / 实际后端
#define MR_BACKEND_AUTO 0
#define MR_BACKEND_CPU  1
#define MR_BACKEND_GPU  2
// surface 种类
#define MR_SURFACE_METAL_LAYER     1   /* CAMetalLayer*（iOS / macOS） */
#define MR_SURFACE_ANDROID_WINDOW  2   /* ANativeWindow* */
// mr_app_pump_ex 的返回位
#define MR_FRAME_NEW              1    /* 有新画面 */
#define MR_FRAME_PRESENTED        2    /* 已由引擎画到挂载的 surface 上，宿主不要再取像素显示 */
#define MR_FRAME_BACKEND_CHANGED  4    /* 实际后端被引擎改了（回退 / set_backend 后重新选择），宿主要立刻切换显示路径；
                                          宿主自己 attach / detach 不置这一位 */
#define MR_FRAME_PENDING          8    /* 本次没上屏（surface 暂时取不到），下一次 pump_ex 会强制重新合成 */

// 设偏好。成功 1；pref 不在 0..=2 返回 0（写 last_error）。
// 环境变量 MINI_BACKEND 设了的话以它为准（诊断开关压过宿主配置）。
int  mr_app_set_backend(MRApp* app, int pref);
// 机型分级用，挂 surface 之前调用。0 表示未知；low_ram 非 0 即真。
void mr_app_set_device_info(MRApp* app, uint64_t total_mem_bytes, int low_ram, uint32_t os_api);
// **建引擎之前**调：字体与 GPU 合成器各开一条线程预热，与建引擎、开首页并行。参数与之后的
// mr_app_create_ex（逻辑宽高、dpr）、mr_app_set_backend、mr_app_set_device_info 一致（宽高 / dpr 不一致时
// 预热的那份认领不上，挂 surface 时照常现开）；pref 传 -1 表示不指定。返回是否开始预热 GPU
int  mr_warmup(uint32_t width, uint32_t height, float dpr, int pref,
               uint64_t total_mem_bytes, int low_ram, uint32_t os_api);
// surface 来之前先在后台线程上开 GPU（设好偏好与机型信息之后、mr_app_launch 之前调）：
// 认领 mr_warmup 那份，没有就现开。返回 1 = 开始预热；偏好 cpu / 门槛不过 / 没编 gpu 返回 0
int  mr_app_prewarm_gpu(MRApp* app);
// 挂 surface（不带宽高：引擎按 mr_app_pixel_size 配置，宿主把层 / SurfaceView 设成同样大）。
// 返回挂载后的实际后端：2 = GPU 生效；1 = 仍是 CPU（原因见 mr_app_backend_reason）；
// 0 = 参数错误（app / native 为空、kind 不认识或当前目标不支持：iOS 上传 2、Android 上传 1）。
// 所有权：CAMetalLayer* 由调用方持有，必须活到 detach / destroy 返回；
//         ANativeWindow* 由引擎在挂载时自己 acquire、卸载时 release。重复 attach 等于先 detach。
int  mr_app_attach_surface(MRApp* app, int kind, void* native);
// surface 将被销毁 / 进后台前调用；之后是 CPU
void mr_app_detach_surface(MRApp* app);
// 实际后端：1 / 2；app 为空返回 0
int  mr_app_backend(MRApp* app);
// UTF-8；到下一次对同一 app 调用任何 mr_app_* 之前有效；app 为空返回 NULL
const char* mr_app_backend_reason(MRApp* app);
// 跑一帧，返回 MR_FRAME_* 的按位或。
// mr_app_pump(app, t) == ((mr_app_pump_ex(app, t) & MR_FRAME_NEW) ? 1 : 0)
int  mr_app_pump_ex(MRApp* app, uint64_t now_ms);

// ─────────────────────── 嵌入宿主（Flutter 插件 / 原生 View）───────────────────────
//
// 只增不改：上面的函数签名与语义都没变；不调这些新函数的宿主行为与从前一致。
// 线程约定同上（同一条线程）。
//
// mr_app_pump_ex 的两位提示位（与后端无关）：
//   MR_FRAME_WANTS_MORE  引擎还有活要干（动画 / 惯性 / 定时器 / 请求在飞 / 按住的手指 …），
//                        下一个 vsync 再 pump；没有这一位时可以停掉 vsync，等输入事件再 pump
//   MR_FRAME_EVENTS      事件队列非空，mr_app_poll_event 取到 NULL 为止
#define MR_FRAME_WANTS_MORE 16
#define MR_FRAME_EVENTS     32

// 带选项创建。options_json 可为 NULL（= mr_app_create）。格式错 / 失败返回 NULL。
//   {"systemInfo":{"platform":"ios","model":"…"},          // 合并进 wx.getSystemInfoSync
//    "safeArea":{"top":47,"right":0,"bottom":34,"left":0},  // 逻辑像素；env(safe-area-inset-*) 取它
//    "hostApis":["chooseImage","getLocation"],              // 宿主实现的 wx.*（经 hostCall 事件）
//    "nativeComponents":["video","map","web-view"],         // 宿主用原生控件承载，引擎只报位置与遮挡
//    "menuButton":true}                                     // custom 导航栏的页面由引擎画右上角胶囊（默认关）
MRApp* mr_app_create_ex(const char* app_dir, const char* data_dir,
                        uint32_t width, uint32_t height, float dpr,
                        const char* options_json);

// 改尺寸（逻辑像素）与像素比。变了返回 1，并已出好新尺寸的一帧；没变返回 0。
int mr_app_resize(MRApp* app, uint32_t width, uint32_t height, float dpr);
// 安全区（逻辑像素）。变了返回 1：系统信息、env(safe-area-inset-*) 随之更新。
int mr_app_set_safe_area(MRApp* app, float top, float right, float bottom, float left);

// 把当前帧写进宿主缓冲：stride = 每行字节数（≥ 宽×4），format = MR_PIXEL_*。
// 返回写到的最远字节（stride×(高−1) + 宽×4）；参数不对返回 0 且不写。
// iOS / macOS 的 CVPixelBuffer（32BGRA）用 BGRA，Android 的 Bitmap / ANativeWindow 用 RGBA。
#define MR_PIXEL_RGBA8 0
#define MR_PIXEL_BGRA8 1
size_t mr_app_pixels_into(MRApp* app, uint8_t* dst, size_t len, uint32_t stride, int format);

// 分栏（微信 frameset：阔折叠展开 / PC 窗口拉宽）。选项 "frameset":"auto"（或 app.json 的
// resizable + frameset）且 mr_app_resize 给的宽 ≥ "framesetMinWidth"（默认 600）时，引擎按一栏
// （宽的一半）排页面，并发 frameset 事件 {on, livePane:"left"|"right", paneWidth, totalWidth}：
// mr_app_pixels_into 的主画面放在 livePane 那一栏（坐标照旧相对这一栏），另一栏放
// mr_app_side_pixels_into 的图（MR_FRAME_SIDE 表示它换了）。栈上一页时活页面在左、右栏是占位；
// 两页以上栈顶在右、下面那页在左。另一栏的滚轮交给 mr_app_side_wheel。
// 两栏都能点：按下落在另一栏（事件 sidePage 为 true）时先 mr_app_frameset_focus(app, 0 左 / 1 右)，
// 返回 1 表示那一栏换成了活页面（livePane 随之变），指针坐标改按这一栏算；左栏 navigateTo 时右栏那页关掉、新页进右栏。
#define MR_FRAME_SIDE 64
int    mr_app_frameset_focus(MRApp* app, int pane);
size_t mr_app_side_pixels_into(MRApp* app, uint8_t* dst, size_t len, uint32_t stride, int format);
void   mr_app_side_pixel_size(MRApp* app, uint32_t* out_w, uint32_t* out_h);
void   mr_app_side_wheel(MRApp* app, float delta_y, int precise);

// 引擎 → 宿主的事件（UTF-8 JSON），没有了返回 NULL。指针活到下一次对同一 app 调用 mr_app_*。
// type：page / navigationBar / inputFocus / inputBlur / nativeViews / hostCall / clipboard /
//       vibrate / app（启动后一次：about 关于信息）/
//       menuButton（点了胶囊：action = "menu" 查看关于 / "close" 返回宿主）/ log
// 字段见 doc/原生App集成指南.md「事件队列」；坐标都是视口逻辑坐标。
const char* mr_app_poll_event(MRApp* app);

// 回 hostCall：ok 非 0 走 success、否则 fail；result_json 是回调拿到的 res（NULL = {}）。
// id 不认识（重复回 / 页面已换）返回 0。
int mr_app_host_reply(MRApp* app, uint32_t id, int ok, const char* result_json);

// 宿主承载的原生组件发生了事件：key 取自 nativeViews 事件，name 如 "ended" / "timeupdate"，
// 派发给节点上的 bind<name>；detail_json 是事件 detail（NULL = {}）。找到绑定返回 1。
int mr_app_native_view_event(MRApp* app, const char* key, const char* name, const char* detail_json);

// 宿主推事件给逻辑层的 wx.onXxx 监听者：name 如 "accelerometerChange" / "locationChange" /
// "bluetoothDeviceFound" / "networkStatusChange" / "themeChange" / "contextEvent"（"onXxx" 写法也认），
// payload_json 是交给回调的 res（NULL = {}）。返回收到它的监听者个数（0 = 没人在听）。
int mr_app_dispatch_event(MRApp* app, const char* name, const char* payload_json);

// 同层渲染的 <video>（创建选项 "sameLayer": ["video"]）：宿主解出的一帧交给引擎画进页面。
// key 取自 videos 事件；任何线程都可以调（不碰引擎实例），调完让引擎线程出一帧。收下返回 1。
// 播放器状态照旧用 mr_app_native_view_event 回传（play / pause / timeupdate / ended …），
// 引擎发来的 videoCommand 事件（play / pause / stop / seek / playbackRate）由宿主执行。
int mr_video_frame_rgba(const char* key, uint32_t width, uint32_t height, uint32_t stride, const uint8_t* data);
// YUV 4:2:0 三个平面（Android ImageReader 的 YUV_420_888：U / V 交错时 uv_pixel_stride 为 2）
int mr_video_frame_yuv(const char* key, uint32_t width, uint32_t height,
                       const uint8_t* y, uint32_t y_stride,
                       const uint8_t* u, const uint8_t* v, uint32_t uv_stride, uint32_t uv_pixel_stride);

// 同层渲染的通用组件（"sameLayer" 里除 video 以外的标签，如 "map"）：宿主把控件画好的一帧交给引擎画进页面。
// 引擎发 sameLayerViews 事件（views[]：key / tag / attrs / width / height / box[设备像素宽, 高] / fixed），
// 宿主按 box 画（离屏），内容变了才交 —— 滚动不用交，画面在引擎画布里跟着页面走（盖上去的控件做不到这一点）。
// 点在框里发 nativeViewTap 事件（key / tag / x / y，视图本地逻辑坐标），宿主判点到了什么、照旧
// mr_app_native_view_event 回传（如 markertap）。任何线程都可以调，调完让引擎线程出一帧。收下返回 1
int mr_native_view_frame_rgba(const char* key, uint32_t width, uint32_t height, uint32_t stride, const uint8_t* data);

// 在逻辑层执行一段 JS，返回结果的字符串形式；抛异常返回 NULL（原因见 mr_app_last_error）。
// 指针活到下一次对同一 app 调用 mr_app_*。注入深链参数、集成测试断言页面数据用。
const char* mr_app_eval(MRApp* app, const char* code);

// ───────────── 小游戏（src/ffi_game.rs）─────────────
// 与 MRApp 平级的独立句柄。所有 mr_game_* 必须在同一条线程上调（与 mr_app_* 同一约定）。
// 每个入口都有 panic 防护：引擎内部出错时实例进入「失效」状态，之后的调用返回失败值，
// 事件队列里留一条 {"type":"error","kind":"panic"}，宿主应销毁重建（mr_game_is_poisoned 查询）。
typedef struct MRGame MRGame;
// 包种类：0 = 认不出（没有 / 同时有 app.json 与 game.json），1 = 小程序，2 = 小游戏
int mr_package_kind(const char* dir);
// 本库编进了哪些能力（JSON：game / webgl / audio / wasm / gpu / version / sdkVersion）。静态字符串，不用释放
const char* mr_capabilities(void);
// 建实例并执行 game.js。options_json 可为 NULL：{"appId","systemInfo","launchOptions","watchdogMs","jsMemoryLimitMB"}。
// appId 决定数据命名空间（<data_dir>/game/<appId>/），请务必给。不是小游戏包 / game.json 坏了返回 NULL
MRGame* mr_game_create(const char* app_dir, const char* data_dir, uint32_t width, uint32_t height, float dpr, const char* options_json);
void mr_game_destroy(MRGame* game);
// 跑一帧，返回 MR_FRAME_* 位（与 mr_app_pump_ex 同一套，另加 MR_FRAME_WAKE_AT = 128：只剩远期定时器，
// 按 mr_game_next_wake_us 定时唤醒即可）。now_us 是宿主 vsync 时刻，单位**微秒**，0 = 引擎自己取
int mr_game_pump_ex(MRGame* game, uint64_t now_us);
// 取一条事件（JSON：log / error），没有了返回 NULL。指针活到下一次对同一实例调用 mr_game_*
const char* mr_game_poll_event(MRGame* game);
// 执行一段 JS（诊断用）。抛异常返回 NULL（原因见 mr_game_last_error）
const char* mr_game_eval(MRGame* game, const char* code);
const char* mr_game_last_error(MRGame* game);
// 1 = 实例已因内部错误失效（或句柄为空）
int mr_game_is_poisoned(MRGame* game);
// 前后台：非 0 = 进后台（不跑 rAF、不要帧），0 = 回前台（不补帧）
void mr_game_set_background(MRGame* game, int background);
// 系统音频打断：非 0 = 开始（来电、闹钟、别的 App 拿走独占焦点），0 = 结束。引擎暂停全部音频后派发
// wx.onAudioInterruptionBegin / End，结束时不自动续播。wx.setInnerAudioOption 的结果以 {"type":"audioSession",…} 事件交给宿主
void mr_game_audio_interruption(MRGame* game, int begin);
// 窗口变了（转屏、分屏、折叠屏）：逻辑尺寸、像素比、方向（"portrait" / "landscape" / "landscapeReverse"，NULL = 按宽高推）。
// 引擎派发 wx.onWindowResize / onDeviceOrientationChange；主画布尺寸不变（游戏自己改）。
// 建实例后会有一条 {"type":"deviceOrientation","value":…,"initial":true} 事件：宿主按它锁方向；wx.setDeviceOrientation 发同样的事件
void mr_game_resize(MRGame* game, uint32_t width, uint32_t height, float dpr, const char* orientation);
// 系统内存告警：level 5 / 10 / 15 派发 wx.onMemoryWarning（Android 前台三档），其它值只回收（图片缓存、JS GC）
void mr_game_memory_warning(MRGame* game, int level);
// 宿主推事件给 wx.onXxx：键盘 keyboardInput / keyboardConfirm / keyboardComplete {"value"}、keyboardHeightChange {"height"}、
// 传感器（accelerometerChange …）、网络等，名字与 mr_app_dispatch_event 相同。返回收到它的监听者个数。
// 键盘相关的宿主事件：{"type":"showKeyboard",defaultValue,maxLength,multiple,confirmHold,confirmType} / updateKeyboard {value} / hideKeyboard
int mr_game_dispatch_event(MRGame* game, const char* name, const char* payload_json);
// 创建选项 hostApis 点名的 wx.* / 对象方法（login、BannerAd.show …）调用时发 {"type":"hostCall","id","api","args"}，宿主用它回结果
int mr_game_host_reply(MRGame* game, uint32_t id, int ok, const char* result_json);
// rAF 目标帧率（1~60）：宿主据此请求显示刷新率（60 的整数倍）
int mr_game_preferred_fps(MRGame* game);
// pump_ex 返回 MR_FRAME_WAKE_AT（128）时：多久之后再 pump（微秒），-1 = 没有定时器
int64_t mr_game_next_wake_us(MRGame* game);
// 主画布像素尺寸：宿主把 surface 缓冲设成这个尺寸，由系统缩放到全屏（不要在 CPU 上缩放）
void mr_game_frame_size(MRGame* game, uint32_t* width, uint32_t* height);
// 当前帧（不透明）写进 buf：stride 字节一行，format 0 = RGBA8、1 = BGRA8。返回写入的行数，0 = 没写
int mr_game_pixels_into(MRGame* game, uint8_t* buf, size_t len, size_t stride, int format);
// 一个指针事件，宿主每个指针调一次：phase 0 下 / 1 移 / 2 抬 / 3 取消；id 宿主指针 id；x, y 视图内逻辑像素；
// force 0~1（没有给 0）；t_us 事件时刻（微秒，与 mr_game_pump_ex 同一时钟，0 = 此刻）。只排队，下一次 pump 在 rAF 之前派发
void mr_game_touch(MRGame* game, int phase, int id, float x, float y, float force, uint64_t t_us);
// PC 鼠标：phase 0 下 / 1 移 / 2 抬；button 0 左 1 中 2 右
void mr_game_mouse(MRGame* game, int phase, int button, float x, float y, uint64_t t_us);
// PC 滚轮：deltaX / deltaY / deltaZ（像素）
void mr_game_wheel(MRGame* game, float dx, float dy, float dz, float x, float y, uint64_t t_us);
// 键盘：down 非 0 = 按下；key 如 "a" / "Enter"，code 如 "KeyA"
void mr_game_key(MRGame* game, int down, const char* key, const char* code, uint64_t t_us);

#ifdef __cplusplus
}
#endif

#endif // MINI_RENDER_H
