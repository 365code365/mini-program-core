//! 场景画廊：一次性渲染全部示例场景到 doc/gallery/。
//! 运行： cargo run --example gallery
//!
//! 各场景代码按主题拆分在 examples/gallery_parts/ 下，通过 include! 组合；
//! 也可单独运行分组示例，见 examples/README.md。
#![allow(dead_code)]

// 共享：use / 常量 / render / render_screen
include!("gallery_parts/common.rs");
// 各主题场景
include!("gallery_parts/basic.rs");
include!("gallery_parts/social.rs");
include!("gallery_parts/widgets.rs");
include!("gallery_parts/weather.rs");
include!("gallery_parts/ecom_home.rs");
include!("gallery_parts/media.rs");
include!("gallery_parts/overlay.rs");
include!("gallery_parts/scroll.rs");
include!("gallery_parts/promo.rs");
include!("gallery_parts/nav.rs");
include!("gallery_parts/canvas_video.rs");
include!("gallery_parts/news.rs");
include!("gallery_parts/business.rs");

fn main() {
    std::fs::create_dir_all("doc/gallery").ok();
    println!("渲染画廊 ->");

    // 电商 · 社交 · 导航
    login();
    product_list();
    product_detail();
    cart();
    profile();
    settings();
    chat();
    feed();
    grid_menu();
    // 表单 · 数据 · 生活
    form();
    dashboard();
    gallery_grid();
    weather(); // 4 个城市天气
    orders();
    home();
    contacts();
    music();
    tags();
    // 弹窗 · 滑动 · 交互
    modal_dialog();
    action_sheet();
    toast();
    swiper_banner();
    h_scroll();
    v_scroll();
    bottom_picker();
    calendar();
    rating_steps();
    // 电商大促
    ecommerce_home();
    coupon_popup();
    // 组件补充
    tabbar();
    search_nav();
    input_events();
    video_player();
    canvas_demo();
    // 动图：连续 4 张快照展示 GIF 逐帧推进
    gif_animation();
    // 资讯类（头条新闻同款设计）
    news_feed();
    news_article();
    news_video_list();
    news_mine();
    // 更多商业级场景
    logistics_track();
    live_shopping();
    food_order();
    member_center();
    payment_confirm();
    review_list();
    search_result();
    message_center();
    checkin_calendar();
    market_board();
    hotel_booking();
    health_dashboard();

    println!("完成，共 57 张场景图（含 4 个城市天气、4 帧 GIF 动画快照、4 张资讯页、12 张商业场景）。");
}
