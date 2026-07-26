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
    .brow:last-child{ border-bottom:0; }
    .bt{ font-size:30rpx; color:#333; }
    .badge{ background-color:#ff3b30; color:#fff; font-size:22rpx; padding:6rpx 14rpx; border-radius:20rpx; }
    .dotb{ width:24rpx; height:24rpx; border-radius:12rpx; }
    .dg{ background-color:#07c160; }
    .dgy{ background-color:#c7c7cc; }
    "#;
    render("18_tags", 0xF5F6F8, wxml, wxss, json!({}));
}


// 35. GIF 动图（逐帧推进）
//
// 原生渲染器会解码 GIF 的全部帧并按各帧延时循环播放；这里连续渲染 4 张，
// 每张之间等待若干毫秒，输出的图片可直观看到帧在推进（不是静态首帧）。
fn gif_animation() {
    let wxml = r##"
    <view class="page">
        <text class="h">GIF 动图 · 原生逐帧播放</text>
        <view class="card">
            <view class="row">
                <image class="gif" src="sample/sample-app/assets/loading.gif" mode="scaleToFill"></image>
                <view class="info">
                    <text class="t">loading.gif</text>
                    <text class="d">8 帧 · 每帧 120ms · 循环播放</text>
                    <text class="d">原生：解码全部帧后按时间取帧</text>
                    <text class="d">HTML：交由浏览器原生播放</text>
                </view>
            </view>
        </view>
        <text class="h">不同尺寸与裁剪模式</text>
        <view class="card row around">
            <image class="gif-sm" src="sample/sample-app/assets/loading.gif" mode="aspectFit"></image>
            <image class="gif-md" src="sample/sample-app/assets/loading.gif" mode="aspectFill"></image>
            <image class="gif-round" src="sample/sample-app/assets/loading.gif" mode="scaleToFill"></image>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ padding:24rpx; }
    .h{ font-size:28rpx; color:#999; margin:20rpx 0 14rpx 8rpx; }
    .card{ background-color:#fff; border-radius:20rpx; padding:28rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .around{ justify-content:space-around; }
    .gif{ width:200rpx; height:200rpx; border-radius:16rpx; }
    .info{ flex:1; margin-left:28rpx; }
    .t{ display:block; font-size:30rpx; color:#333; font-weight:bold; }
    .d{ display:block; font-size:24rpx; color:#999; margin-top:10rpx; }
    .gif-sm{ width:120rpx; height:120rpx; }
    .gif-md{ width:160rpx; height:120rpx; border-radius:12rpx; }
    .gif-round{ width:140rpx; height:140rpx; border-radius:70rpx; }
    "#;
    // 连续 4 帧快照，间隔 240ms（跨越两帧延时），可见动画推进
    for shot in 0..4 {
        if shot > 0 {
            std::thread::sleep(std::time::Duration::from_millis(240));
        }
        render(&format!("35_gif_frame{}", shot + 1), 0xF5F6F8, wxml, wxss, json!({}));
    }
}
