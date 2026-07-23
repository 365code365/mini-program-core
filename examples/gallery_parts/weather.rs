// 13. 天气
// 13. 天气（多城市多天气，AI 城市背景 + 叠加信息）
fn weather() {
    weather_scene("13_weather", "深圳市 · 南山区", "doc/gallery/assets/w_sunny.jpg", 28, "晴 · 东南风 3级", 31, 24,
        json!([{"t":"现在","v":28,"c":"#ffcc00"},{"t":"14时","v":30,"c":"#ffcc00"},{"t":"15时","v":31,"c":"#ffcc00"},{"t":"16时","v":30,"c":"#ffcc00"},{"t":"17时","v":28,"c":"#ff9500"}]),
        json!([{"d":"今天","w":"晴","t":"24° ~ 31°"},{"d":"明天","w":"多云","t":"25° ~ 32°"},{"d":"周三","w":"阵雨","t":"23° ~ 28°"}]));

    weather_scene("35_weather_rain", "上海市 · 浦东新区", "doc/gallery/assets/w_rain.jpg", 19, "中雨 · 北风 4级", 22, 16,
        json!([{"t":"现在","v":19,"c":"#4a90d9"},{"t":"14时","v":20,"c":"#4a90d9"},{"t":"15时","v":19,"c":"#4a90d9"},{"t":"16时","v":18,"c":"#4a90d9"},{"t":"17时","v":17,"c":"#5f6caf"}]),
        json!([{"d":"今天","w":"中雨","t":"16° ~ 22°"},{"d":"明天","w":"小雨","t":"17° ~ 23°"},{"d":"周三","w":"多云","t":"18° ~ 25°"}]));

    weather_scene("36_weather_cloudy", "重庆市 · 渝中区", "doc/gallery/assets/w_cloudy.jpg", 23, "多云 · 微风 2级", 27, 20,
        json!([{"t":"现在","v":23,"c":"#9aa7b5"},{"t":"14时","v":25,"c":"#9aa7b5"},{"t":"15时","v":26,"c":"#9aa7b5"},{"t":"16时","v":25,"c":"#9aa7b5"},{"t":"17时","v":24,"c":"#9aa7b5"}]),
        json!([{"d":"今天","w":"多云","t":"20° ~ 27°"},{"d":"明天","w":"阴","t":"19° ~ 24°"},{"d":"周三","w":"晴","t":"21° ~ 29°"}]));

    weather_scene("37_weather_night", "广州市 · 天河区", "doc/gallery/assets/w_night.jpg", 26, "晴 夜间 · 无风", 30, 25,
        json!([{"t":"现在","v":26,"c":"#ffcc00"},{"t":"21时","v":27,"c":"#ffcc00"},{"t":"22时","v":26,"c":"#ffcc00"},{"t":"23时","v":26,"c":"#ff9500"},{"t":"0时","v":25,"c":"#5f6caf"}]),
        json!([{"d":"今天","w":"晴","t":"25° ~ 30°"},{"d":"明天","w":"晴","t":"26° ~ 33°"},{"d":"周三","w":"多云","t":"24° ~ 30°"}]));
}

/// 单个城市天气场景：AI 城市背景 + 半透明遮罩 + 叠加信息 + 小时/多日预报卡片
fn weather_scene(id: &str, city: &str, bg: &str, temp: i32, desc: &str, hi: i32, lo: i32, hours: Value, days: Value) {
    let wxml = r#"
    <view class="wrap">
        <view class="hero">
            <image class="bg" src="{{bg}}" mode="aspectFill" />
            <view class="mask"></view>
            <view class="info">
                <text class="city">{{city}}</text>
                <view class="trow">
                    <text class="temp">{{temp}}</text>
                    <text class="unit">°C</text>
                </view>
                <text class="desc">{{desc}}</text>
                <text class="range">最高 {{hi}}°   最低 {{lo}}°</text>
            </view>
        </view>
        <view class="hours card">
            <view class="hour" wx:for="{{hours}}" wx:key="t">
                <text class="ht">{{item.t}}</text>
                <view class="dot" style="background-color:{{item.c}}"></view>
                <text class="hv">{{item.v}}°</text>
            </view>
        </view>
        <view class="card">
            <view class="drow" wx:for="{{days}}" wx:key="d">
                <text class="dd">{{item.d}}</text><text class="dw">{{item.w}}</text><text class="dt">{{item.t}}</text>
            </view>
        </view>
    </view>"#;
    let wxss = r#"
    .wrap{ background-color:#eef1f5; }
    .hero{ position:relative; width:750rpx; height:560rpx; }
    .bg{ position:absolute; top:0; left:0; width:750rpx; height:560rpx; }
    .mask{ position:absolute; top:0; left:0; width:750rpx; height:560rpx; background-color:rgba(0,0,0,0.30); }
    .info{ position:absolute; top:70rpx; left:0; width:750rpx; display:flex; flex-direction:column; align-items:center; }
    .city{ font-size:36rpx; color:#ffffff; font-weight:bold; }
    .trow{ display:flex; flex-direction:row; align-items:flex-start; margin-top:16rpx; }
    .temp{ font-size:150rpx; color:#ffffff; font-weight:bold; line-height:150rpx; }
    .unit{ font-size:52rpx; color:#ffffff; margin-top:18rpx; margin-left:6rpx; }
    .desc{ font-size:30rpx; color:#ffffff; margin-top:20rpx; }
    .range{ font-size:26rpx; color:#eaeef5; margin-top:14rpx; }
    .card{ background-color:#ffffff; border-radius:20rpx; padding:28rpx; margin:24rpx; box-shadow:0 6rpx 20rpx rgba(0,0,0,0.06); }
    .hours{ display:flex; flex-direction:row; justify-content:space-between; }
    .hour{ display:flex; flex-direction:column; align-items:center; flex:1; }
    .ht{ font-size:24rpx; color:#999; }
    .dot{ width:20rpx; height:20rpx; border-radius:10rpx; margin:18rpx 0; }
    .hv{ font-size:30rpx; color:#333; font-weight:bold; }
    .drow{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; padding:22rpx 0; border-bottom:1rpx solid #f2f2f2; }
    .dd{ font-size:28rpx; color:#333; width:120rpx; }
    .dw{ font-size:28rpx; color:#888; }
    .dt{ font-size:28rpx; color:#333; }
    "#;
    let data = json!({"city":city,"bg":bg,"temp":temp,"desc":desc,"hi":hi,"lo":lo,"hours":hours,"days":days});
    render(id, 0xEEF1F5, wxml, wxss, data);
}

