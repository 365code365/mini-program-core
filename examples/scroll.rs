//! 分组示例：cargo run --example scroll
#![allow(unused)]
include!("gallery_parts/common.rs");
include!("gallery_parts/scroll.rs");

fn main() {
    std::fs::create_dir_all("doc/gallery").ok();
    swiper_banner(); h_scroll(); v_scroll(); bottom_picker(); calendar(); rating_steps();
}
