//! 分组示例：cargo run --example widgets
#![allow(unused)]
include!("gallery_parts/common.rs");
include!("gallery_parts/widgets.rs");

fn main() {
    std::fs::create_dir_all("doc/gallery").ok();
    grid_menu(); form(); dashboard(); gallery_grid();
}
