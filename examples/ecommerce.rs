//! 分组示例：cargo run --example ecommerce
#![allow(unused)]
include!("gallery_parts/common.rs");
include!("gallery_parts/promo.rs");

fn main() {
    std::fs::create_dir_all("doc/gallery").ok();
    ecommerce_home(); coupon_popup();
}
