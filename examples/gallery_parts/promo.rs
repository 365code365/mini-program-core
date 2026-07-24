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
    .pn{ width:311rpx; font-size:28rpx; color:#1a1a1a; line-height:40rpx; }
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
        </view>
        <view class="close">X</view>
    </view>"##;
    let wxss = r##"
    .mask{ height:1334rpx; background-color:rgba(0,0,0,0.6); display:flex; flex-direction:column; justify-content:center; align-items:center; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .col{ display:flex; flex-direction:column; }
    .grow{ flex:1; }
    .bl{ align-items:baseline; }
    .pop{ width:600rpx; background-color:#ffffff; border-radius:28rpx; display:flex; flex-direction:column; align-items:center; padding-bottom:36rpx; overflow:hidden; }
    .head{ width:600rpx; background:linear-gradient(135deg, #ff5f6d, #ff3b30, #ff9500); display:flex; flex-direction:column; align-items:center; padding:44rpx 0 36rpx 0; }
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
    .close{ width:64rpx; height:64rpx; border-radius:32rpx; border:2rpx solid rgba(255,255,255,0.8); color:#ffffff; text-align:center; font-size:34rpx; margin-top:44rpx; }
    "##;
    let data = json!({"coupons":[
        {"amt":"20","cond":"100","name":"全场通用券","exp":"07-31"},
        {"amt":"50","cond":"299","name":"数码专享券","exp":"07-25"},
        {"amt":"88","cond":"499","name":"品牌大额券","exp":"08-05"}
    ]});
    render_screen("29_coupon_popup", 0xEDEDED, wxml, wxss, data);
}

