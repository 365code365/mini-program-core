// 30. 底部 TabBar（自定义 tabBar，选中态高亮）
fn tabbar() {
    let wxml = r##"
    <view class="wrap">
        <view class="nav"><text class="nav-t">首页</text></view>
        <view class="content">
            <image class="hero" src="doc/gallery/assets/banner.jpg" mode="aspectFill" />
            <view class="row cards">
                <view class="mini col"><image class="mi" src="doc/gallery/assets/p_earbuds.jpg" mode="aspectFill" /><text class="mt">蓝牙耳机</text></view>
                <view class="mini col"><image class="mi" src="doc/gallery/assets/p_watch.jpg" mode="aspectFill" /><text class="mt">运动手表</text></view>
            </view>
        </view>
        <view class="tabbar row">
            <view class="tab col">
                <image class="ti" src="doc/gallery/assets/icons/tab_home_on.png" mode="aspectFit" />
                <text class="tl on">首页</text>
            </view>
            <view class="tab col">
                <image class="ti" src="doc/gallery/assets/icons/tab_cat.png" mode="aspectFit" />
                <text class="tl">分类</text>
            </view>
            <view class="tab col">
                <view class="ti-wrap"><image class="ti" src="doc/gallery/assets/icons/tab_cart.png" mode="aspectFit" /><view class="badge">3</view></view>
                <text class="tl">购物车</text>
            </view>
            <view class="tab col">
                <image class="ti" src="doc/gallery/assets/icons/tab_user.png" mode="aspectFit" />
                <text class="tl">我的</text>
            </view>
        </view>
    </view>"##;
    let wxss = r##"
    .wrap{ height:1334rpx; background-color:#f5f6f8; display:flex; flex-direction:column; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .col{ display:flex; flex-direction:column; align-items:center; }
    .nav{ height:88rpx; background-color:#ffffff; display:flex; flex-direction:row; justify-content:center; align-items:center; border-bottom:1rpx solid #eeeeee; }
    .nav-t{ font-size:34rpx; font-weight:bold; color:#1a1a1a; }
    .content{ flex:1; padding:24rpx; }
    .hero{ width:702rpx; height:300rpx; border-radius:20rpx; }
    .cards{ display:flex; flex-direction:row; margin-top:24rpx; }
    .mini{ flex:1; background-color:#ffffff; border-radius:16rpx; padding:20rpx; margin-right:20rpx; display:flex; flex-direction:column; align-items:center; }
    .mi{ width:200rpx; height:200rpx; border-radius:12rpx; }
    .mt{ font-size:26rpx; color:#333; margin-top:14rpx; }
    .tabbar{ height:110rpx; background-color:#ffffff; display:flex; flex-direction:row; border-top:1rpx solid #eeeeee; }
    .tab{ flex:1; display:flex; flex-direction:column; align-items:center; justify-content:center; padding-top:12rpx; }
    .ti{ width:52rpx; height:52rpx; }
    .ti-wrap{ width:52rpx; height:52rpx; position:relative; }
    .badge{ position:absolute; top:-10rpx; right:-18rpx; background-color:#ff3b30; color:#ffffff; font-size:20rpx; min-width:32rpx; height:32rpx; border-radius:16rpx; padding:0 6rpx; display:flex; flex-direction:row; align-items:center; justify-content:center; }
    .tl{ font-size:22rpx; color:#999999; margin-top:6rpx; }
    .on{ color:#07c160; }
    "##;
    render_screen("30_tabbar", 0xF5F6F8, wxml, wxss, json!({}));
}

// 31. 顶部自定义搜索栏（自定义导航栏）
fn search_nav() {
    let wxml = r##"
    <view class="wrap">
        <view class="navbar">
            <view class="statusbar"></view>
            <view class="barrow row">
                <text class="back">〈</text>
                <view class="searchbox row">
                    <image class="sic" src="doc/gallery/assets/icons/search.png" mode="aspectFit" />
                    <text class="sph">搜索商品 / 品牌 / 店铺</text>
                    <view class="sbtn">搜索</view>
                </view>
                <text class="more">···</text>
            </view>
            <view class="tabs row">
                <text class="tab on">综合</text>
                <text class="tab">销量</text>
                <text class="tab">新品</text>
                <text class="tab">价格</text>
            </view>
        </view>
        <view class="body">
            <view class="row item" wx:for="{{list}}" wx:key="n">
                <image class="thumb" src="{{item.img}}" mode="aspectFill" />
                <view class="col grow info">
                    <text class="nm">{{item.n}}</text>
                    <text class="pp">¥{{item.p}}</text>
                </view>
            </view>
        </view>
    </view>"##;
    let wxss = r##"
    .wrap{ background-color:#f5f6f8; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .col{ display:flex; flex-direction:column; }
    .grow{ flex:1; }
    .navbar{ background-color:#07c160; padding:0 24rpx 20rpx 24rpx; }
    .statusbar{ height:44rpx; }
    .barrow{ display:flex; flex-direction:row; align-items:center; }
    .back{ font-size:40rpx; color:#ffffff; margin-right:16rpx; }
    .searchbox{ flex:1; background-color:#ffffff; border-radius:36rpx; padding:12rpx 12rpx 12rpx 24rpx; display:flex; flex-direction:row; align-items:center; }
    .sic{ width:32rpx; height:32rpx; margin-right:12rpx; }
    .sph{ flex:1; font-size:26rpx; color:#bbbbbb; }
    .sbtn{ background-color:#07c160; color:#ffffff; font-size:26rpx; padding:10rpx 28rpx; border-radius:30rpx; }
    .more{ font-size:36rpx; color:#ffffff; margin-left:16rpx; }
    .tabs{ display:flex; flex-direction:row; margin-top:24rpx; }
    .tab{ font-size:28rpx; color:#eafff2; margin-right:44rpx; }
    .tab.on{ color:#ffffff; font-weight:bold; }
    .body{ padding:24rpx; }
    .item{ display:flex; flex-direction:row; align-items:center; background-color:#ffffff; border-radius:16rpx; padding:20rpx; margin-bottom:20rpx; }
    .thumb{ width:150rpx; height:150rpx; border-radius:12rpx; }
    .info{ margin-left:24rpx; display:flex; flex-direction:column; flex:1; }
    .nm{ font-size:30rpx; color:#1a1a1a; }
    .pp{ font-size:34rpx; color:#ff5000; font-weight:bold; margin-top:20rpx; }
    "##;
    render("31_search_nav", 0xF5F6F8, wxml, wxss, json!({"list":[
        {"n":"无线蓝牙耳机 主动降噪","p":"199","img":"doc/gallery/assets/p_earbuds.jpg"},
        {"n":"智能运动手表 血氧监测","p":"599","img":"doc/gallery/assets/p_watch.jpg"},
        {"n":"复古潮流运动鞋","p":"399","img":"doc/gallery/assets/p_sneakers.jpg"}
    ]}));
}

// 32. 输入事件（bindinput / bindfocus / bindconfirm 等）
fn input_events() {
    let wxml = r##"
    <view class="page">
        <text class="h">表单输入与事件</text>
        <view class="card">
            <view class="fi col">
                <text class="lb">昵称 · bindinput 实时同步</text>
                <view class="ipt"><input value="小明" placeholder="请输入昵称" bindinput="onNick" /></view>
                <text class="hint">当前输入：小明（4/20）</text>
            </view>
            <view class="fi col">
                <text class="lb">手机号 · type=number bindconfirm</text>
                <view class="ipt"><input type="number" value="13800138000" placeholder="请输入手机号" bindconfirm="onPhone" /></view>
            </view>
            <view class="fi col">
                <text class="lb">密码 · password 聚焦态（绿框 + 光标）</text>
                <view class="ipt focus"><input password="true" value="secret" focus="true" bindfocus="onFocus" bindblur="onBlur" /></view>
            </view>
            <view class="fi col">
                <text class="lb">搜索 · confirm-type=search</text>
                <view class="ipt"><input placeholder="搜索关键词" confirm-type="search" bindconfirm="onSearch" /></view>
            </view>
            <view class="fi col">
                <text class="lb">多行备注 · textarea</text>
                <view class="ipt ta"><textarea value="点击输入多行备注内容..." /></view>
            </view>
        </view>
        <view class="log">
            <text class="log-t">事件日志</text>
            <text class="log-i">onFocus -> 密码框获得焦点</text>
            <text class="log-i">onNick -> value=小明 cursor=2</text>
            <text class="log-i">onConfirm -> 触发搜索</text>
        </view>
    </view>"##;
    let wxss = r##"
    .page{ padding:24rpx; }
    .h{ font-size:34rpx; font-weight:bold; color:#1a1a1a; margin-bottom:20rpx; }
    .card{ background-color:#ffffff; border-radius:20rpx; padding:12rpx 28rpx; }
    .col{ display:flex; flex-direction:column; }
    .fi{ padding:24rpx 0; border-bottom:1rpx solid #f2f2f2; display:flex; flex-direction:column; }
    .fi:last-child{ border-bottom:0; }
    .lb{ font-size:26rpx; color:#888; margin-bottom:16rpx; }
    .ipt{ background-color:#f7f8fa; border-radius:12rpx; }
    .ipt.focus{ border:2rpx solid #07c160; background-color:#ffffff; }
    .ta{ height:140rpx; }
    .hint{ font-size:24rpx; color:#07c160; margin-top:12rpx; }
    .log{ background-color:#1e1e28; border-radius:20rpx; padding:28rpx; margin-top:24rpx; display:flex; flex-direction:column; }
    .log-t{ font-size:28rpx; color:#ffffff; font-weight:bold; margin-bottom:16rpx; }
    .log-i{ font-size:24rpx; color:#7ee2a8; margin-top:10rpx; }
    "##;
    render("32_input_events", 0xF5F6F8, wxml, wxss, json!({}));
}

