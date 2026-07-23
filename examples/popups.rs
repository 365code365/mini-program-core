//! 分组示例：cargo run --example popups
#![allow(unused)]
include!("gallery_parts/common.rs");
include!("gallery_parts/overlay.rs");

fn main() {
    std::fs::create_dir_all("doc/gallery").ok();
    modal_dialog(); action_sheet(); toast();
}
