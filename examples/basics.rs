//! 分组示例：cargo run --example basics
#![allow(unused)]
include!("gallery_parts/common.rs");
include!("gallery_parts/basic.rs");

fn main() {
    std::fs::create_dir_all("doc/gallery").ok();
    login(); product_list(); product_detail(); cart(); profile(); settings();
}
