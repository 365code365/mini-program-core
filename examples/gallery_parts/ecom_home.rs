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

