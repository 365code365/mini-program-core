//! 分组示例：cargo run --example storefront
#![allow(unused)]
include!("gallery_parts/common.rs");
include!("gallery_parts/ecom_home.rs");

fn main() {
    std::fs::create_dir_all("doc/gallery").ok();
    orders(); home(); contacts();
}
