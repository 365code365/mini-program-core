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
    .fi:last-child{ border-bottom:0; }
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
            <image class="tile" wx:for="{{tiles}}" wx:key="*this" src="{{item}}" mode="aspectFill" />
        </view>
    </view>"#;
    let wxss = r#"
    .page{ padding:24rpx; }
    .head{ font-size:34rpx; font-weight:bold; color:#1a1a1a; margin-bottom:20rpx; }
    .grid{ display:flex; flex-direction:row; flex-wrap:wrap; }
    .tile{ width:220rpx; height:220rpx; border-radius:16rpx; margin:0 12rpx 12rpx 0; }
    "#;
    let data = json!({"tiles":[
        "doc/gallery/assets/album.jpg","doc/gallery/assets/banner.jpg","doc/gallery/assets/p_coffee.jpg",
        "doc/gallery/assets/p_cake.jpg","doc/gallery/assets/p_sneakers.jpg","doc/gallery/assets/p_backpack.jpg",
        "doc/gallery/assets/p_lipstick.jpg","doc/gallery/assets/p_thermos.jpg","doc/gallery/assets/p_watch.jpg"
    ]});
    render("12_gallery", 0xFFFFFF, wxml, wxss, data);
}

