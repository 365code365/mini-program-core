// CSS 动画 / 变换 / 表单开关：验证原生端 @keyframes 求值、transform 离屏合成、
// 以及 switch 等表单控件与微信外观的一致性。

/// CSS `@keyframes` 逐帧快照：同一份 WXML/WXSS 在 4 个时刻各截一张，
/// 连起来看就是动画的推进过程（旋转 / 脉冲 / 弹跳 / 淡入）。
fn css_animation_frames() {
    let wxml = r##"
    <view class="page">
      <view class="card">
        <text class="title">CSS @keyframes · 原生端逐帧求值</text>
        <text class="sub">同一份样式在不同时刻的渲染结果</text>
      </view>
      <view class="card">
        <view class="row around">
          <view class="cell">
            <view class="loader"></view>
            <text class="label">旋转 spin</text>
          </view>
          <view class="cell">
            <view class="pulse"></view>
            <text class="label">脉冲 pulse</text>
          </view>
          <view class="cell">
            <view class="bounce"><text class="bounce-tx">HOT</text></view>
            <text class="label">弹跳 bounce</text>
          </view>
        </view>
      </view>
      <view class="card">
        <view class="fade"><text class="fade-tx">进入淡入 fade-in（opacity + translateY）</text></view>
        <view class="bar-wrap">
          <view class="bar"></view>
        </view>
        <text class="label">进度条 · 宽度由 transform: scaleX 驱动</text>
      </view>
      <view class="card">
        <view class="row around">
          <view class="cell"><view class="swing">↔</view><text class="label">往复 alternate</text></view>
          <view class="cell"><view class="blink"></view><text class="label">呼吸 opacity</text></view>
          <view class="cell"><view class="spin-slow"></view><text class="label">慢转 steps(8)</text></view>
        </view>
      </view>
    </view>
    "##;

    let wxss = r##"
    @keyframes spin { to { transform: rotate(360deg); } }
    @keyframes pulse { 0%,100% { transform: scale(1); opacity: 1; } 50% { transform: scale(1.35); opacity: .55; } }
    @keyframes bounce { 0%,100% { transform: translateY(0); } 50% { transform: translateY(-24rpx); } }
    @keyframes fadeIn { from { opacity: 0; transform: translateY(24rpx); } to { opacity: 1; transform: translateY(0); } }
    @keyframes grow { from { transform: scaleX(0.05); } to { transform: scaleX(1); } }
    @keyframes swing { 0% { transform: translateX(-20rpx); } 100% { transform: translateX(20rpx); } }
    @keyframes breathe { 0%,100% { opacity: .25; } 50% { opacity: 1; } }
    .page{ padding:24rpx; }
    .card{ background-color:#ffffff; border-radius:20rpx; padding:28rpx; margin-bottom:20rpx; }
    .title{ font-size:32rpx; color:#1a1a1a; font-weight:bold; }
    .sub{ font-size:24rpx; color:#999999; margin-top:8rpx; }
    .row{ display:flex; flex-direction:row; }
    .around{ justify-content:space-around; }
    .cell{ display:flex; flex-direction:column; align-items:center; }
    .label{ font-size:22rpx; color:#999999; margin-top:16rpx; }
    .loader{ width:72rpx; height:72rpx; border:8rpx solid #f0f0f0; border-top-color:#FF6B35; border-radius:50%;
             animation: spin 0.9s linear infinite; }
    .pulse{ width:72rpx; height:72rpx; border-radius:50%; background-color:#07c160;
            animation: pulse 1.2s ease-in-out infinite; }
    .bounce{ width:72rpx; height:72rpx; border-radius:16rpx; background-color:#FF3B30;
             display:flex; align-items:center; justify-content:center;
             animation: bounce 0.8s ease-in-out infinite; }
    .bounce-tx{ color:#ffffff; font-size:24rpx; font-weight:bold; }
    .fade{ padding:24rpx; border-radius:12rpx; background-color:#FF6B35; animation: fadeIn 1.2s ease-out both; }
    .fade-tx{ color:#ffffff; font-size:26rpx; }
    .bar-wrap{ height:16rpx; border-radius:8rpx; background-color:#f0f0f0; margin-top:24rpx; overflow:hidden; }
    .bar{ height:16rpx; border-radius:8rpx; background-color:#4A90D9; animation: grow 1.2s ease-out both; }
    .swing{ width:72rpx; height:72rpx; border-radius:16rpx; background-color:#722ed1; color:#ffffff;
            font-size:32rpx; text-align:center; animation: swing 1s ease-in-out infinite alternate; }
    .blink{ width:72rpx; height:72rpx; border-radius:50%; background-color:#13c2c2;
            animation: breathe 1.4s linear infinite; }
    .spin-slow{ width:72rpx; height:72rpx; border:8rpx solid #f0f0f0; border-top-color:#52C41A; border-radius:50%;
                animation: spin 1.6s steps(8) infinite; }
    "##;

    for (i, t) in [0.0f32, 0.3, 0.6, 0.9].iter().enumerate() {
        render_at_time(
            &format!("58_css_anim_frame{}", i + 1),
            0xF5F6F8,
            wxml,
            wxss,
            json!({}),
            *t,
        );
    }
}

/// transform 展示：平移直接偏移，缩放/旋转/倾斜走离屏仿射合成（整棵子树都被变换）。
fn transform_showcase() {
    let wxml = r##"
    <view class="page">
      <view class="card">
        <text class="title">transform · 作用于整棵子树</text>
        <text class="sub">平移直接偏移；缩放 / 旋转 / 倾斜走离屏仿射合成</text>
      </view>
      <view class="grid">
        <view class="box t-none"><text class="bx">原始</text></view>
        <view class="box t-move"><text class="bx">translate</text></view>
        <view class="box t-scale"><text class="bx">scale 1.2</text></view>
        <view class="box t-rot"><text class="bx">rotate 15°</text></view>
        <view class="box t-skew"><text class="bx">skewX 12°</text></view>
        <view class="box t-mix"><text class="bx">组合变换</text></view>
      </view>
      <view class="card">
        <view class="ticket t-rot8">
          <text class="tk-amt">¥50</text>
          <text class="tk-tip">满 299 元可用 · 旋转卡片</text>
        </view>
      </view>
    </view>
    "##;
    let wxss = r##"
    .page{ padding:24rpx; }
    .card{ background-color:#ffffff; border-radius:20rpx; padding:28rpx; margin-bottom:20rpx; }
    .title{ font-size:32rpx; color:#1a1a1a; font-weight:bold; }
    .sub{ font-size:24rpx; color:#999999; margin-top:8rpx; }
    .grid{ display:flex; flex-direction:row; flex-wrap:wrap; }
    .box{ width:33%; height:180rpx; display:flex; align-items:center; justify-content:center; }
    .bx{ background-color:#4A90D9; color:#ffffff; font-size:24rpx; padding:16rpx 20rpx; border-radius:12rpx; }
    .t-move{ transform: translate(16rpx, -12rpx); }
    .t-scale{ transform: scale(1.2); }
    .t-rot{ transform: rotate(15deg); }
    .t-skew{ transform: skewX(12deg); }
    .t-mix{ transform: rotate(-8deg) scale(1.1); }
    .ticket{ background-color:#FF6B35; border-radius:16rpx; padding:28rpx; display:flex; flex-direction:column; }
    .t-rot8{ transform: rotate(-6deg); }
    .tk-amt{ color:#ffffff; font-size:52rpx; font-weight:bold; }
    .tk-tip{ color:#ffffff; font-size:24rpx; margin-top:8rpx; }
    "##;
    render("59_transform", 0xF5F6F8, wxml, wxss, json!({}));
}

/// 表单控件外观：switch / checkbox / radio / slider / progress 各状态，
/// 对齐微信（weui）度量，便于和编译出的 H5 逐像素比对。
fn form_controls() {
    let wxml = r##"
    <view class="page">
      <view class="card">
        <text class="title">表单控件 · 微信度量</text>
        <text class="sub">switch 52x32 · 滑块直径 30 · 关态浅灰边框</text>
      </view>
      <view class="card">
        <view class="line between">
          <text class="lb">开关 · 关</text>
          <switch checked="{{false}}"></switch>
        </view>
        <view class="line between">
          <text class="lb">开关 · 开</text>
          <switch checked="{{true}}"></switch>
        </view>
        <view class="line between">
          <text class="lb">开关 · 自定义色</text>
          <switch checked="{{true}}" color="#FF6B35"></switch>
        </view>
        <view class="line between">
          <text class="lb">开关 · 禁用</text>
          <switch checked="{{true}}" disabled="{{true}}"></switch>
        </view>
      </view>
      <view class="card">
        <view class="line"><checkbox checked="{{true}}" color="#07c160"></checkbox><text class="lb ml">已勾选</text></view>
        <view class="line"><checkbox checked="{{false}}"></checkbox><text class="lb ml">未勾选</text></view>
        <view class="line"><radio checked="{{true}}"></radio><text class="lb ml">单选选中</text></view>
        <view class="line"><radio checked="{{false}}"></radio><text class="lb ml">单选未选</text></view>
      </view>
      <view class="card">
        <text class="lb">滑块 · 60%</text>
        <slider value="60" show-value="{{true}}" activeColor="#07c160"></slider>
        <text class="lb">进度 · 35%</text>
        <progress percent="35" show-info="{{true}}"></progress>
      </view>
      <view class="card">
        <view class="line between">
          <button size="mini">次要</button>
          <button size="mini" type="primary">主要</button>
          <button size="mini" type="warn">警告</button>
        </view>
        <button type="primary">整行主按钮</button>
      </view>
    </view>
    "##;
    let wxss = r##"
    .page{ padding:24rpx; }
    .card{ background-color:#ffffff; border-radius:20rpx; padding:28rpx; margin-bottom:20rpx; }
    .title{ font-size:32rpx; color:#1a1a1a; font-weight:bold; }
    .sub{ font-size:24rpx; color:#999999; margin-top:8rpx; }
    .line{ display:flex; flex-direction:row; align-items:center; padding:16rpx 0; }
    .between{ justify-content:space-between; }
    .lb{ font-size:28rpx; color:#333333; }
    .ml{ margin-left:20rpx; }
    "##;
    render("60_form_controls", 0xF5F6F8, wxml, wxss, json!({}));
}

/// 彩色 emoji：位图字形（Apple sbix）与文字混排，验证基线、前进宽度与缩放。
fn emoji_text() {
    let wxml = r##"
    <view class="page">
      <view class="card">
        <text class="title">彩色 Emoji · 位图字形</text>
        <text class="sub">sbix 表按字号取图并缓存，与文字同基线混排</text>
      </view>
      <view class="card">
        <text class="big">🛒 购物车 · ✅ 已完成</text>
        <text class="mid">📦 我的订单　💳 支付方式　🎁 优惠券</text>
        <text class="small">🔥 热销榜 · 📰 资讯 · 📂 分类 · 🔍 搜索</text>
      </view>
      <view class="card">
        <view class="line"><text class="ico">🏠</text><text class="lb">首页</text></view>
        <view class="line"><text class="ico">📺</text><text class="lb">视频</text></view>
        <view class="line"><text class="ico">👤</text><text class="lb">我的</text></view>
      </view>
      <view class="card">
        <text class="quote">混排换行测试：这一段里 🎨 表情与中文、English words 混在一起，用来检查换行位置和行高是否稳定 ✨。</text>
      </view>
    </view>
    "##;
    let wxss = r##"
    .page{ padding:24rpx; }
    .card{ background-color:#ffffff; border-radius:20rpx; padding:28rpx; margin-bottom:20rpx; }
    .title{ font-size:32rpx; color:#1a1a1a; font-weight:bold; }
    .sub{ font-size:24rpx; color:#999999; margin-top:8rpx; }
    .big{ font-size:40rpx; color:#1a1a1a; display:block; }
    .mid{ font-size:30rpx; color:#333333; display:block; margin-top:20rpx; }
    .small{ font-size:24rpx; color:#666666; display:block; margin-top:20rpx; }
    .line{ display:flex; flex-direction:row; align-items:center; padding:12rpx 0; }
    .ico{ font-size:44rpx; margin-right:20rpx; }
    .lb{ font-size:28rpx; color:#333333; }
    .quote{ font-size:26rpx; color:#555555; line-height:1.8; }
    "##;
    render("61_emoji_text", 0xF5F6F8, wxml, wxss, json!({}));
}
