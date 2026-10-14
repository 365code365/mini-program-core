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
void   mr_app_pointer_up(MRApp* app, float x, float y);
void   mr_app_pointer_cancel(MRApp* app);
void   mr_app_wheel(MRApp* app, float delta_y, int precise);
void   mr_app_text_input(MRApp* app, const char* utf8);
// enter / backspace / delete / left / right / home / end / selectall / escape
void   mr_app_key(MRApp* app, const char* name);

// 返回 0 表示页面栈只剩一页，宿主该关掉小程序
int    mr_app_back(MRApp* app);
uint32_t mr_app_page_depth(MRApp* app);

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
// 现状：GPU 后端还在开发中。没编进 gpu feature 时 attach 返回 1、原因串含「未编译」；
// 编进来了（目前是空壳）也只能是 CPU，原因串说明「GPU 合成器还在开发中」。
// mr_app_pump_ex 目前只会返回 0 或 MR_FRAME_NEW。
//
// 挂了 surface 之后的用法（GPU 落地后生效）：
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

#ifdef __cplusplus
}
#endif

#endif // MINI_RENDER_H
