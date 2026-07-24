// 1. 登录
fn login() {
    let wxml = r#"
    <view class="page col">
        <image class="logo" src="doc/gallery/assets/logo.jpg" mode="aspectFill" />
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
    .page{ padding:80rpx 48rpx; display:flex; flex-direction:column; }
    .col{ display:flex; flex-direction:column; }
    .logo{ width:150rpx; height:150rpx; border-radius:34rpx; margin-bottom:36rpx; align-self:center; }
    .h1{ font-size:48rpx; font-weight:bold; color:#1a1a1a; text-align:center; }
    .tip{ font-size:26rpx; color:#999; text-align:center; margin-top:12rpx; margin-bottom:70rpx; }
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
    let wxml = r##"
    <view class="wrap">
        <image class="hero" src="doc/gallery/assets/p_phone.jpg" mode="aspectFill" />
        <view class="thumbs row">
            <image class="tb tbon" src="doc/gallery/assets/p_phone.jpg" mode="aspectFill" />
            <image class="tb" src="doc/gallery/assets/p_phone2.jpg" mode="aspectFill" />
            <image class="tb" src="doc/gallery/assets/p_earbuds.jpg" mode="aspectFill" />
            <image class="tb" src="doc/gallery/assets/p_watch.jpg" mode="aspectFill" />
        </view>
        <view class="body">
            <view class="row bl">
                <text class="cur">¥</text><text class="price">3999</text>
                <text class="old">¥4299</text>
            </view>
            <text class="name">全面屏旗舰手机 12+256GB 幻夜黑</text>
            <view class="tags row">
                <text class="tag">顺丰包邮</text>
                <text class="tag">7天无理由</text>
                <text class="tag">正品保障</text>
            </view>
            <view class="spec row between">
                <text class="spec-t">已选：幻夜黑 / 12+256GB</text>
                <text class="spec-a">></text>
            </view>
            <view class="param">
                <view class="row between p"><text class="k">品牌</text><text class="v">MiniTech</text></view>
                <view class="row between p"><text class="k">屏幕</text><text class="v">6.7英寸 OLED</text></view>
                <view class="row between p"><text class="k">电池</text><text class="v">5000mAh</text></view>
            </view>
        </view>
        <view class="bar row">
            <view class="ib col"><icon type="chat" size="24" color="#666666" /><text class="ibt">客服</text></view>
            <view class="ib col"><icon type="star-o" size="24" color="#666666" /><text class="ibt">收藏</text></view>
            <view class="cart">加入购物车</view>
            <view class="now">立即购买</view>
        </view>
    </view>"##;
    let wxss = r#"
    .hero{ width:750rpx; height:560rpx; }
    .thumbs{ display:flex; flex-direction:row; padding:20rpx 28rpx 0 28rpx; }
    .tb{ width:120rpx; height:120rpx; border-radius:12rpx; margin-right:18rpx; border:2rpx solid #eeeeee; }
    .tbon{ border:4rpx solid #ff5000; }
    .body{ padding:28rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .col{ display:flex; flex-direction:column; align-items:center; }
    .between{ justify-content:space-between; }
    .bl{ align-items:baseline; }
    .cur{ font-size:32rpx; color:#ff5000; font-weight:bold; white-space:nowrap; }
    .price{ font-size:56rpx; color:#ff5000; font-weight:bold; white-space:nowrap; }
    .old{ font-size:28rpx; color:#bbb; text-decoration:line-through; margin-left:16rpx; white-space:nowrap; }
    .name{ font-size:34rpx; color:#1a1a1a; font-weight:bold; margin-top:16rpx; line-height:48rpx; }
    .tags{ display:flex; flex-direction:row; margin-top:20rpx; }
    .tag{ background-color:#fff0e8; color:#ff5000; font-size:22rpx; padding:8rpx 14rpx; border-radius:8rpx; margin-right:12rpx; }
    .spec{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; background-color:#f7f8fa; border-radius:16rpx; padding:26rpx 24rpx; margin-top:24rpx; }
    .spec-t{ font-size:28rpx; color:#333; }
    .spec-a{ font-size:28rpx; color:#bbb; }
    .param{ margin-top:12rpx; }
    .p{ padding:20rpx 0; border-bottom:1rpx solid #f2f2f2; }
    .p:last-child{ border-bottom:0; }
    .k{ font-size:28rpx; color:#999; white-space:nowrap; }
    .v{ font-size:28rpx; color:#333; white-space:nowrap; }
    .bar{ display:flex; flex-direction:row; align-items:center; padding:16rpx 24rpx; }
    .ib{ display:flex; flex-direction:column; align-items:center; margin-right:28rpx; }
    .ibt{ font-size:20rpx; color:#666; margin-top:4rpx; }
    .cart{ flex:1; background-color:#ffb400; color:#fff; text-align:center; font-size:30rpx; padding:24rpx; border-radius:44rpx 0 0 44rpx; }
    .now{ flex:1; background-color:#ff5000; color:#fff; text-align:center; font-size:30rpx; padding:24rpx; border-radius:0 44rpx 44rpx 0; }
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
            <view class="mi row between" wx:for="{{menu}}" wx:key="n">
                <view class="row"><image class="mi-ico" src="{{item.icon}}" mode="aspectFit"></image><text class="mt">{{item.n}}</text></view>
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
    .mi:last-child{ border-bottom:0; }
    .mi-ico{ width:52rpx; height:52rpx; margin-right:22rpx; }
    .mt{ font-size:30rpx; color:#333; }
    .arrow{ font-size:30rpx; color:#ccc; }
    "#;
    let data = json!({"menu":[
        {"n":"我的订单","icon":"doc/gallery/assets/icons/mi_order.png"},
        {"n":"收货地址","icon":"doc/gallery/assets/icons/mi_address.png"},
        {"n":"账户安全","icon":"doc/gallery/assets/icons/mi_security.png"},
        {"n":"消息通知","icon":"doc/gallery/assets/icons/mi_notify.png"},
        {"n":"帮助中心","icon":"doc/gallery/assets/icons/mi_help.png"},
        {"n":"关于我们","icon":"doc/gallery/assets/icons/mi_about.png"}
    ]});
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
    .item:last-child{ border-bottom:0; }
    .t{ font-size:30rpx; color:#333; }
    .v{ font-size:28rpx; color:#999; }
    .logout{ background-color:#fff; color:#ff3b30; text-align:center; font-size:30rpx; padding:28rpx; border-radius:20rpx; margin-top:40rpx; }
    "#;
    render("06_settings", 0xF5F6F8, wxml, wxss, json!({}));
}

