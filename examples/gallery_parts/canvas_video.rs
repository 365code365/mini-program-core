// 33. video 组件（本地视频，控制条）
fn video_player() {
    let wxml = r##"
    <view class="page">
        <text class="h">视频播放器</text>
        <video class="vid" src="doc/videos/video.mp4" controls="true" show-center-play-btn="true" object-fit="cover"></video>
        <view class="meta">
            <text class="title">示例视频 · video.mp4</text>
            <text class="sub">支持 autoplay / loop / muted / controls；桌面端默认走 macOS 音频接口播放声音</text>
        </view>
        <view class="row acts">
            <view class="act">播放</view>
            <view class="act ghost">循环</view>
            <view class="act ghost">静音</view>
        </view>
    </view>"##;
    let wxss = r##"
    .page{ padding:24rpx; }
    .h{ font-size:34rpx; font-weight:bold; color:#1a1a1a; margin-bottom:20rpx; }
    .vid{ width:702rpx; height:395rpx; border-radius:16rpx; background-color:#000000; }
    .meta{ display:flex; flex-direction:column; margin-top:24rpx; }
    .title{ font-size:30rpx; color:#1a1a1a; font-weight:bold; }
    .sub{ display:block; font-size:24rpx; color:#999; margin-top:12rpx; line-height:36rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .acts{ display:flex; flex-direction:row; margin-top:28rpx; }
    .act{ background-color:#07c160; color:#ffffff; font-size:28rpx; padding:18rpx 36rpx; border-radius:40rpx; margin-right:20rpx; }
    .ghost{ background-color:#ffffff; color:#07c160; border:2rpx solid #07c160; }
    "##;
    render("33_video", 0xF5F6F8, wxml, wxss, json!({}));
}

// 34. Canvas 2D 绘图（JS 风格命令：矩形/路径/圆弧/文本/变换）
fn canvas_demo() {
    // 画布上下文尺寸与元素物理像素一致（600rpx -> 600 物理像素 @SCALE2）
    ensure_canvas_context("demo", 600, 600);
    let mut cmds: Vec<Value> = Vec::new();
    // 白底
    cmds.push(json!({"type":"setFillStyle","color":"#ffffff"}));
    cmds.push(json!({"type":"fillRect","x":0,"y":0,"width":600,"height":600}));
    // 标题
    cmds.push(json!({"type":"setFillStyle","color":"#1a1a1a"}));
    cmds.push(json!({"type":"setFontSize","size":34}));
    cmds.push(json!({"type":"fillText","text":"Canvas 2D 绘图","x":40,"y":56}));
    cmds.push(json!({"type":"setFillStyle","color":"#999999"}));
    cmds.push(json!({"type":"setFontSize","size":20}));
    cmds.push(json!({"type":"fillText","text":"矩形 / 路径 / 圆弧 / 文本 / 变换","x":40,"y":88}));

    // 柱状图
    let bars = [("周一", 150.0, "#07c160"), ("周二", 220.0, "#4a90d9"), ("周三", 110.0, "#ff9500"), ("周四", 260.0, "#ff3b30"), ("周五", 190.0, "#af52de")];
    let base_y = 400.0; let bar_w = 72.0; let gap = 28.0; let x0 = 50.0;
    // 基线
    cmds.push(json!({"type":"setStrokeStyle","color":"#dddddd"}));
    cmds.push(json!({"type":"setLineWidth","width":2}));
    cmds.push(json!({"type":"beginPath"}));
    cmds.push(json!({"type":"moveTo","x":40,"y":base_y}));
    cmds.push(json!({"type":"lineTo","x":560,"y":base_y}));
    cmds.push(json!({"type":"stroke"}));
    for (i, (label, bh, color)) in bars.iter().enumerate() {
        let x = x0 + i as f64 * (bar_w + gap);
        cmds.push(json!({"type":"setFillStyle","color":*color}));
        cmds.push(json!({"type":"fillRect","x":x,"y":base_y - *bh,"width":bar_w,"height":*bh}));
        cmds.push(json!({"type":"setFillStyle","color":"#333333"}));
        cmds.push(json!({"type":"setFontSize","size":22}));
        cmds.push(json!({"type":"setTextAlign","align":"center"}));
        cmds.push(json!({"type":"fillText","text":*label,"x":x + bar_w/2.0,"y":base_y + 28.0}));
        cmds.push(json!({"type":"fillText","text":format!("{}", *bh as i32),"x":x + bar_w/2.0,"y":base_y - *bh - 12.0}));
    }
    cmds.push(json!({"type":"setTextAlign","align":"left"}));

    // 圆弧 / 圆（甜甜圈进度环）
    cmds.push(json!({"type":"setStrokeStyle","color":"#eeeeee"}));
    cmds.push(json!({"type":"setLineWidth","width":20}));
    cmds.push(json!({"type":"beginPath"}));
    cmds.push(json!({"type":"arc","x":140,"y":510,"r":60,"sAngle":0.0,"eAngle":6.2832,"counterclockwise":false}));
    cmds.push(json!({"type":"stroke"}));
    cmds.push(json!({"type":"setStrokeStyle","color":"#07c160"}));
    cmds.push(json!({"type":"beginPath"}));
    cmds.push(json!({"type":"arc","x":140,"y":510,"r":60,"sAngle":-1.5708,"eAngle":2.5,"counterclockwise":false}));
    cmds.push(json!({"type":"stroke"}));
    cmds.push(json!({"type":"setFillStyle","color":"#07c160"}));
    cmds.push(json!({"type":"setFontSize","size":28}));
    cmds.push(json!({"type":"setTextAlign","align":"center"}));
    cmds.push(json!({"type":"fillText","text":"65%","x":140,"y":520}));
    cmds.push(json!({"type":"setTextAlign","align":"left"}));

    // 三角形路径（fill）
    cmds.push(json!({"type":"setFillStyle","color":"#ff9500"}));
    cmds.push(json!({"type":"beginPath"}));
    cmds.push(json!({"type":"moveTo","x":300,"y":460}));
    cmds.push(json!({"type":"lineTo","x":380,"y":560}));
    cmds.push(json!({"type":"lineTo","x":220,"y":560}));
    cmds.push(json!({"type":"closePath"}));
    cmds.push(json!({"type":"fill"}));

    // 变换演示：save/translate/rotate 画旋转方块
    cmds.push(json!({"type":"save"}));
    cmds.push(json!({"type":"translate","x":480,"y":510}));
    cmds.push(json!({"type":"rotate","angle":0.6}));
    cmds.push(json!({"type":"setFillStyle","color":"#4a90d9"}));
    cmds.push(json!({"type":"fillRect","x":-45,"y":-45,"width":90,"height":90}));
    cmds.push(json!({"type":"restore"}));

    execute_canvas_draw("demo", &serde_json::to_string(&cmds).unwrap());

    let wxml = r#"
    <view class="page">
        <text class="h">Canvas 画布</text>
        <canvas canvas-id="demo" class="cv"></canvas>
        <text class="tip">以上完全由 wx.createCanvasContext 的 2D 命令绘制</text>
    </view>"#;
    let wxss = r#"
    .page{ padding:24rpx; display:flex; flex-direction:column; align-items:center; }
    .h{ font-size:34rpx; font-weight:bold; color:#1a1a1a; margin-bottom:20rpx; align-self:flex-start; }
    .cv{ width:600rpx; height:600rpx; border-radius:16rpx; border:1rpx solid #eeeeee; }
    .tip{ font-size:24rpx; color:#999; margin-top:20rpx; }
    "#;
    render("34_canvas", 0xF5F6F8, wxml, wxss, json!({}));
}
