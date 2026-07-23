//! 分组示例：cargo run --example canvas
#![allow(unused)]
include!("gallery_parts/common.rs");
include!("gallery_parts/canvas_video.rs");

fn main() {
    std::fs::create_dir_all("doc/gallery").ok();
    canvas_demo();
}
