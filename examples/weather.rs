//! 分组示例：cargo run --example weather
#![allow(unused)]
include!("gallery_parts/common.rs");
include!("gallery_parts/weather.rs");

fn main() {
    std::fs::create_dir_all("doc/gallery").ok();
    weather();
}
