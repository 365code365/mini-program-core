//! 内存封顶与回收的测试。
//!
//! 引擎里所有「越用越大」的表都必须有上限，否则长时间使用/多页浏览之后内存只涨不降，
//! 在手机上就是被系统杀掉。
//!
//! **这里刻意不调用 `trim_memory()` / `clear_image_caches()`**：图片、动图、字体
//! 缓存都是**进程级**的，而 `cargo test` 在同一个进程里并发跑所有测试 ——
//! 在这里清一次缓存，会把正在依赖缓存的别人打挂（实测会让
//! `gif_tests::test_gif_frame_advances_over_time` 因为动图起播时间被重置而失败，
//! 以及 `image_net` 的磁盘缓存往返测试莫名其妙地红）。
//!
//! 所以分工是：
//! - 逐出策略与字节记账 → `renderer::components::image_cache` 的单测（纯数据结构，无全局状态）；
//! - 上限是否存在、报告是否可读 → 本文件（只读断言）；
//! - `trim_memory()` 的实际效果 → `cargo run --release --example mem_report`（独立进程）。

#[test]
fn image_cache_report_is_readable_and_states_the_budget() {
    let r = crate::renderer::components::image_cache_report();
    assert!(r.contains("预算"), "报告应写明预算，实际: {r}");
    assert!(r.contains("静态图") && r.contains("动图"), "两张表都要报: {r}");
    assert!(r.contains("逐出"), "要能看到逐出次数（判断预算是否偏小）: {r}");
}

#[test]
fn memory_report_covers_images_and_custom_fonts() {
    let r = crate::memory_report();
    assert!(r.contains("静态图"), "实际: {r}");
    assert!(r.contains("自定义字体"), "实际: {r}");
}

#[test]
fn image_cache_budget_is_sane() {
    // 预算太小会导致「每帧重新解码」，太大等于没有上限
    let mb = crate::renderer::components::image_cache_budget_mb();
    assert!(mb >= 8, "预算至少 8MB，实际 {mb}MB");
    assert!(mb <= 512, "预算不该无上限，实际 {mb}MB");
}

#[test]
fn custom_font_files_have_an_upper_bound() {
    // 逐个请求几种字族（本机不一定都有），已加载的自定义字体文件数不得无限增长。
    // 一个 CJK 字面在 fontdue 里要 150~300MB，所以这个上限是内存的硬约束。
    for stack in [
        "\"Songti SC\", serif",
        "\"Times New Roman\", serif",
        "Georgia, serif",
        "Menlo, monospace",
        "\"Courier New\", monospace",
    ] {
        let _ = crate::text_family::renderer_for_family(Some(stack));
    }
    let files = crate::text_family::loaded_font_files();
    assert!(
        files.len() <= 8,
        "自定义字体文件数应有上限（默认 2，可用 MINI_FONT_CACHE_MAX 调），实际 {} 个: {:?}",
        files.len(),
        files
    );
}

#[test]
fn system_font_is_a_process_wide_singleton() {
    // 每个渲染器各自加载一份系统字体的话，每份 300MB —— 这条必须钉住
    let a = crate::text::shared_fonts().expect("系统字体");
    let b = crate::text::shared_fonts().expect("系统字体");
    assert!(
        std::sync::Arc::ptr_eq(&a, &b),
        "shared_fonts() 必须返回同一份实例"
    );
}
