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

