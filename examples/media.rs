//! 分组示例：cargo run --example media
#![allow(unused)]
include!("gallery_parts/common.rs");
include!("gallery_parts/media.rs");

fn main() {
    std::fs::create_dir_all("doc/gallery").ok();
    music(); tags();
}
