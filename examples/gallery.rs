//! 渲染画廊：把多个不同场景的小程序页面渲染成 PNG，输出到 doc/gallery/
//! 用于 README 九宫格展示。运行： cargo run --example gallery

use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::WxmlRenderer;
use mini_render::{Canvas, Color};
use serde_json::{json, Value};

const W: u32 = 375;
const H: u32 = 667;
const SCALE: f32 = 2.0;

fn render(name: &str, bg: u32, wxml: &str, wxss: &str, data: Value) {
    let nodes = WxmlParser::new(wxml).parse().expect("wxml");
    let ss = WxssParser::new(wxss).parse().expect("wxss");
    let r = WxmlRenderer::new_with_scale(ss, W as f32, H as f32, SCALE);
    // 按内容高度自适应画布：保证截图完整、无裁切、无大片空白
    let content_h = r.measure_content_height(&nodes, &data).max(240.0);
    let mut r = r;
    let cw = (W as f32 * SCALE) as u32;
    let ch = (content_h * SCALE).ceil() as u32;
    let mut canvas = Canvas::new(cw, ch);
    canvas.clear(Color::from_hex(bg));
    r.render(&mut canvas, &nodes, &data);
    let path = format!("doc/gallery/{}.png", name);
    canvas.save_png(&path).expect("save");
    println!("  ✓ {} ({}x{})", path, cw, ch);
}

fn main() {
    std::fs::create_dir_all("doc/gallery").ok();
    println!("渲染画廊 ->");

    login();
    product_list();
    product_detail();
    cart();
    profile();
    settings();
    chat();
    feed();
    grid_menu();
    form();
    dashboard();
    gallery_grid();
    weather();
    orders();
    home();
    contacts();
    music();
    tags();
    // 复杂交互场景
    modal_dialog();
    action_sheet();
    toast();
    swiper_banner();
    h_scroll();
    v_scroll();
    bottom_picker();
    calendar();
    rating_steps();
    // 电商复杂场景
    ecommerce_home();
    coupon_popup();

    println!("完成，共 29 个场景。");
}

/// 固定手机屏幕高度渲染（内容超出即裁切，用于弹窗遮罩、上下滑动等）
fn render_screen(name: &str, bg: u32, wxml: &str, wxss: &str, data: Value) {
    let nodes = WxmlParser::new(wxml).parse().expect("wxml");
    let ss = WxssParser::new(wxss).parse().expect("wxss");
    let mut r = WxmlRenderer::new_with_scale(ss, W as f32, H as f32, SCALE);
    let cw = (W as f32 * SCALE) as u32;
    let ch = (H as f32 * SCALE) as u32;
    let mut canvas = Canvas::new(cw, ch);
    canvas.clear(Color::from_hex(bg));
    r.render(&mut canvas, &nodes, &data);
    let path = format!("doc/gallery/{}.png", name);
    canvas.save_png(&path).expect("save");
    println!("  ✓ {} ({}x{})", path, cw, ch);
}

// 通用样式片段
fn common() -> &'static str {
    r#"
    .page{ padding:24rpx; }
    .card{ background-color:#ffffff; border-radius:20rpx; padding:28rpx; margin-bottom:20rpx; box-shadow:0 4rpx 16rpx rgba(0,0,0,0.06); }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .col{ display:flex; flex-direction:column; }
    .grow{ flex:1; }
    .title{ font-size:36rpx; color:#1a1a1a; font-weight:bold; }
    .sub{ font-size:26rpx; color:#999999; }
    .price{ font-size:34rpx; color:#ff5000; font-weight:bold; }
    .primary{ background-color:#07c160; color:#ffffff; border-radius:44rpx; padding:20rpx; text-align:center; font-size:30rpx; }
    .avatar{ width:88rpx; height:88rpx; border-radius:44rpx; background-color:#c7e0ff; }
    .thumb{ width:140rpx; height:140rpx; border-radius:16rpx; background-color:#e8eaf0; }
    .tag{ background-color:#fff0e8; color:#ff5000; font-size:22rpx; padding:6rpx; border-radius:8rpx; margin-right:12rpx; }
    "#
}

// 1. 登录
fn login() {
    let wxml = r#"
    <view class="page">
        <view class="logo"></view>
        <text class="h1">欢迎登录</text>
        <text class="tip">请输入账号密码</text>
        <view class="field"><input value="13800138000" placeholder="手机号" /></view>
        <view class="field"><input type="password" value="password" placeholder="密码" /></view>
        <view class="btn">登 录</view>
        <view class="links">
            <text class="link">忘记密码</text>
            <text class="link">注册账号</text>
        </view>
    </view>"#;
    let wxss = r#"
    .page{ padding:60rpx 48rpx; }
    .logo{ width:140rpx; height:140rpx; border-radius:32rpx; background-color:#07c160; margin:40rpx auto 40rpx auto; }
    .h1{ font-size:48rpx; font-weight:bold; color:#1a1a1a; text-align:center; }
    .tip{ font-size:26rpx; color:#999; text-align:center; margin-bottom:60rpx; }
    .field{ background-color:#f5f6f8; border-radius:16rpx; padding:26rpx; margin-bottom:28rpx; }
    .field input{ font-size:30rpx; color:#333; }
    .btn{ background-color:#07c160; color:#fff; text-align:center; font-size:32rpx; padding:26rpx; border-radius:44rpx; margin-top:20rpx; }
    .links{ display:flex; flex-direction:row; justify-content:space-between; margin-top:36rpx; }
    .link{ font-size:26rpx; color:#576b95; }
    "#;
    render("01_login", 0xFFFFFF, wxml, wxss, json!({}));
}

// 2. 商品列表
fn product_list() {
    let wxml = r#"
    <view class="page">
        <view class="card row" wx:for="{{items}}" wx:key="id">
            <image class="thumb" src="{{item.img}}" mode="aspectFill" />
            <view class="col grow info">
                <text class="title">{{item.name}}</text>
                <text class="sub">{{item.desc}}</text>
                <view class="row between bottom">
                    <text class="price">¥{{item.price}}</text>
                    <view class="buy">加入购物车</view>
                </view>
            </view>
        </view>
    </view>"#;
    let wxss = r#"
    .page{ padding:24rpx; }
    .card{ background-color:#fff; border-radius:20rpx; padding:24rpx; margin-bottom:20rpx; display:flex; flex-direction:row; }
    .thumb{ width:180rpx; height:180rpx; border-radius:16rpx; background-color:#e8eaf0; }
    .info{ margin-left:24rpx; display:flex; flex-direction:column; flex:1; }
    .title{ font-size:32rpx; color:#1a1a1a; font-weight:bold; }
    .sub{ font-size:24rpx; color:#999; margin-top:8rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .bottom{ margin-top:40rpx; }
    .price{ font-size:36rpx; color:#ff5000; font-weight:bold; }
    .buy{ background-color:#07c160; color:#fff; font-size:24rpx; padding:12rpx; border-radius:32rpx; }
    "#;
    let data = json!({"items":[
        {"id":1,"name":"无线蓝牙耳机","desc":"降噪 · 30h续航","price":"199","img":"doc/gallery/assets/p_earbuds.jpg"},
        {"id":2,"name":"智能运动手表","desc":"心率 · 血氧监测","price":"599","img":"doc/gallery/assets/p_watch.jpg"},
        {"id":3,"name":"潮流运动鞋","desc":"轻质 · 缓震回弹","price":"329","img":"doc/gallery/assets/p_sneakers.jpg"},
        {"id":4,"name":"时尚双肩包","desc":"大容量 · 防泼水","price":"259","img":"doc/gallery/assets/p_backpack.jpg"}
    ]});
    render("02_product_list", 0xF5F6F8, wxml, wxss, data);
}

// 3. 商品详情
fn product_detail() {
    let wxml = r#"
    <view class="wrap">
        <view class="hero"></view>
        <view class="body">
            <view class="row between">
                <text class="price">¥3999</text>
                <text class="old">¥4299</text>
            </view>
            <text class="name">全面屏旗舰手机 12+256GB 幻夜黑</text>
            <view class="tags">
                <text class="tag">顺丰包邮</text>
                <text class="tag">7天无理由</text>
                <text class="tag">正品保障</text>
            </view>
            <view class="spec">
                <text class="spec-t">已选：幻夜黑 / 12+256GB</text>
            </view>
            <view class="param">
                <view class="row between p"><text class="k">品牌</text><text class="v">MiniTech</text></view>
                <view class="row between p"><text class="k">屏幕</text><text class="v">6.7英寸 OLED</text></view>
                <view class="row between p"><text class="k">电池</text><text class="v">5000mAh</text></view>
            </view>
        </view>
        <view class="bar">
            <view class="icon"></view>
            <view class="icon"></view>
            <view class="cart">加入购物车</view>
            <view class="now">立即购买</view>
        </view>
    </view>"#;
    let wxss = r#"
    .hero{ width:750rpx; height:560rpx; background-color:#dfe6ef; }
    .body{ padding:28rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .price{ font-size:52rpx; color:#ff5000; font-weight:bold; }
    .old{ font-size:28rpx; color:#bbb; text-decoration:line-through; }
    .name{ font-size:34rpx; color:#1a1a1a; font-weight:bold; margin-top:12rpx; }
    .tags{ display:flex; flex-direction:row; margin-top:20rpx; }
    .tag{ background-color:#fff0e8; color:#ff5000; font-size:22rpx; padding:8rpx; border-radius:8rpx; margin-right:12rpx; }
    .spec{ background-color:#f7f8fa; border-radius:16rpx; padding:24rpx; margin-top:24rpx; }
    .spec-t{ font-size:28rpx; color:#333; }
    .param{ margin-top:24rpx; }
    .p{ padding:18rpx 0; }
    .k{ font-size:28rpx; color:#999; }
    .v{ font-size:28rpx; color:#333; }
    .bar{ display:flex; flex-direction:row; align-items:center; padding:16rpx 24rpx; }
    .icon{ width:72rpx; height:72rpx; border-radius:36rpx; background-color:#f0f0f0; margin-right:16rpx; }
    .cart{ flex:1; background-color:#ffb400; color:#fff; text-align:center; font-size:28rpx; padding:22rpx; border-radius:44rpx 0 0 44rpx; }
    .now{ flex:1; background-color:#ff5000; color:#fff; text-align:center; font-size:28rpx; padding:22rpx; border-radius:0 44rpx 44rpx 0; }
    "#;
    render("03_product_detail", 0xFFFFFF, wxml, wxss, json!({}));
}

// 4. 购物车
fn cart() {
    let wxml = r##"
    <view class="page">
        <text class="head">购物车（3）</text>
        <view class="card row" wx:for="{{items}}" wx:key="id">
            <icon class="ck" wx:if="{{item.checked}}" type="success" size="22" />
            <icon class="ck" wx:else type="circle" size="22" color="#cccccc" />
            <image class="thumb" src="{{item.img}}" mode="aspectFill" />
            <view class="col grow info">
                <text class="name">{{item.name}}</text>
                <text class="sku">{{item.sku}}</text>
                <view class="row between">
                    <text class="price">¥{{item.price}}</text>
                    <view class="stepper row">
                        <text class="minus">-</text>
                        <text class="qty">{{item.qty}}</text>
                        <text class="plus">+</text>
                    </view>
                </view>
            </view>
        </view>
        <view class="bar row between">
            <text class="total">合计：<text class="tp">¥927</text></text>
            <view class="checkout">结算(3)</view>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ padding:24rpx; }
    .head{ font-size:34rpx; font-weight:bold; color:#1a1a1a; margin-bottom:20rpx; }
    .card{ background-color:#fff; border-radius:20rpx; padding:24rpx; margin-bottom:20rpx; display:flex; flex-direction:row; align-items:center; }
    .ck{ margin-right:20rpx; }
    .thumb{ width:150rpx; height:150rpx; border-radius:16rpx; background-color:#e8eaf0; }
    .info{ margin-left:20rpx; flex:1; display:flex; flex-direction:column; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .name{ font-size:30rpx; color:#1a1a1a; }
    .sku{ font-size:24rpx; color:#999; margin:10rpx 0 30rpx 0; }
    .price{ font-size:34rpx; color:#ff5000; font-weight:bold; }
    .stepper{ display:flex; flex-direction:row; align-items:center; }
    .minus,.plus{ width:48rpx; height:48rpx; background-color:#f2f2f2; border-radius:8rpx; text-align:center; font-size:32rpx; color:#666; }
    .qty{ font-size:28rpx; color:#333; padding:0 24rpx; }
    .bar{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; background-color:#fff; border-radius:20rpx; padding:24rpx; margin-top:12rpx; }
    .total{ font-size:28rpx; color:#333; }
    .tp{ font-size:38rpx; color:#ff5000; font-weight:bold; }
    .checkout{ background-color:#ff5000; color:#fff; font-size:30rpx; padding:20rpx 48rpx; border-radius:44rpx; }
    "#;
    let data = json!({"items":[
        {"id":1,"name":"无线蓝牙耳机","sku":"白色 标准版","price":"199","qty":1,"checked":true,"img":"doc/gallery/assets/p_earbuds.jpg"},
        {"id":2,"name":"智能运动手表","sku":"黑色 46mm","price":"599","qty":1,"checked":true,"img":"doc/gallery/assets/p_watch.jpg"},
        {"id":3,"name":"时尚双肩包","sku":"深灰色","price":"259","qty":1,"checked":false,"img":"doc/gallery/assets/p_backpack.jpg"}
    ]});
    render("04_cart", 0xF5F6F8, wxml, wxss, data);
}

// 5. 个人中心
fn profile() {
    let wxml = r#"
    <view class="wrap">
        <view class="header">
            <view class="row">
                <image class="avatar" src="doc/gallery/assets/avatar1.jpg" mode="aspectFill" />
                <view class="col hi">
                    <text class="name">时光旅人</text>
                    <text class="id">ID: 88888888</text>
                </view>
                <view class="vip">VIP</view>
            </view>
            <view class="stats row">
                <view class="col stat"><text class="num">12</text><text class="lbl">收藏</text></view>
                <view class="col stat"><text class="num">36</text><text class="lbl">关注</text></view>
                <view class="col stat"><text class="num">128</text><text class="lbl">粉丝</text></view>
                <view class="col stat"><text class="num">6</text><text class="lbl">优惠券</text></view>
            </view>
        </view>
        <view class="menu">
            <view class="mi row between" wx:for="{{menu}}" wx:key="*this">
                <view class="row"><view class="mi-ico"></view><text class="mt">{{item}}</text></view>
                <text class="arrow">></text>
            </view>
        </view>
    </view>"#;
    let wxss = r#"
    .header{ background-color:#07c160; padding:60rpx 40rpx 40rpx 40rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .avatar{ width:120rpx; height:120rpx; border-radius:60rpx; background-color:#ffffff; }
    .hi{ margin-left:24rpx; flex:1; display:flex; flex-direction:column; }
    .name{ font-size:38rpx; color:#fff; font-weight:bold; }
    .id{ font-size:24rpx; color:#e6fff0; margin-top:8rpx; }
    .vip{ background-color:#ffd700; color:#7a5b00; font-size:24rpx; padding:8rpx 16rpx; border-radius:20rpx; }
    .stats{ display:flex; flex-direction:row; justify-content:space-between; margin-top:44rpx; }
    .stat{ display:flex; flex-direction:column; align-items:center; flex:1; }
    .num{ font-size:38rpx; color:#fff; font-weight:bold; }
    .lbl{ font-size:24rpx; color:#e6fff0; margin-top:6rpx; }
    .menu{ margin:24rpx; background-color:#fff; border-radius:20rpx; padding:8rpx 28rpx; }
    .mi{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; padding:30rpx 0; border-bottom:1rpx solid #f0f0f0; }
    .mi-ico{ width:48rpx; height:48rpx; border-radius:12rpx; background-color:#e8f7ee; margin-right:20rpx; }
    .mt{ font-size:30rpx; color:#333; }
    .arrow{ font-size:30rpx; color:#ccc; }
    "#;
    let data = json!({"menu":["我的订单","收货地址","账户安全","消息通知","帮助中心","关于我们"]});
    render("05_profile", 0xF5F6F8, wxml, wxss, data);
}

// 6. 设置
fn settings() {
    let wxml = r#"
    <view class="page">
        <text class="grp">通用</text>
        <view class="card">
            <view class="item row between"><text class="t">夜间模式</text><switch checked="true" /></view>
            <view class="item row between"><text class="t">消息推送</text><switch checked="true" /></view>
            <view class="item row between"><text class="t">自动播放</text><switch /></view>
            <view class="item row between"><text class="t">流量提醒</text><switch checked="true" /></view>
        </view>
        <text class="grp">隐私</text>
        <view class="card">
            <view class="item row between"><text class="t">个性化推荐</text><switch checked="true" /></view>
            <view class="item row between"><text class="t">位置权限</text><text class="v">仅使用时 ></text></view>
            <view class="item row between"><text class="t">清除缓存</text><text class="v">128MB ></text></view>
        </view>
        <view class="logout">退出登录</view>
    </view>"#;
    let wxss = r#"
    .page{ padding:24rpx; }
    .grp{ font-size:26rpx; color:#999; margin:20rpx 0 12rpx 12rpx; }
    .card{ background-color:#fff; border-radius:20rpx; padding:0 28rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .item{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; padding:30rpx 0; border-bottom:1rpx solid #f2f2f2; }
    .t{ font-size:30rpx; color:#333; }
    .v{ font-size:28rpx; color:#999; }
    .logout{ background-color:#fff; color:#ff3b30; text-align:center; font-size:30rpx; padding:28rpx; border-radius:20rpx; margin-top:40rpx; }
    "#;
    render("06_settings", 0xF5F6F8, wxml, wxss, json!({}));
}

// 7. 聊天
fn chat() {
    let wxml = r#"
    <view class="page">
        <view class="msg row {{item.me ? 'me' : ''}}" wx:for="{{msgs}}" wx:key="id">
            <view class="av" wx:if="{{!item.me}}"></view>
            <view class="bubble {{item.me ? 'b-me' : ''}}"><text class="tx {{item.me ? 'tx-me' : ''}}">{{item.text}}</text></view>
            <view class="av" wx:if="{{item.me}}"></view>
        </view>
    </view>"#;
    let wxss = r#"
    .page{ padding:24rpx; }
    .msg{ display:flex; flex-direction:row; align-items:flex-start; margin-bottom:30rpx; }
    .msg.me{ justify-content:flex-end; }
    .av{ width:72rpx; height:72rpx; border-radius:16rpx; background-color:#c7e0ff; }
    .bubble{ background-color:#ffffff; border-radius:16rpx; padding:20rpx 24rpx; margin:0 20rpx; max-width:460rpx; }
    .b-me{ background-color:#95ec69; }
    .tx{ font-size:30rpx; color:#1a1a1a; }
    "#;
    let data = json!({"msgs":[
        {"id":1,"me":false,"text":"在吗？周末有空一起爬山不？"},
        {"id":2,"me":true,"text":"有空啊！几点集合？"},
        {"id":3,"me":false,"text":"早上八点，老地方见"},
        {"id":4,"me":true,"text":"好嘞，带上水和干粮"},
        {"id":5,"me":false,"text":"好的，到时候见"}
    ]});
    render("07_chat", 0xEDEDED, wxml, wxss, data);
}

// 8. 动态流
fn feed() {
    let wxml = r#"
    <view class="page">
        <view class="card" wx:for="{{posts}}" wx:key="id">
            <view class="row top">
                <image class="avatar" src="{{item.avatar}}" mode="aspectFill"></image>
                <view class="col grow"><text class="name">{{item.user}}</text><text class="time">{{item.time}}</text></view>
                <view class="follow">关注</view>
            </view>
            <text class="content">{{item.text}}</text>
            <view class="imgs row">
                <image class="ig" src="{{item.img1}}" mode="aspectFill"></image>
                <image class="ig" src="{{item.img2}}" mode="aspectFill"></image>
                <image class="ig" src="{{item.img3}}" mode="aspectFill"></image>
            </view>
            <view class="actions row between">
                <text class="act">赞 {{item.likes}}</text>
                <text class="act">评论 {{item.comments}}</text>
                <text class="act">转发</text>
            </view>
        </view>
    </view>"#;
    let wxss = r#"
    .page{ padding:20rpx; }
    .card{ background-color:#fff; border-radius:20rpx; padding:28rpx; margin-bottom:20rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .top{ margin-bottom:20rpx; }
    .avatar{ width:84rpx; height:84rpx; border-radius:42rpx; margin-right:20rpx; }
    .col{ display:flex; flex-direction:column; }
    .grow{ flex:1; }
    .name{ font-size:30rpx; color:#1a1a1a; font-weight:bold; }
    .time{ font-size:24rpx; color:#bbb; margin-top:6rpx; }
    .follow{ background-color:#07c160; color:#fff; font-size:24rpx; padding:10rpx 24rpx; border-radius:28rpx; }
    .content{ font-size:30rpx; color:#333; line-height:44rpx; }
    .imgs{ display:flex; flex-direction:row; margin-top:20rpx; }
    .ig{ width:210rpx; height:210rpx; border-radius:12rpx; margin-right:12rpx; }
    .actions{ display:flex; flex-direction:row; justify-content:space-between; margin-top:24rpx; }
    .act{ font-size:26rpx; color:#888; }
    "#;
    let data = json!({"posts":[
        {"id":1,"user":"摄影师阿凯","time":"10分钟前","text":"周末去了趟海边，随手一拍都是壁纸","likes":328,"comments":42,
         "avatar":"doc/gallery/assets/avatar1.jpg","img1":"doc/gallery/assets/p_coffee.jpg","img2":"doc/gallery/assets/p_cake.jpg","img3":"doc/gallery/assets/banner.jpg"},
        {"id":2,"user":"美食日记","time":"1小时前","text":"在家复刻了一份提拉米苏，成功！","likes":156,"comments":23,
         "avatar":"doc/gallery/assets/avatar2.jpg","img1":"doc/gallery/assets/p_cake.jpg","img2":"doc/gallery/assets/p_coffee.jpg","img3":"doc/gallery/assets/p_thermos.jpg"}
    ]});
    render("08_feed", 0xF5F6F8, wxml, wxss, data);
}

// 9. 宫格导航
fn grid_menu() {
    let wxml = r#"
    <view class="page">
        <text class="head">全部服务</text>
        <view class="grid">
            <view class="cell col" wx:for="{{apps}}" wx:key="*this">
                <view class="ico" style="background-color:{{item.c}}"></view>
                <text class="lbl">{{item.n}}</text>
            </view>
        </view>
    </view>"#;
    let wxss = r#"
    .page{ padding:24rpx; }
    .head{ font-size:34rpx; font-weight:bold; color:#1a1a1a; margin-bottom:24rpx; }
    .grid{ display:flex; flex-direction:row; flex-wrap:wrap; background-color:#fff; border-radius:20rpx; padding:24rpx; }
    .cell{ width:25%; display:flex; flex-direction:column; align-items:center; margin-bottom:36rpx; }
    .ico{ width:96rpx; height:96rpx; border-radius:24rpx; }
    .lbl{ font-size:24rpx; color:#555; margin-top:14rpx; }
    "#;
    let data = json!({"apps":[
        {"n":"扫一扫","c":"#4a90d9"},{"n":"付款码","c":"#07c160"},{"n":"卡包","c":"#ff9500"},{"n":"出行","c":"#5856d6"},
        {"n":"外卖","c":"#ff3b30"},{"n":"充值","c":"#34c759"},{"n":"理财","c":"#ffcc00"},{"n":"医疗","c":"#af52de"},
        {"n":"公益","c":"#ff2d55"},{"n":"游戏","c":"#00c7be"},{"n":"读书","c":"#ff6b35"},{"n":"更多","c":"#8e8e93"}
    ]});
    render("09_grid_menu", 0xF5F6F8, wxml, wxss, data);
}

// 10. 表单
fn form() {
    let wxml = r#"
    <view class="page">
        <view class="card">
            <view class="fi col"><text class="lb">姓名</text><input value="张三" placeholder="请输入姓名" /></view>
            <view class="fi col"><text class="lb">手机号</text><input value="13800138000" /></view>
            <view class="fi row between"><text class="lb">性别</text>
                <view class="row"><radio checked="true" /><text class="rl">男</text><radio /><text class="rl">女</text></view>
            </view>
            <view class="fi row between"><text class="lb">接收通知</text><switch checked="true" /></view>
            <view class="fi col"><text class="lb">满意度</text><slider value="70" show-value="true" /></view>
            <view class="fi row"><checkbox checked="true" /><text class="agree">我已阅读并同意用户协议</text></view>
        </view>
        <view class="submit">提交</view>
    </view>"#;
    let wxss = r#"
    .page{ padding:24rpx; }
    .card{ background-color:#fff; border-radius:20rpx; padding:12rpx 28rpx; }
    .fi{ padding:26rpx 0; border-bottom:1rpx solid #f2f2f2; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .col{ display:flex; flex-direction:column; }
    .between{ justify-content:space-between; }
    .lb{ font-size:28rpx; color:#999; margin-bottom:12rpx; }
    .fi input{ font-size:32rpx; color:#333; }
    .rl{ font-size:30rpx; color:#333; margin:0 30rpx 0 10rpx; }
    .agree{ font-size:26rpx; color:#666; margin-left:14rpx; }
    .submit{ background-color:#07c160; color:#fff; text-align:center; font-size:32rpx; padding:26rpx; border-radius:44rpx; margin-top:40rpx; }
    "#;
    render("10_form", 0xF5F6F8, wxml, wxss, json!({}));
}

// 11. 数据看板
fn dashboard() {
    let wxml = r#"
    <view class="page">
        <text class="head">数据概览</text>
        <view class="cards row">
            <view class="stat" style="background-color:#4a90d9"><text class="sv">1,286</text><text class="sl">今日访客</text></view>
            <view class="stat" style="background-color:#07c160"><text class="sv">¥8,420</text><text class="sl">今日收入</text></view>
        </view>
        <view class="cards row">
            <view class="stat" style="background-color:#ff9500"><text class="sv">328</text><text class="sl">新增订单</text></view>
            <view class="stat" style="background-color:#af52de"><text class="sv">92%</text><text class="sl">好评率</text></view>
        </view>
        <view class="card">
            <text class="ct">分类占比</text>
            <view class="bar-row row" wx:for="{{bars}}" wx:key="n">
                <text class="bn">{{item.n}}</text>
                <view class="track grow"><view class="fill" style="width:{{item.p}};background-color:{{item.c}}"></view></view>
                <text class="bp">{{item.p}}</text>
            </view>
        </view>
    </view>"#;
    let wxss = r#"
    .page{ padding:24rpx; }
    .head{ font-size:36rpx; font-weight:bold; color:#1a1a1a; margin-bottom:20rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .cards{ display:flex; flex-direction:row; margin-bottom:20rpx; }
    .stat{ flex:1; border-radius:20rpx; padding:32rpx; margin-right:20rpx; display:flex; flex-direction:column; }
    .sv{ font-size:44rpx; color:#fff; font-weight:bold; }
    .sl{ font-size:24rpx; color:#eafff2; margin-top:10rpx; }
    .card{ background-color:#fff; border-radius:20rpx; padding:28rpx; margin-top:8rpx; }
    .ct{ font-size:30rpx; color:#1a1a1a; font-weight:bold; }
    .bar-row{ display:flex; flex-direction:row; align-items:center; margin-top:26rpx; }
    .bn{ font-size:26rpx; color:#666; width:120rpx; }
    .track{ height:20rpx; background-color:#f0f0f0; border-radius:10rpx; flex:1; margin:0 16rpx; }
    .fill{ height:20rpx; border-radius:10rpx; }
    .bp{ font-size:24rpx; color:#999; width:70rpx; text-align:right; }
    "#;
    let data = json!({"bars":[
        {"n":"电子产品","p":"75%","c":"#4a90d9"},
        {"n":"服饰","p":"55%","c":"#07c160"},
        {"n":"食品","p":"40%","c":"#ff9500"},
        {"n":"其他","p":"20%","c":"#af52de"}
    ]});
    render("11_dashboard", 0xF5F6F8, wxml, wxss, data);
}

// 12. 图片画廊
fn gallery_grid() {
    let wxml = r#"
    <view class="page">
        <text class="head">发现美图</text>
        <view class="grid">
            <view class="tile" wx:for="{{tiles}}" wx:key="*this" style="background-color:{{item}}"></view>
        </view>
    </view>"#;
    let wxss = r#"
    .page{ padding:24rpx; }
    .head{ font-size:34rpx; font-weight:bold; color:#1a1a1a; margin-bottom:20rpx; }
    .grid{ display:flex; flex-direction:row; flex-wrap:wrap; }
    .tile{ width:220rpx; height:220rpx; border-radius:16rpx; margin:0 12rpx 12rpx 0; }
    "#;
    let data = json!({"tiles":[
        "#ffadad","#ffd6a5","#fdffb6","#caffbf","#9bf6ff","#a0c4ff","#bdb2ff","#ffc6ff","#fffffc"
    ]});
    render("12_gallery", 0xFFFFFF, wxml, wxss, data);
}

// 13. 天气
fn weather() {
    let wxml = r#"
    <view class="wrap">
        <view class="hero">
            <text class="city">深圳市 · 南山区</text>
            <text class="temp">26°</text>
            <text class="desc">多云转晴  ·  东南风 3级</text>
            <text class="range">最高 29°  最低 21°</text>
        </view>
        <view class="hours card">
            <view class="hour col" wx:for="{{hours}}" wx:key="t">
                <text class="ht">{{item.t}}</text>
                <view class="dot"></view>
                <text class="hv">{{item.v}}°</text>
            </view>
        </view>
        <view class="card">
            <view class="drow row between" wx:for="{{days}}" wx:key="d">
                <text class="dd">{{item.d}}</text><text class="dw">{{item.w}}</text><text class="dt">{{item.t}}</text>
            </view>
        </view>
    </view>"#;
    let wxss = r#"
    .hero{ background-color:#4a90d9; padding:70rpx 40rpx 50rpx 40rpx; display:flex; flex-direction:column; align-items:center; }
    .city{ font-size:32rpx; color:#eaf3ff; }
    .temp{ font-size:140rpx; color:#fff; font-weight:bold; }
    .desc{ font-size:28rpx; color:#eaf3ff; }
    .range{ font-size:26rpx; color:#cfe2ff; margin-top:12rpx; }
    .card{ background-color:#fff; border-radius:20rpx; padding:28rpx; margin:24rpx; }
    .hours{ display:flex; flex-direction:row; justify-content:space-between; }
    .hour{ display:flex; flex-direction:column; align-items:center; flex:1; }
    .ht{ font-size:24rpx; color:#999; }
    .dot{ width:20rpx; height:20rpx; border-radius:10rpx; background-color:#ffcc00; margin:18rpx 0; }
    .hv{ font-size:28rpx; color:#333; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .drow{ display:flex; flex-direction:row; justify-content:space-between; padding:20rpx 0; border-bottom:1rpx solid #f2f2f2; }
    .dd{ font-size:28rpx; color:#333; width:120rpx; }
    .dw{ font-size:28rpx; color:#888; }
    .dt{ font-size:28rpx; color:#333; }
    "#;
    let data = json!({
        "hours":[{"t":"现在","v":26},{"t":"14时","v":28},{"t":"15时","v":29},{"t":"16时","v":28},{"t":"17时","v":26}],
        "days":[{"d":"今天","w":"多云","t":"21°~29°"},{"d":"明天","w":"晴","t":"22°~31°"},{"d":"周三","w":"小雨","t":"20°~26°"}]
    });
    render("13_weather", 0x4A90D9, wxml, wxss, data);
}

// 14. 订单列表
fn orders() {
    let wxml = r#"
    <view class="page">
        <view class="tabs row">
            <text class="tab on">全部</text><text class="tab">待付款</text><text class="tab">待发货</text><text class="tab">待收货</text>
        </view>
        <view class="card" wx:for="{{orders}}" wx:key="id">
            <view class="row between oh">
                <text class="shop">{{item.shop}}</text>
                <text class="st" style="color:{{item.c}}">{{item.status}}</text>
            </view>
            <view class="row goods">
                <image class="thumb" src="{{item.img}}" mode="aspectFill"></image>
                <view class="col grow gi"><text class="gn">{{item.name}}</text><text class="gs">{{item.sku}}</text></view>
                <view class="col gp"><text class="pp">¥{{item.price}}</text><text class="qn">x{{item.qty}}</text></view>
            </view>
            <view class="row between foot">
                <text class="ft">共{{item.qty}}件 合计 <text class="fp">¥{{item.price}}</text></text>
                <view class="btn">{{item.action}}</view>
            </view>
        </view>
    </view>"#;
    let wxss = r#"
    .page{ padding:0 0 24rpx 0; }
    .tabs{ display:flex; flex-direction:row; background-color:#fff; padding:24rpx 0; }
    .tab{ flex:1; text-align:center; font-size:28rpx; color:#666; }
    .tab.on{ color:#07c160; font-weight:bold; }
    .card{ background-color:#fff; border-radius:20rpx; padding:26rpx; margin:20rpx 24rpx 0 24rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .col{ display:flex; flex-direction:column; }
    .grow{ flex:1; }
    .oh{ padding-bottom:20rpx; border-bottom:1rpx solid #f2f2f2; }
    .shop{ font-size:28rpx; color:#333; }
    .st{ font-size:26rpx; }
    .goods{ display:flex; flex-direction:row; padding:24rpx 0; }
    .thumb{ width:140rpx; height:140rpx; border-radius:12rpx; }
    .gi{ margin-left:20rpx; flex:1; }
    .gn{ font-size:30rpx; color:#1a1a1a; }
    .gs{ font-size:24rpx; color:#999; margin-top:10rpx; }
    .gp{ display:flex; flex-direction:column; align-items:flex-end; }
    .pp{ font-size:30rpx; color:#333; }
    .qn{ font-size:24rpx; color:#999; margin-top:10rpx; }
    .foot{ padding-top:20rpx; }
    .ft{ font-size:26rpx; color:#666; }
    .fp{ font-size:32rpx; color:#ff5000; font-weight:bold; }
    .btn{ border:1rpx solid #07c160; color:#07c160; font-size:26rpx; padding:12rpx 28rpx; border-radius:32rpx; }
    "#;
    let data = json!({"orders":[
        {"id":1,"shop":"官方旗舰店","status":"待发货","c":"#ff9500","name":"智能运动手表","sku":"黑色 46mm","price":"599","qty":1,"action":"提醒发货","img":"doc/gallery/assets/p_watch.jpg"},
        {"id":2,"shop":"数码专营店","status":"已完成","c":"#999","name":"无线蓝牙耳机","sku":"白色","price":"199","qty":2,"action":"再次购买","img":"doc/gallery/assets/p_earbuds.jpg"}
    ]});
    render("14_orders", 0xF5F6F8, wxml, wxss, data);
}

// 15. 首页
fn home() {
    let wxml = r#"
    <view class="wrap">
        <view class="search row"><view class="sbox row"><view class="sicon"></view><text class="sp">搜索商品</text></view></view>
        <image class="banner" src="doc/gallery/assets/banner.jpg" mode="aspectFill"></image>
        <view class="cats row">
            <view class="cat col" wx:for="{{cats}}" wx:key="n"><view class="ci" style="background-color:{{item.c}}"></view><text class="cn">{{item.n}}</text></view>
        </view>
        <view class="sec row between"><text class="stitle">热销推荐</text><text class="more">更多 ></text></view>
        <view class="plist row">
            <view class="pcard col" wx:for="{{prods}}" wx:key="n">
                <image class="pimg" src="{{item.img}}" mode="aspectFill"></image>
                <text class="pn">{{item.n}}</text>
                <text class="pp">¥{{item.p}}</text>
            </view>
        </view>
    </view>"#;
    let wxss = r#"
    .search{ background-color:#07c160; padding:24rpx; }
    .sbox{ flex:1; background-color:#fff; border-radius:32rpx; padding:18rpx 28rpx; display:flex; flex-direction:row; align-items:center; }
    .sicon{ width:30rpx; height:30rpx; border-radius:15rpx; background-color:#ddd; margin-right:14rpx; }
    .sp{ font-size:28rpx; color:#bbb; }
    .banner{ height:260rpx; margin:24rpx; border-radius:20rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .col{ display:flex; flex-direction:column; }
    .cats{ display:flex; flex-direction:row; justify-content:space-between; padding:0 24rpx; }
    .cat{ display:flex; flex-direction:column; align-items:center; flex:1; }
    .ci{ width:96rpx; height:96rpx; border-radius:48rpx; }
    .cn{ font-size:24rpx; color:#555; margin-top:12rpx; }
    .sec{ display:flex; flex-direction:row; justify-content:space-between; padding:36rpx 24rpx 16rpx 24rpx; }
    .stitle{ font-size:34rpx; font-weight:bold; color:#1a1a1a; }
    .more{ font-size:26rpx; color:#999; }
    .plist{ display:flex; flex-direction:row; padding:0 24rpx; }
    .pcard{ flex:1; background-color:#fff; border-radius:16rpx; padding:16rpx; margin-right:16rpx; display:flex; flex-direction:column; }
    .pimg{ width:100%; height:200rpx; border-radius:12rpx; }
    .pn{ font-size:26rpx; color:#333; margin-top:14rpx; }
    .pp{ font-size:32rpx; color:#ff5000; font-weight:bold; margin-top:8rpx; }
    "#;
    let data = json!({
        "cats":[{"n":"数码","c":"#4a90d9"},{"n":"服饰","c":"#ff3b30"},{"n":"美妆","c":"#ff9500"},{"n":"食品","c":"#34c759"},{"n":"家居","c":"#af52de"}],
        "prods":[{"n":"蓝牙耳机","p":"199","img":"doc/gallery/assets/p_earbuds.jpg"},{"n":"运动手表","p":"599","img":"doc/gallery/assets/p_watch.jpg"},{"n":"双肩背包","p":"129","img":"doc/gallery/assets/p_backpack.jpg"}]
    });
    render("15_home", 0xF5F6F8, wxml, wxss, data);
}

// 16. 通讯录
fn contacts() {
    let wxml = r#"
    <view class="page">
        <view class="grp" wx:for="{{groups}}" wx:key="letter">
            <text class="gl">{{item.letter}}</text>
            <view class="ci row" wx:for="{{item.people}}" wx:for-item="p" wx:key="*this">
                <view class="av" style="background-color:{{p.c}}"></view>
                <text class="nm">{{p.n}}</text>
            </view>
        </view>
    </view>"#;
    let wxss = r#"
    .page{ background-color:#fff; }
    .gl{ font-size:26rpx; color:#999; background-color:#f5f6f8; padding:10rpx 32rpx; }
    .ci{ display:flex; flex-direction:row; align-items:center; padding:22rpx 32rpx; border-bottom:1rpx solid #f5f5f5; }
    .av{ width:80rpx; height:80rpx; border-radius:16rpx; margin-right:24rpx; }
    .nm{ font-size:32rpx; color:#1a1a1a; }
    "#;
    let data = json!({"groups":[
        {"letter":"A","people":[{"n":"阿伟","c":"#ff9f43"},{"n":"安琪","c":"#ee5253"}]},
        {"letter":"L","people":[{"n":"李雷","c":"#0abde3"},{"n":"林徽因","c":"#10ac84"},{"n":"刘备","c":"#5f27cd"}]},
        {"letter":"Z","people":[{"n":"张三","c":"#576574"},{"n":"赵云","c":"#ff6b6b"}]}
    ]});
    render("16_contacts", 0xFFFFFF, wxml, wxss, data);
}

// 17. 音乐播放器（真实封面 + 控制图标）
fn music() {
    let wxml = r#"
    <view class="wrap col">
        <image class="cover" src="doc/gallery/assets/album.jpg" mode="aspectFill" />
        <text class="song">夜空中最亮的星</text>
        <text class="singer">逃跑计划 · 世界</text>
        <view class="prog row">
            <text class="tm">01:24</text>
            <view class="track grow"><view class="fill"></view><view class="knob"></view></view>
            <text class="tm">04:12</text>
        </view>
        <view class="ctrls row between">
            <image class="c" src="doc/gallery/assets/icons/shuffle.png" mode="aspectFit" />
            <image class="c2" src="doc/gallery/assets/icons/prev.png" mode="aspectFit" />
            <view class="playbtn"><image class="pico" src="doc/gallery/assets/icons/play.png" mode="aspectFit" /></view>
            <image class="c2" src="doc/gallery/assets/icons/next.png" mode="aspectFit" />
            <image class="c" src="doc/gallery/assets/icons/repeat.png" mode="aspectFit" />
        </view>
    </view>"#;
    let wxss = r#"
    .wrap{ display:flex; flex-direction:column; align-items:center; padding:70rpx 48rpx; }
    .cover{ width:520rpx; height:520rpx; border-radius:28rpx; margin-bottom:56rpx; background-color:#2a2a34; }
    .song{ font-size:44rpx; color:#fff; font-weight:bold; }
    .singer{ font-size:28rpx; color:#aaa; margin-top:16rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .col{ display:flex; flex-direction:column; }
    .grow{ flex:1; }
    .between{ justify-content:space-between; }
    .prog{ display:flex; flex-direction:row; align-items:center; width:100%; margin-top:70rpx; }
    .tm{ font-size:24rpx; color:#999; }
    .track{ height:8rpx; background-color:#3a3a4a; border-radius:4rpx; margin:0 20rpx; }
    .fill{ width:190rpx; height:8rpx; background-color:#07c160; border-radius:4rpx; }
    .knob{ width:26rpx; height:26rpx; border-radius:13rpx; background-color:#fff; margin-top:-9rpx; margin-left:-13rpx; }
    .ctrls{ display:flex; flex-direction:row; align-items:center; justify-content:space-between; width:100%; margin-top:70rpx; }
    .c{ width:52rpx; height:52rpx; }
    .c2{ width:64rpx; height:64rpx; }
    .playbtn{ width:130rpx; height:130rpx; border-radius:65rpx; background-color:#07c160; display:flex; flex-direction:row; justify-content:center; align-items:center; }
    .pico{ width:56rpx; height:56rpx; }
    "#;
    render("17_music", 0x18181F, wxml, wxss, json!({}));
}

// 18. 标签 / 徽章 / 按钮
fn tags() {
    let wxml = r#"
    <view class="page">
        <text class="h">按钮</text>
        <view class="card row wrap">
            <view class="btn primary">主要按钮</view>
            <view class="btn default">默认按钮</view>
            <view class="btn warn">警告按钮</view>
            <view class="btn ghost">描边按钮</view>
        </view>
        <text class="h">标签</text>
        <view class="card row wrap">
            <text class="tag t1">新品</text><text class="tag t2">热卖</text><text class="tag t3">包邮</text>
            <text class="tag t4">限时</text><text class="tag t5">秒杀</text><text class="tag t6">推荐</text>
        </view>
        <text class="h">状态徽章</text>
        <view class="card">
            <view class="brow row between"><text class="bt">未读消息</text><view class="badge">99+</view></view>
            <view class="brow row between"><text class="bt">进行中</text><view class="dotb dg"></view></view>
            <view class="brow row between"><text class="bt">已离线</text><view class="dotb dgy"></view></view>
        </view>
    </view>"#;
    let wxss = r#"
    .page{ padding:24rpx; }
    .h{ font-size:28rpx; color:#999; margin:20rpx 0 14rpx 8rpx; }
    .card{ background-color:#fff; border-radius:20rpx; padding:28rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .wrap{ flex-wrap:wrap; }
    .between{ justify-content:space-between; }
    .btn{ font-size:28rpx; padding:18rpx 32rpx; border-radius:12rpx; margin:0 16rpx 16rpx 0; }
    .primary{ background-color:#07c160; color:#fff; }
    .default{ background-color:#f2f2f2; color:#333; }
    .warn{ background-color:#ff9500; color:#fff; }
    .ghost{ border:1rpx solid #07c160; color:#07c160; }
    .tag{ font-size:24rpx; padding:8rpx 20rpx; border-radius:24rpx; margin:0 16rpx 16rpx 0; }
    .t1{ background-color:#e8f7ee; color:#07c160; }
    .t2{ background-color:#ffeceb; color:#ff3b30; }
    .t3{ background-color:#fff3e0; color:#ff9500; }
    .t4{ background-color:#eef1ff; color:#5856d6; }
    .t5{ background-color:#ffeaf3; color:#ff2d70; }
    .t6{ background-color:#e6f7ff; color:#0a94ff; }
    .brow{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; padding:24rpx 0; border-bottom:1rpx solid #f2f2f2; }
    .bt{ font-size:30rpx; color:#333; }
    .badge{ background-color:#ff3b30; color:#fff; font-size:22rpx; padding:6rpx 14rpx; border-radius:20rpx; }
    .dotb{ width:24rpx; height:24rpx; border-radius:12rpx; }
    .dg{ background-color:#07c160; }
    .dgy{ background-color:#c7c7cc; }
    "#;
    render("18_tags", 0xF5F6F8, wxml, wxss, json!({}));
}

// 19. 确认弹窗（Modal）
fn modal_dialog() {
    let wxml = r##"
    <view class="mask">
        <view class="dialog">
            <text class="d-title">删除确认</text>
            <text class="d-content">确定要删除选中的 3 件商品吗？删除后将无法恢复。</text>
            <view class="d-btns row">
                <text class="d-btn cancel">取消</text>
                <text class="d-btn ok">删除</text>
            </view>
        </view>
    </view>"##;
    let wxss = r##"
    .mask{ height:1334rpx; background-color:rgba(0,0,0,0.55); display:flex; flex-direction:column; justify-content:center; align-items:center; }
    .dialog{ width:580rpx; background-color:#ffffff; border-radius:24rpx; display:flex; flex-direction:column; align-items:center; padding-top:48rpx; }
    .d-title{ font-size:34rpx; font-weight:bold; color:#1a1a1a; }
    .d-content{ width:490rpx; font-size:28rpx; color:#888888; text-align:center; line-height:44rpx; margin:24rpx 0 40rpx 0; }
    .row{ display:flex; flex-direction:row; }
    .d-btns{ display:flex; flex-direction:row; width:580rpx; border-top:1rpx solid #f0f0f0; }
    .d-btn{ flex:1; text-align:center; padding:30rpx; font-size:32rpx; }
    .cancel{ color:#888888; }
    .ok{ color:#fa5151; font-weight:bold; }
    "##;
    render_screen("19_modal", 0xEDEDED, wxml, wxss, json!({}));
}

// 20. 底部操作面板（ActionSheet）
fn action_sheet() {
    let wxml = r##"
    <view class="mask">
        <view class="sheet">
            <text class="opt" wx:for="{{opts}}" wx:key="*this">{{item}}</text>
            <view class="gap"></view>
            <text class="opt cancel">取消</text>
        </view>
    </view>"##;
    let wxss = r##"
    .mask{ height:1334rpx; background-color:rgba(0,0,0,0.55); display:flex; flex-direction:column; justify-content:flex-end; }
    .sheet{ background-color:#f7f7f7; display:flex; flex-direction:column; }
    .opt{ background-color:#ffffff; text-align:center; font-size:32rpx; color:#1a1a1a; padding:34rpx; border-bottom:1rpx solid #f0f0f0; }
    .cancel{ color:#576b95; font-weight:bold; border-bottom:0; }
    .gap{ height:16rpx; background-color:#ededed; }
    "##;
    render_screen("20_action_sheet", 0xEDEDED, wxml, wxss, json!({
        "opts": ["拍照", "从相册选择", "保存到本地", "分享给好友"]
    }));
}

// 21. 轻提示（Toast）
fn toast() {
    let wxml = r##"
    <view class="wrap">
        <view class="toast">
            <icon type="success_no_circle" size="60" color="#ffffff" />
            <text class="tt">操作成功</text>
        </view>
    </view>"##;
    let wxss = r##"
    .wrap{ height:1334rpx; display:flex; flex-direction:column; justify-content:center; align-items:center; }
    .toast{ background-color:rgba(0,0,0,0.8); border-radius:20rpx; padding:44rpx 56rpx; display:flex; flex-direction:column; align-items:center; }
    .tt{ color:#ffffff; font-size:30rpx; margin-top:24rpx; }
    "##;
    render_screen("21_toast", 0xF5F6F8, wxml, wxss, json!({}));
}

// 22. 轮播（Swiper）
fn swiper_banner() {
    let wxml = r##"
    <view class="page">
        <text class="head">Swiper 轮播</text>
        <swiper class="sw" indicator-dots="true" indicator-active-color="#ffffff" indicator-color="rgba(255,255,255,0.5)">
            <swiper-item class="s1"><text class="st">春季新品 低至 5 折</text></swiper-item>
            <swiper-item class="s2"><text class="st">会员专享 满 199 减 50</text></swiper-item>
            <swiper-item class="s3"><text class="st">限时秒杀 每日 10 点</text></swiper-item>
        </swiper>
        <text class="tip">支持左右滑动切换 · 自动轮播 · 指示点</text>
        <view class="dots row">
            <view class="d1"></view><view class="d0"></view><view class="d0"></view>
        </view>
    </view>"##;
    let wxss = r##"
    .page{ padding:24rpx; }
    .head{ font-size:36rpx; font-weight:bold; color:#1a1a1a; margin-bottom:24rpx; }
    .sw{ height:320rpx; border-radius:20rpx; }
    .s1{ background-color:#ff7a45; }
    .s2{ background-color:#4a90d9; }
    .s3{ background-color:#722ed1; }
    .st{ color:#ffffff; font-size:40rpx; font-weight:bold; padding:130rpx 40rpx; }
    .tip{ font-size:26rpx; color:#999999; margin-top:28rpx; text-align:center; }
    .row{ display:flex; flex-direction:row; justify-content:center; align-items:center; }
    .dots{ display:flex; flex-direction:row; justify-content:center; margin-top:20rpx; }
    .d1{ width:36rpx; height:12rpx; border-radius:6rpx; background-color:#ff7a45; margin:0 6rpx; }
    .d0{ width:12rpx; height:12rpx; border-radius:6rpx; background-color:#dddddd; margin:0 6rpx; }
    "##;
    render("22_swiper", 0xFFFFFF, wxml, wxss, json!({}));
}

// 23. 横向滚动（左右滑动）
fn h_scroll() {
    let wxml = r##"
    <view class="page">
        <view class="row between hd">
            <text class="head">猜你喜欢</text>
            <text class="more">左右滑动 ></text>
        </view>
        <scroll-view scroll-x="true" class="hs">
            <view class="hcard" wx:for="{{cards}}" wx:key="n">
                <image class="hthumb" src="{{item.img}}" mode="aspectFill"></image>
                <text class="hn">{{item.n}}</text>
                <text class="hp">¥{{item.p}}</text>
            </view>
        </scroll-view>
    </view>"##;
    let wxss = r##"
    .page{ padding:24rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .hd{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; margin-bottom:20rpx; }
    .head{ font-size:36rpx; font-weight:bold; color:#1a1a1a; }
    .more{ font-size:26rpx; color:#999999; }
    .hs{ display:flex; flex-direction:row; }
    .hcard{ width:240rpx; background-color:#ffffff; border-radius:16rpx; padding:16rpx; margin-right:20rpx; display:flex; flex-direction:column; }
    .hthumb{ width:100%; height:240rpx; border-radius:12rpx; }
    .hn{ font-size:28rpx; color:#333333; margin-top:16rpx; }
    .hp{ font-size:32rpx; color:#ff5000; font-weight:bold; margin-top:8rpx; }
    "##;
    render("23_h_scroll", 0xF5F6F8, wxml, wxss, json!({"cards":[
        {"n":"手工蛋糕","p":"168","img":"doc/gallery/assets/p_cake.jpg"},
        {"n":"精品咖啡","p":"39","img":"doc/gallery/assets/p_coffee.jpg"},
        {"n":"保温杯","p":"89","img":"doc/gallery/assets/p_thermos.jpg"},
        {"n":"双肩背包","p":"258","img":"doc/gallery/assets/p_backpack.jpg"},
        {"n":"口红套装","p":"158","img":"doc/gallery/assets/p_lipstick.jpg"}
    ]}));
}

// 24. 纵向长列表（上下滑动）
fn v_scroll() {
    let wxml = r##"
    <view class="page">
        <view class="nav"><text class="nav-t">消息</text></view>
        <scroll-view scroll-y="true" class="vs">
            <view class="mi row" wx:for="{{msgs}}" wx:key="n">
                <view class="mav" style="background-color:{{item.c}}"></view>
                <view class="col grow mc">
                    <view class="row between"><text class="mn">{{item.n}}</text><text class="mtime">{{item.t}}</text></view>
                    <text class="mm">{{item.m}}</text>
                </view>
            </view>
        </scroll-view>
    </view>"##;
    let wxss = r##"
    .page{ }
    .nav{ background-color:#ffffff; padding:28rpx; border-bottom:1rpx solid #f0f0f0; }
    .nav-t{ font-size:36rpx; font-weight:bold; color:#1a1a1a; text-align:center; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .col{ display:flex; flex-direction:column; }
    .grow{ flex:1; }
    .between{ justify-content:space-between; }
    .vs{ height:1100rpx; background-color:#ffffff; }
    .mi{ display:flex; flex-direction:row; align-items:center; padding:24rpx 28rpx; border-bottom:1rpx solid #f2f2f2; }
    .mav{ width:96rpx; height:96rpx; border-radius:16rpx; margin-right:24rpx; }
    .mc{ flex:1; display:flex; flex-direction:column; }
    .mn{ font-size:32rpx; color:#1a1a1a; }
    .mtime{ font-size:24rpx; color:#bbbbbb; }
    .mm{ font-size:28rpx; color:#999999; margin-top:10rpx; }
    "##;
    render_screen("24_v_scroll", 0xF5F6F8, wxml, wxss, json!({"msgs":[
        {"n":"订单助手","m":"您的订单已发货，请注意查收","t":"09:30","c":"#4a90d9"},
        {"n":"微信支付","m":"向 星巴克 付款 32 元","t":"08:15","c":"#07c160"},
        {"n":"张三","m":"晚上一起吃饭吗？","t":"昨天","c":"#ff9f43"},
        {"n":"购物优惠","m":"您有 3 张优惠券即将过期","t":"昨天","c":"#ee5253"},
        {"n":"李四","m":"文件已收到，谢谢","t":"周三","c":"#5f27cd"},
        {"n":"运动打卡","m":"今日步数 12800 步，超过 85% 好友","t":"周三","c":"#10ac84"},
        {"n":"新闻早报","m":"每日 8 点为你送上今日热点","t":"周二","c":"#576574"},
        {"n":"王五","m":"周末爬山计划定了吗？","t":"周一","c":"#ff6b6b"},
        {"n":"系统通知","m":"您的账号已完成实名认证","t":"周一","c":"#0abde3"}
    ]}));
}

// 25. 底部选择器（Picker 弹出）
fn bottom_picker() {
    let wxml = r##"
    <view class="mask">
        <view class="pk">
            <view class="pk-hd row between">
                <text class="pk-cancel">取消</text>
                <text class="pk-title">选择城市</text>
                <text class="pk-ok">确定</text>
            </view>
            <view class="pk-body">
                <text class="pk-item dim">北京市</text>
                <text class="pk-item dim">上海市</text>
                <text class="pk-item sel">深圳市</text>
                <text class="pk-item dim">广州市</text>
                <text class="pk-item dim">杭州市</text>
            </view>
        </view>
    </view>"##;
    let wxss = r##"
    .mask{ height:1334rpx; background-color:rgba(0,0,0,0.55); display:flex; flex-direction:column; justify-content:flex-end; }
    .pk{ background-color:#ffffff; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .pk-hd{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; padding:28rpx; border-bottom:1rpx solid #f0f0f0; }
    .pk-cancel{ font-size:30rpx; color:#888888; }
    .pk-title{ font-size:32rpx; color:#1a1a1a; font-weight:bold; }
    .pk-ok{ font-size:30rpx; color:#07c160; font-weight:bold; }
    .pk-body{ display:flex; flex-direction:column; align-items:center; padding:20rpx 0; }
    .pk-item{ font-size:32rpx; padding:18rpx; text-align:center; }
    .dim{ color:#cccccc; }
    .sel{ color:#1a1a1a; font-weight:bold; font-size:36rpx; }
    "##;
    render_screen("25_picker", 0xEDEDED, wxml, wxss, json!({}));
}

// 26. 日历
fn calendar() {
    let wxml = r##"
    <view class="page">
        <view class="cal">
            <view class="c-hd row between">
                <text class="c-arrow">上月</text>
                <text class="c-title">2024年 7月</text>
                <text class="c-arrow">下月</text>
            </view>
            <view class="week row">
                <text class="wd" wx:for="{{weeks}}" wx:key="*this">{{item}}</text>
            </view>
            <view class="days row">
                <view class="day" wx:for="{{days}}" wx:key="d">
                    <text class="dn {{item.s}}">{{item.d}}</text>
                </view>
            </view>
        </view>
    </view>"##;
    let wxss = r##"
    .page{ padding:24rpx; }
    .cal{ background-color:#ffffff; border-radius:20rpx; padding:24rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .c-hd{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; padding:12rpx 20rpx 28rpx 20rpx; }
    .c-title{ font-size:34rpx; font-weight:bold; color:#1a1a1a; }
    .c-arrow{ font-size:34rpx; color:#999999; padding:0 20rpx; }
    .week{ display:flex; flex-direction:row; }
    .wd{ width:14%; text-align:center; font-size:26rpx; color:#bbbbbb; padding:12rpx 0; }
    .days{ display:flex; flex-direction:row; flex-wrap:wrap; }
    .day{ width:14%; display:flex; flex-direction:row; justify-content:center; padding:12rpx 0; }
    .dn{ width:64rpx; height:64rpx; text-align:center; font-size:28rpx; color:#333333; border-radius:32rpx; }
    .today{ background-color:#07c160; color:#ffffff; }
    .mark{ background-color:#e8f7ee; color:#07c160; }
    .off{ color:#dddddd; }
    "##;
    let mut days = Vec::new();
    // 前置空白（7月1日为周一，前面补 1 个空 - 简化为周日起，补0）
    for _ in 0..1 { days.push(json!({"d":"","s":"off"})); }
    for d in 1..=31 {
        let s = if d == 18 { "today" } else if d == 6 || d == 20 || d == 25 { "mark" } else { "" };
        days.push(json!({"d": d.to_string(), "s": s}));
    }
    render("26_calendar", 0xF5F6F8, wxml, wxss, json!({
        "weeks": ["日","一","二","三","四","五","六"],
        "days": days
    }));
}

// 27. 评分与步骤条
fn rating_steps() {
    let wxml = r##"
    <view class="page">
        <view class="card">
            <text class="ct">商品评分</text>
            <view class="row stars">
                <icon class="star" type="star" size="30" color="#ffb400" />
                <icon class="star" type="star" size="30" color="#ffb400" />
                <icon class="star" type="star" size="30" color="#ffb400" />
                <icon class="star" type="star" size="30" color="#ffb400" />
                <icon class="star" type="star" size="30" color="#e0e0e0" />
                <text class="score">4.0</text>
            </view>
        </view>
        <view class="card">
            <text class="ct">物流进度</text>
            <view class="steps">
                <view class="step row" wx:for="{{steps}}" wx:key="t">
                    <view class="col dotcol">
                        <view class="dot {{item.on ? 'done' : ''}}"></view>
                        <view class="line {{item.on ? 'lon' : ''}}" wx:if="{{!item.last}}"></view>
                    </view>
                    <view class="col sc">
                        <text class="stt {{item.on ? 'ton' : ''}}">{{item.t}}</text>
                        <text class="sd">{{item.d}}</text>
                    </view>
                </view>
            </view>
        </view>
    </view>"##;
    let wxss = r##"
    .page{ padding:24rpx; }
    .card{ background-color:#ffffff; border-radius:20rpx; padding:32rpx; margin-bottom:24rpx; }
    .ct{ font-size:32rpx; font-weight:bold; color:#1a1a1a; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .col{ display:flex; flex-direction:column; }
    .stars{ display:flex; flex-direction:row; align-items:center; margin-top:24rpx; }
    .star{ margin-right:10rpx; }
    .score{ font-size:36rpx; color:#ffb400; font-weight:bold; margin-left:20rpx; }
    .steps{ margin-top:24rpx; }
    .step{ display:flex; flex-direction:row; }
    .dotcol{ display:flex; flex-direction:column; align-items:center; margin-right:24rpx; }
    .dot{ width:28rpx; height:28rpx; border-radius:14rpx; background-color:#dddddd; }
    .done{ background-color:#07c160; }
    .line{ width:4rpx; height:56rpx; background-color:#e0e0e0; }
    .lon{ background-color:#07c160; }
    .sc{ display:flex; flex-direction:column; padding-bottom:28rpx; }
    .stt{ font-size:30rpx; color:#999999; }
    .ton{ color:#1a1a1a; font-weight:bold; }
    .sd{ font-size:24rpx; color:#bbbbbb; margin-top:8rpx; }
    "##;
    render("27_rating_steps", 0xF5F6F8, wxml, wxss, json!({"steps":[
        {"t":"已签收","d":"07-18 14:20 快递已被本人签收","on":true,"last":false},
        {"t":"派送中","d":"07-18 09:10 快递员正在派送","on":true,"last":false},
        {"t":"运输中","d":"07-17 20:00 到达深圳分拨中心","on":true,"last":false},
        {"t":"已揽收","d":"07-17 10:30 商家已发货","on":true,"last":true}
    ]}));
}

// 28. 电商首页（活动 / 秒杀 / 商品瀑布流，真实图片）
fn ecommerce_home() {
    let wxml = r##"
    <view class="wrap">
        <view class="top row">
            <view class="loc row"><text class="loc-t">深圳</text></view>
            <view class="sbox row"><view class="sic"></view><text class="sp">搜索你想要的好物</text></view>
            <text class="msg">消息</text>
        </view>
        <swiper class="banner" indicator-dots="true" indicator-active-color="#ffffff" indicator-color="rgba(255,255,255,0.5)">
            <swiper-item><image class="bimg" src="doc/gallery/assets/banner.jpg" mode="aspectFill" /></swiper-item>
            <swiper-item><image class="bimg" src="doc/gallery/assets/p_sneakers.jpg" mode="aspectFill" /></swiper-item>
        </swiper>
        <view class="quick row">
            <view class="qi col" wx:for="{{quick}}" wx:key="n">
                <view class="qic" style="background-color:{{item.c}}"><text class="qie" style="color:{{item.tc}}">{{item.s}}</text></view>
                <text class="qn">{{item.n}}</text>
            </view>
        </view>
        <view class="flash card">
            <view class="fh row between">
                <view class="row"><text class="ft">限时秒杀</text><text class="fclock">08:12:33</text></view>
                <text class="fmore">更多 ></text>
            </view>
            <scroll-view scroll-x="true" class="frow">
                <view class="fcard col" wx:for="{{flash}}" wx:key="n">
                    <image class="fimg" src="{{item.img}}" mode="aspectFill" />
                    <text class="fp">¥{{item.p}}</text>
                    <text class="fo">¥{{item.o}}</text>
                </view>
            </scroll-view>
        </view>
        <view class="sech row between"><text class="sect">为你推荐</text><text class="sectip">品质好物</text></view>
        <view class="grid">
            <view class="pcard col" wx:for="{{prods}}" wx:key="n">
                <image class="pimg" src="{{item.img}}" mode="aspectFill" />
                <view class="pinfo col">
                    <text class="pn">{{item.n}}</text>
                    <view class="ptags row">
                        <text class="pt1">{{item.tag}}</text>
                    </view>
                    <view class="prow row between">
                        <view class="row bl"><text class="pcur">¥</text><text class="pp">{{item.p}}</text></view>
                        <text class="psold">已售{{item.sold}}</text>
                    </view>
                </view>
            </view>
        </view>
    </view>"##;
    let wxss = r##"
    .wrap{ background-color:#f5f6f8; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .col{ display:flex; flex-direction:column; }
    .between{ justify-content:space-between; }
    .bl{ align-items:baseline; }
    .top{ display:flex; flex-direction:row; align-items:center; background-color:#07c160; padding:20rpx 24rpx; }
    .loc{ display:flex; flex-direction:row; align-items:center; margin-right:16rpx; }
    .loc-t{ font-size:28rpx; color:#ffffff; font-weight:bold; }
    .loc-a{ font-size:22rpx; color:#ffffff; margin-left:4rpx; }
    .sbox{ flex:1; background-color:#ffffff; border-radius:32rpx; padding:14rpx 24rpx; display:flex; flex-direction:row; align-items:center; }
    .sic{ width:28rpx; height:28rpx; border-radius:14rpx; background-color:#cccccc; margin-right:12rpx; }
    .sp{ font-size:26rpx; color:#bbbbbb; }
    .msg{ font-size:26rpx; color:#ffffff; margin-left:16rpx; }
    .banner{ height:300rpx; margin:24rpx; border-radius:20rpx; }
    .bimg{ width:100%; height:300rpx; border-radius:20rpx; }
    .quick{ display:flex; flex-direction:row; justify-content:space-between; padding:0 24rpx 8rpx 24rpx; }
    .qi{ display:flex; flex-direction:column; align-items:center; flex:1; }
    .qic{ width:96rpx; height:96rpx; border-radius:28rpx; display:flex; flex-direction:row; justify-content:center; align-items:center; }
    .qie{ font-size:32rpx; font-weight:bold; }
    .qn{ font-size:24rpx; color:#555555; margin-top:12rpx; }
    .card{ background-color:#ffffff; border-radius:20rpx; margin:16rpx 24rpx; padding:24rpx; }
    .flash{ }
    .fh{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; margin-bottom:20rpx; }
    .ft{ font-size:34rpx; font-weight:bold; color:#ff3b30; margin-right:16rpx; }
    .fclock{ font-size:24rpx; color:#ffffff; background-color:#1a1a1a; padding:6rpx 14rpx; border-radius:8rpx; }
    .fmore{ font-size:26rpx; color:#999999; }
    .frow{ display:flex; flex-direction:row; }
    .fcard{ display:flex; flex-direction:column; width:180rpx; margin-right:20rpx; }
    .fimg{ width:180rpx; height:180rpx; border-radius:12rpx; }
    .fp{ font-size:32rpx; color:#ff3b30; font-weight:bold; margin-top:12rpx; }
    .fo{ font-size:24rpx; color:#bbbbbb; text-decoration:line-through; margin-top:4rpx; }
    .sech{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; padding:24rpx 24rpx 8rpx 24rpx; }
    .sect{ font-size:34rpx; font-weight:bold; color:#1a1a1a; }
    .sectip{ font-size:24rpx; color:#999999; }
    .grid{ display:flex; flex-direction:row; flex-wrap:wrap; padding:16rpx; }
    .pcard{ width:50%; display:flex; flex-direction:column; }
    .pimg{ width:329rpx; height:329rpx; border-radius:16rpx; margin:8rpx; }
    .pinfo{ display:flex; flex-direction:column; padding:0 16rpx; }
    .pn{ font-size:28rpx; color:#1a1a1a; line-height:40rpx; }
    .ptags{ display:flex; flex-direction:row; margin-top:10rpx; }
    .pt1{ background-color:#fff0e8; color:#ff5000; font-size:20rpx; padding:4rpx 12rpx; border-radius:6rpx; }
    .prow{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; margin-top:12rpx; }
    .pcur{ font-size:24rpx; color:#ff3b30; }
    .pp{ font-size:38rpx; color:#ff3b30; font-weight:bold; }
    .psold{ font-size:22rpx; color:#bbbbbb; }
    "##;
    let data = json!({
        "quick":[
            {"n":"领券中心","s":"券","c":"#fff0e8","tc":"#ff5000"},
            {"n":"限时秒杀","s":"秒","c":"#ffeceb","tc":"#ff3b30"},
            {"n":"品牌特卖","s":"品","c":"#eef1ff","tc":"#5856d6"},
            {"n":"新人专享","s":"新","c":"#e8f7ee","tc":"#07c160"},
            {"n":"签到有礼","s":"签","c":"#fff3e0","tc":"#ff9500"}
        ],
        "flash":[
            {"p":"39","o":"99","img":"doc/gallery/assets/p_coffee.jpg"},
            {"p":"168","o":"299","img":"doc/gallery/assets/p_cake.jpg"},
            {"p":"89","o":"159","img":"doc/gallery/assets/p_thermos.jpg"},
            {"p":"158","o":"258","img":"doc/gallery/assets/p_lipstick.jpg"}
        ],
        "prods":[
            {"n":"无线蓝牙耳机 主动降噪 长续航","p":"199","tag":"包邮","sold":"2.3万","img":"doc/gallery/assets/p_earbuds.jpg"},
            {"n":"智能运动手表 心率监测 防水","p":"599","tag":"新品","sold":"8600","img":"doc/gallery/assets/p_watch.jpg"},
            {"n":"轻便双肩背包 大容量 商务通勤","p":"258","tag":"热卖","sold":"1.5万","img":"doc/gallery/assets/p_backpack.jpg"},
            {"n":"复古潮流运动鞋 舒适透气","p":"399","tag":"限时","sold":"6200","img":"doc/gallery/assets/p_sneakers.jpg"}
        ]
    });
    render("28_ecommerce_home", 0xF5F6F8, wxml, wxss, data);
}

// 29. 优惠券弹窗（活动促销弹层）
fn coupon_popup() {
    let wxml = r##"
    <view class="mask">
        <view class="pop col">
            <view class="head col">
                <text class="htitle">新人专享福利</text>
                <text class="hsub">超值优惠券限时领取</text>
            </view>
            <view class="body col">
                <view class="cp row" wx:for="{{coupons}}" wx:key="t">
                    <view class="cp-l col">
                        <view class="row bl"><text class="cur">¥</text><text class="amt">{{item.amt}}</text></view>
                        <text class="cond">满{{item.cond}}可用</text>
                    </view>
                    <view class="cp-m col grow">
                        <text class="cp-name">{{item.name}}</text>
                        <text class="cp-time">有效期至 {{item.exp}}</text>
                    </view>
                    <text class="cp-btn">立即领取</text>
                </view>
            </view>
            <view class="allbtn">一键领取全部</view>
            <view class="close">X</view>
        </view>
    </view>"##;
    let wxss = r##"
    .mask{ height:1334rpx; background-color:rgba(0,0,0,0.6); display:flex; flex-direction:column; justify-content:center; align-items:center; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .col{ display:flex; flex-direction:column; }
    .grow{ flex:1; }
    .bl{ align-items:baseline; }
    .pop{ width:600rpx; background-color:#ffffff; border-radius:28rpx; display:flex; flex-direction:column; align-items:center; padding-bottom:36rpx; overflow:hidden; }
    .head{ width:600rpx; background-color:#ff3b30; display:flex; flex-direction:column; align-items:center; padding:44rpx 0 36rpx 0; }
    .htitle{ font-size:44rpx; color:#ffffff; font-weight:bold; }
    .hsub{ font-size:26rpx; color:#ffe0dd; margin-top:12rpx; }
    .body{ display:flex; flex-direction:column; width:540rpx; margin-top:28rpx; }
    .cp{ display:flex; flex-direction:row; align-items:center; background-color:#fff6f5; border-radius:16rpx; padding:24rpx; margin-bottom:20rpx; }
    .cp-l{ display:flex; flex-direction:column; align-items:center; width:170rpx; }
    .cur{ font-size:28rpx; color:#ff3b30; }
    .amt{ font-size:64rpx; color:#ff3b30; font-weight:bold; }
    .cond{ font-size:22rpx; color:#ff7a6b; }
    .cp-m{ display:flex; flex-direction:column; flex:1; padding-left:24rpx; }
    .cp-name{ font-size:30rpx; color:#1a1a1a; font-weight:bold; }
    .cp-time{ font-size:22rpx; color:#999999; margin-top:10rpx; }
    .cp-btn{ background-color:#ff3b30; color:#ffffff; font-size:26rpx; padding:14rpx 24rpx; border-radius:28rpx; }
    .allbtn{ width:500rpx; background-color:#ff3b30; color:#ffffff; text-align:center; font-size:32rpx; font-weight:bold; padding:26rpx; border-radius:48rpx; margin-top:16rpx; }
    .close{ width:56rpx; height:56rpx; border-radius:28rpx; background-color:rgba(255,255,255,0.25); color:#ffffff; text-align:center; font-size:32rpx; margin-top:32rpx; }
    "##;
    let data = json!({"coupons":[
        {"amt":"20","cond":"100","name":"全场通用券","exp":"07-31"},
        {"amt":"50","cond":"299","name":"数码专享券","exp":"07-25"},
        {"amt":"88","cond":"499","name":"品牌大额券","exp":"08-05"}
    ]});
    render_screen("29_coupon_popup", 0xEDEDED, wxml, wxss, data);
}
