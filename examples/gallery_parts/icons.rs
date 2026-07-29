// 内置 `<icon>` 图标总览：所有 type × 常用尺寸 × 配色场景。
//
// 这张图是 `<icon>` 的回归基线。图形数据来自 WeUI（微信自家那套），单色 + even-odd
// 挖洞，所以：
// - `success` 是「一整块绿 + 对勾形状的洞」，不是「绿圆 + 白对勾」；
// - 因此把 `color` 设成白色再放到有色底上，对勾会**透出底色**（最后一组专门验证这一点，
//   以前白对勾会糊成一坨白）；
// - 字形自带留白（24 视图盒里圆直径 20），所以 `size="23"` 的圆约 19px 而不是 23px。

fn icon_showcase() {
    let wxss = format!(
        "{}{}",
        common(),
        r#"
        .page{ background-color:#f5f6f8; padding:20rpx; }
        .sect{ font-size:26rpx; color:#888888; margin:16rpx 0 12rpx 8rpx; }
        .grid{ display:flex; flex-direction:row; flex-wrap:wrap; background-color:#ffffff;
               border-radius:16rpx; padding:12rpx 0; }
        .cell{ width:25%; display:flex; flex-direction:column; align-items:center;
               justify-content:flex-start; padding:16rpx 0; }
        .cname{ font-size:19rpx; color:#999999; margin-top:10rpx; }
        .sizes{ display:flex; flex-direction:row; align-items:flex-end;
                background-color:#ffffff; border-radius:16rpx; padding:24rpx; }
        .sz{ display:flex; flex-direction:column; align-items:center; margin-right:28rpx; }
        .szl{ font-size:19rpx; color:#bbbbbb; margin-top:8rpx; }
        .onbg{ display:flex; flex-direction:row; background-color:#ffffff;
               border-radius:16rpx; padding:24rpx; }
        .bub{ width:88rpx; height:88rpx; border-radius:44rpx; display:flex;
              align-items:center; justify-content:center; margin-right:24rpx; }
        .dark{ display:flex; flex-direction:row; background-color:#1f2430;
               border-radius:16rpx; padding:24rpx; }
        .dcell{ margin-right:28rpx; }
        "#
    );

    let wxml = r##"
    <view class="page">
      <view class="sect">微信官方 type（默认色）</view>
      <view class="grid">
        <view class="cell" wx:for="{{official}}" wx:key="t">
          <icon type="{{item}}" size="30" />
          <text class="cname">{{item}}</text>
        </view>
      </view>

      <view class="sect">引擎扩展 type（默认色）</view>
      <view class="grid">
        <view class="cell" wx:for="{{extra}}" wx:key="t">
          <icon type="{{item}}" size="30" />
          <text class="cname">{{item}}</text>
        </view>
      </view>

      <view class="sect">尺寸档（success，字形自带留白：圆 ≈ size × 5/6）</view>
      <view class="sizes">
        <view class="sz" wx:for="{{sizes}}" wx:key="s">
          <icon type="success" size="{{item}}" />
          <text class="szl">{{item}}</text>
        </view>
      </view>

      <view class="sect">白色图标 + 有色底：对勾/叉应透出底色</view>
      <view class="onbg">
        <view class="bub" style="background-color:{{item.c}}" wx:for="{{bubbles}}" wx:key="t">
          <icon type="{{item.t}}" size="26" color="#ffffff" />
        </view>
      </view>

      <view class="sect">深色背景 + 自定义颜色</view>
      <view class="dark">
        <view class="dcell" wx:for="{{tinted}}" wx:key="t">
          <icon type="{{item.t}}" size="28" color="{{item.c}}" />
        </view>
      </view>
    </view>
    "##;

    render(
        "65_icon_showcase",
        0xf5f6f8,
        wxml,
        &wxss,
        json!({
            "official": [
                "success", "success_no_circle", "info", "warn",
                "waiting", "cancel", "clear", "download", "search"
            ],
            "extra": [
                "success_circle", "info_circle", "waiting_circle", "circle",
                "close", "back", "arrow_right", "arrow_up", "arrow_down",
                "plus", "minus", "star", "star-o", "heart", "heart-o",
                "chat", "chat-o", "info_no_circle", "warn_no_circle", "clock"
            ],
            "sizes": [14, 18, 23, 30, 40, 56],
            "bubbles": [
                {"t": "success", "c": "#07c160"},
                {"t": "clear",   "c": "#fa5151"},
                {"t": "info",    "c": "#10aeff"},
                {"t": "waiting", "c": "#ff8f1f"}
            ],
            "tinted": [
                {"t": "star", "c": "#ffb400"},
                {"t": "heart", "c": "#ff4d6a"},
                {"t": "success_no_circle", "c": "#07c160"},
                {"t": "search", "c": "#ffffff"},
                {"t": "arrow_right", "c": "#8a8f9c"}
            ]
        }),
    );
}
