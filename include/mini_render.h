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

#ifdef __cplusplus
}
#endif

#endif // MINI_RENDER_H
