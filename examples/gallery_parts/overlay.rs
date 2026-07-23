// 19. 确认弹窗（Modal）
fn modal_dialog() {
    let wxml = r##"
    <view class="mask">
        <view class="dialog">
            <text class="d-title">删除确认</text>
            <text class="d-content">确定要删除选中的 3 件商品吗？删除后将无法恢复。</text>
            <view class="d-btns row">
                <text class="d-btn cancel">取消</text>
                <text class="d-btn ok">删除</text>
            </view>
        </view>
    </view>"##;
    let wxss = r##"
    .mask{ height:1334rpx; background-color:rgba(0,0,0,0.55); display:flex; flex-direction:column; justify-content:center; align-items:center; }
    .dialog{ width:580rpx; background-color:#ffffff; border-radius:24rpx; display:flex; flex-direction:column; align-items:center; padding-top:48rpx; }
    .d-title{ font-size:34rpx; font-weight:bold; color:#1a1a1a; }
    .d-content{ width:490rpx; font-size:28rpx; color:#888888; text-align:center; line-height:44rpx; margin:24rpx 0 40rpx 0; }
    .row{ display:flex; flex-direction:row; }
    .d-btns{ display:flex; flex-direction:row; width:580rpx; border-top:1rpx solid #f0f0f0; }
    .d-btn{ flex:1; text-align:center; padding:30rpx; font-size:32rpx; }
    .cancel{ color:#888888; }
    .ok{ color:#fa5151; font-weight:bold; }
    "##;
    render_screen("19_modal", 0xEDEDED, wxml, wxss, json!({}));
}

// 20. 底部操作面板（ActionSheet）
fn action_sheet() {
    let wxml = r##"
    <view class="mask">
        <view class="sheet">
            <text class="opt" wx:for="{{opts}}" wx:key="*this">{{item}}</text>
            <view class="gap"></view>
            <text class="opt cancel">取消</text>
        </view>
    </view>"##;
    let wxss = r##"
    .mask{ height:1334rpx; background-color:rgba(0,0,0,0.55); display:flex; flex-direction:column; justify-content:flex-end; }
    .sheet{ background-color:#f7f7f7; display:flex; flex-direction:column; }
    .opt{ background-color:#ffffff; text-align:center; font-size:32rpx; color:#1a1a1a; padding:34rpx; border-bottom:1rpx solid #f0f0f0; }
    .cancel{ color:#576b95; font-weight:bold; border-bottom:0; }
    .gap{ height:16rpx; background-color:#ededed; }
    "##;
    render_screen("20_action_sheet", 0xEDEDED, wxml, wxss, json!({
        "opts": ["拍照", "从相册选择", "保存到本地", "分享给好友"]
    }));
}

// 21. 轻提示（Toast）
fn toast() {
    let wxml = r##"
    <view class="wrap">
        <view class="toast">
            <icon type="success_no_circle" size="60" color="#ffffff" />
            <text class="tt">操作成功</text>
        </view>
    </view>"##;
    let wxss = r##"
    .wrap{ height:1334rpx; display:flex; flex-direction:column; justify-content:center; align-items:center; }
    .toast{ background-color:rgba(0,0,0,0.8); border-radius:20rpx; padding:44rpx 56rpx; display:flex; flex-direction:column; align-items:center; }
    .tt{ color:#ffffff; font-size:30rpx; margin-top:24rpx; }
    "##;
    render_screen("21_toast", 0xF5F6F8, wxml, wxss, json!({}));
}

