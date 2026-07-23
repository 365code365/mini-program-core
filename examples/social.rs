//! 分组示例：cargo run --example social
#![allow(unused)]
include!("gallery_parts/common.rs");
include!("gallery_parts/social.rs");

fn main() {
    std::fs::create_dir_all("doc/gallery").ok();
    chat(); feed();
}
