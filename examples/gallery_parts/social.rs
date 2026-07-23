// 7. 聊天
fn chat() {
    let wxml = r#"
    <view class="page">
        <view class="msg row {{item.me ? 'me' : ''}}" wx:for="{{msgs}}" wx:key="id">
            <image class="av" wx:if="{{!item.me}}" src="doc/gallery/assets/avatar2.jpg" mode="aspectFill" />
            <view class="bubble {{item.me ? 'b-me' : ''}}"><text class="tx {{item.me ? 'tx-me' : ''}}">{{item.text}}</text></view>
            <image class="av" wx:if="{{item.me}}" src="doc/gallery/assets/avatar1.jpg" mode="aspectFill" />
        </view>
    </view>"#;
    let wxss = r#"
    .page{ padding:24rpx; }
    .msg{ display:flex; flex-direction:row; align-items:flex-start; margin-bottom:30rpx; }
    .msg.me{ justify-content:flex-end; }
    .av{ width:72rpx; height:72rpx; border-radius:16rpx; background-color:#c7e0ff; }
    .bubble{ background-color:#ffffff; border-radius:16rpx; padding:20rpx 24rpx; margin:0 20rpx; max-width:460rpx; }
    .b-me{ background-color:#95ec69; }
    .tx{ font-size:30rpx; color:#1a1a1a; }
    "#;
    let data = json!({"msgs":[
        {"id":1,"me":false,"text":"在吗？周末有空一起爬山不？"},
        {"id":2,"me":true,"text":"有空啊！几点集合？"},
        {"id":3,"me":false,"text":"早上八点，老地方见"},
        {"id":4,"me":true,"text":"好嘞，带上水和干粮"},
        {"id":5,"me":false,"text":"好的，到时候见"}
    ]});
    render("07_chat", 0xEDEDED, wxml, wxss, data);
}

// 8. 动态流
fn feed() {
    let wxml = r#"
    <view class="page">
        <view class="card" wx:for="{{posts}}" wx:key="id">
            <view class="row top">
                <image class="avatar" src="{{item.avatar}}" mode="aspectFill"></image>
                <view class="col grow"><text class="name">{{item.user}}</text><text class="time">{{item.time}}</text></view>
                <view class="follow">关注</view>
            </view>
            <text class="content">{{item.text}}</text>
            <view class="imgs row">
                <image class="ig" src="{{item.img1}}" mode="aspectFill"></image>
                <image class="ig" src="{{item.img2}}" mode="aspectFill"></image>
                <image class="ig" src="{{item.img3}}" mode="aspectFill"></image>
            </view>
            <view class="actions row between">
                <text class="act">赞 {{item.likes}}</text>
                <text class="act">评论 {{item.comments}}</text>
                <text class="act">转发</text>
            </view>
        </view>
    </view>"#;
    let wxss = r#"
    .page{ padding:20rpx; }
    .card{ background-color:#fff; border-radius:20rpx; padding:28rpx; margin-bottom:20rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .between{ justify-content:space-between; }
    .top{ margin-bottom:20rpx; }
    .avatar{ width:84rpx; height:84rpx; border-radius:42rpx; margin-right:20rpx; }
    .col{ display:flex; flex-direction:column; }
    .grow{ flex:1; }
    .name{ font-size:30rpx; color:#1a1a1a; font-weight:bold; }
    .time{ font-size:24rpx; color:#bbb; margin-top:6rpx; }
    .follow{ background-color:#07c160; color:#fff; font-size:24rpx; padding:10rpx 24rpx; border-radius:28rpx; }
    .content{ font-size:30rpx; color:#333; line-height:44rpx; }
    .imgs{ display:flex; flex-direction:row; margin-top:20rpx; }
    .ig{ width:210rpx; height:210rpx; border-radius:12rpx; margin-right:12rpx; }
    .actions{ display:flex; flex-direction:row; justify-content:space-between; margin-top:24rpx; }
    .act{ font-size:26rpx; color:#888; }
    "#;
    let data = json!({"posts":[
        {"id":1,"user":"摄影师阿凯","time":"10分钟前","text":"周末去了趟海边，随手一拍都是壁纸","likes":328,"comments":42,
         "avatar":"doc/gallery/assets/avatar1.jpg","img1":"doc/gallery/assets/p_coffee.jpg","img2":"doc/gallery/assets/p_cake.jpg","img3":"doc/gallery/assets/banner.jpg"},
        {"id":2,"user":"美食日记","time":"1小时前","text":"在家复刻了一份提拉米苏，成功！","likes":156,"comments":23,
         "avatar":"doc/gallery/assets/avatar2.jpg","img1":"doc/gallery/assets/p_cake.jpg","img2":"doc/gallery/assets/p_coffee.jpg","img3":"doc/gallery/assets/p_thermos.jpg"}
    ]});
    render("08_feed", 0xF5F6F8, wxml, wxss, data);
}

