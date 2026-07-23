//! 分组示例：cargo run --example navigation
#![allow(unused)]
include!("gallery_parts/common.rs");
include!("gallery_parts/nav.rs");

fn main() {
    std::fs::create_dir_all("doc/gallery").ok();
    tabbar(); search_nav(); input_events();
}
