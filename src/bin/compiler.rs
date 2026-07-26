//! 小程序 → 多端源码编译器 CLI
//!
//! 统一入口：加载小程序源码为 `AppSource`，选择一个 [`CompileTarget`] 编译为产物文件，
//! 写入输出目录。新增目标端（Android / iOS ...）只需在 `mini_render::compiler` 下实现
//! `CompileTarget`，并在此处 `select_target` 注册。
//!
//! 运行：cargo run --bin mini-compiler [小程序根目录] [输出目录] [目标]
//!   默认：sample-app  dist-html  html

use mini_render::compiler::{load_app_source, write_files, CompileTarget};
use mini_render::compiler::html::HtmlTarget;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let app_arg = args.get(1).cloned().unwrap_or_else(|| "sample-app".to_string());
    // 示例小程序都在 sample/ 下，但命令行习惯写裸名字，统一交给解析器
    let app_root = mini_render::app_dir::resolve(&app_arg).to_string_lossy().to_string();
    let out_dir = args.get(2).cloned().unwrap_or_else(|| "dist-html".to_string());
    let target_name = args.get(3).cloned().unwrap_or_else(|| "html".to_string());

    let target = select_target(&target_name)?;

    println!("📦 编译小程序 -> {} 工程", target.name());
    println!("   源: {}\n   目标: {}", app_root, out_dir);

    let app = load_app_source(&app_root)?;
    let page_count = app.pages.len();
    let files = target.compile(&app)?;
    let file_count = files.len();
    write_files(&out_dir, &files)?;

    for page in &app.pages {
        println!("  ✓ {}", page.route);
    }
    println!("\n✅ 完成：{} 个页面 · {} 个产物文件", page_count, file_count);
    println!("   工程目录: {}/", out_dir);
    println!("   启动页:   {}/index.html", out_dir);
    Ok(())
}

/// 按名字选择编译目标（扩展点：未来在此注册 android / ios 等）
fn select_target(name: &str) -> Result<Box<dyn CompileTarget>, String> {
    match name {
        "html" => Ok(Box::new(HtmlTarget::new())),
        other => Err(format!(
            "未知编译目标 '{}'（当前支持: html）",
            other
        )),
    }
}
