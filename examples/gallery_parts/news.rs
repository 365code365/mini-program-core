// 头条新闻类场景（36-39）：信息流首页 / 文章详情 / 视频频道 / 我的
//
// 与 news-app 小程序同款设计，用于验证资讯类版式：红色品牌栏、横向频道栏、
// 大图/左文右图/三图卡、热榜、长文排版、视频封面角标、设置项开关与滑块。

// 36. 新闻信息流首页
fn news_feed() {
    let wxml = r##"
    <view class="page">
        <view class="nav">
            <text class="logo">头条</text>
            <view class="search"><icon type="search" size="14" color="#9a9a9a"></icon><text class="stext">搜索热点新闻</text></view>
        </view>
        <view class="channels">
            <view class="ch chon"><text class="cht chton">推荐</text><view class="chline"></view></view>
            <view class="ch"><text class="cht">科技</text></view>
            <view class="ch"><text class="cht">体育</text></view>
            <view class="ch"><text class="cht">娱乐</text></view>
            <view class="ch"><text class="cht">财经</text></view>
            <view class="ch"><text class="cht">军事</text></view>
        </view>
        <view class="top"><text class="topbadge">置顶</text><text class="toptitle">国务院发布新一轮稳增长政策，涉及六大领域</text></view>
        <view class="card">
            <text class="bigtitle">数字经济规模再创新高，实体产业融合成为主线</text>
            <image class="bigcover" src="doc/gallery/assets/banner.jpg" mode="aspectFill" />
            <view class="meta"><text class="tag">独家</text><text class="src">新华观察</text><view class="dot"></view><text class="cmt">1286评论</text></view>
        </view>
        <view class="card rowcard" wx:for="{{list}}" wx:key="id">
            <view class="rleft">
                <text class="rtitle">{{item.title}}</text>
                <view class="meta"><text wx:if="{{item.hot}}" class="tag">热</text><text class="src">{{item.src}}</text><view class="dot"></view><text class="cmt">{{item.cmt}}评论</text></view>
            </view>
            <image class="rthumb" src="{{item.img}}" mode="aspectFill" />
        </view>
        <view class="card">
            <view class="hothead"><text class="hottitle">今日热榜</text><text class="hotmore">更多</text></view>
            <view class="hotitem" wx:for="{{hot}}" wx:key="t">
                <text class="rank {{index < 3 ? 'rtop' : 'rnor'}}">{{index + 1}}</text>
                <text class="hottext">{{item.t}}</text>
                <text class="heat">{{item.h}}万</text>
            </view>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f4f5f6; }
    .nav{ background-color:#d43c33; padding:16rpx 24rpx; display:flex; flex-direction:row; align-items:center; }
    .logo{ font-size:34rpx; color:#ffffff; font-weight:bold; margin-right:16rpx; }
    .search{ flex:1; background-color:#ffffff; border-radius:30rpx; height:60rpx; display:flex; flex-direction:row; align-items:center; padding:0 20rpx; }
    .stext{ font-size:26rpx; color:#9a9a9a; margin-left:10rpx; }
    .channels{ background-color:#ffffff; display:flex; flex-direction:row; height:84rpx; align-items:center; border-bottom:1rpx solid #ededed; }
    .ch{ display:flex; flex-direction:column; align-items:center; justify-content:center; height:84rpx; padding:0 26rpx; }
    .cht{ font-size:30rpx; color:#4a4a4a; }
    .chton{ color:#d43c33; font-weight:bold; font-size:32rpx; }
    .chline{ width:36rpx; height:6rpx; border-radius:3rpx; background-color:#d43c33; margin-top:6rpx; }
    .top{ background-color:#fff8f7; padding:18rpx 24rpx; display:flex; flex-direction:row; align-items:center; border-bottom:1rpx solid #f4e3e1; }
    .topbadge{ font-size:20rpx; color:#ffffff; background-color:#d43c33; border-radius:4rpx; padding:3rpx 10rpx; margin-right:14rpx; }
    .toptitle{ flex:1; font-size:28rpx; color:#1a1a1a; font-weight:bold; }
    .card{ background-color:#ffffff; padding:24rpx; border-bottom:1rpx solid #ededed; }
    .bigtitle{ display:block; font-size:34rpx; line-height:46rpx; color:#1a1a1a; font-weight:bold; }
    .bigcover{ width:100%; height:300rpx; border-radius:8rpx; margin-top:16rpx; }
    .meta{ display:flex; flex-direction:row; align-items:center; margin-top:14rpx; }
    .tag{ font-size:20rpx; color:#d43c33; border:1rpx solid #f0bdb9; border-radius:4rpx; padding:2rpx 8rpx; margin-right:10rpx; }
    .src{ font-size:22rpx; color:#8a8a8a; }
    .dot{ width:6rpx; height:6rpx; border-radius:50%; background-color:#cfcfcf; margin:0 12rpx; }
    .cmt{ font-size:22rpx; color:#8a8a8a; }
    .rowcard{ display:flex; flex-direction:row; align-items:flex-start; }
    .rleft{ flex:1; margin-right:18rpx; }
    .rtitle{ display:block; font-size:30rpx; line-height:42rpx; color:#1a1a1a; }
    .rthumb{ width:220rpx; height:150rpx; border-radius:8rpx; flex-shrink:0; }
    .hothead{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; margin-bottom:8rpx; }
    .hottitle{ font-size:30rpx; font-weight:bold; color:#1a1a1a; }
    .hotmore{ font-size:24rpx; color:#8a8a8a; }
    .hotitem{ display:flex; flex-direction:row; align-items:center; padding:16rpx 0; border-bottom:1rpx solid #f5f5f5; }
    .rank{ width:40rpx; font-size:26rpx; font-weight:bold; text-align:center; }
    .rtop{ color:#d43c33; }
    .rnor{ color:#b5b5b5; }
    .hottext{ flex:1; font-size:28rpx; color:#2a2a2a; margin-left:12rpx; }
    .heat{ font-size:22rpx; color:#b5b5b5; }
    "#;
    let data = json!({
        "list":[
            {"id":1,"title":"国产大飞机再获百架订单 交付节奏明显加快","src":"经济观察","cmt":328,"hot":true,"img":"doc/gallery/assets/p_phone.jpg"},
            {"id":2,"title":"多地推出住房消费新政 首套房贷利率下调","src":"财经快讯","cmt":156,"hot":false,"img":"doc/gallery/assets/p_backpack.jpg"},
            {"id":3,"title":"人工智能进课堂：一线教师的真实体验","src":"教育前沿","cmt":92,"hot":false,"img":"doc/gallery/assets/p_watch.jpg"}
        ],
        "hot":[
            {"t":"多部门联合发布消费提振举措","h":128},
            {"t":"台风路径调整 沿海地区加强防御","h":96},
            {"t":"国产芯片良率提升引发关注","h":87},
            {"t":"暑期研学市场火爆背后的隐忧","h":65}
        ]
    });
    render("36_news_feed", 0xF4F5F6, wxml, wxss, data);
}

// 37. 新闻文章详情
fn news_article() {
    let wxml = r##"
    <view class="page">
        <view class="art">
            <text class="title">数字经济规模再创新高，实体产业融合成为主线</text>
            <view class="author">
                <image class="avatar" src="doc/gallery/assets/avatar1.jpg" mode="aspectFill" />
                <view class="ainfo"><text class="aname">新华观察</text><text class="atime">2 小时前 · 12.8万阅读</text></view>
                <view class="follow"><text class="ftext">关注</text></view>
            </view>
            <image class="cover" src="doc/gallery/assets/banner.jpg" mode="aspectFill" />
            <text class="body">记者从相关部门获悉，上半年数字经济核心产业增加值同比增长明显，占国内生产总值比重进一步提升。

从结构上看，数字技术与实体产业的融合成为增长主线：智能制造改造项目数量同比增加，工业互联网平台连接设备规模持续扩大。</text>
            <view class="tags"><text class="t">数字经济</text><text class="t">产业升级</text><text class="t">独家</text></view>
        </view>
        <view class="block">
            <text class="stitle">精彩评论（3）</text>
            <view class="cm" wx:for="{{comments}}" wx:key="u">
                <image class="cavatar" src="doc/gallery/assets/avatar1.jpg" mode="aspectFill" />
                <view class="cbody">
                    <text class="cuser">{{item.u}}</text>
                    <text class="ctext">{{item.c}}</text>
                    <view class="cfoot"><text class="ctime">{{item.t}}</text><text class="clike">👍 {{item.l}}</text></view>
                </view>
            </view>
        </view>
        <view class="bar">
            <view class="input"><text class="itext">写评论…</text></view>
            <view class="act"><icon type="success" size="20" color="#d43c33"></icon><text class="acnt">1286</text></view>
            <view class="act"><icon type="waiting" size="20" color="#8a8a8a"></icon><text class="acnt">收藏</text></view>
            <view class="act"><icon type="info" size="20" color="#8a8a8a"></icon><text class="acnt">分享</text></view>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f4f5f6; }
    .art{ background-color:#ffffff; padding:28rpx 24rpx; }
    .title{ display:block; font-size:40rpx; line-height:56rpx; font-weight:bold; color:#1a1a1a; }
    .author{ display:flex; flex-direction:row; align-items:center; margin-top:24rpx; }
    .avatar{ width:68rpx; height:68rpx; border-radius:50%; flex-shrink:0; }
    .ainfo{ flex:1; margin-left:16rpx; }
    .aname{ display:block; font-size:28rpx; color:#2a2a2a; font-weight:bold; }
    .atime{ display:block; font-size:22rpx; color:#9a9a9a; margin-top:6rpx; }
    .follow{ border:1rpx solid #d43c33; border-radius:28rpx; padding:8rpx 24rpx; flex-shrink:0; }
    .ftext{ font-size:24rpx; color:#d43c33; }
    .cover{ width:100%; height:340rpx; border-radius:10rpx; margin-top:24rpx; }
    .body{ display:block; font-size:30rpx; line-height:52rpx; color:#333333; margin-top:24rpx; }
    .tags{ display:flex; flex-direction:row; flex-wrap:wrap; margin-top:24rpx; }
    .t{ font-size:20rpx; color:#d43c33; border:1rpx solid #f0bdb9; border-radius:4rpx; padding:4rpx 12rpx; margin-right:12rpx; }
    .block{ background-color:#ffffff; margin-top:16rpx; padding:24rpx; }
    .stitle{ display:block; font-size:30rpx; font-weight:bold; color:#1a1a1a; margin-bottom:12rpx; }
    .cm{ display:flex; flex-direction:row; padding:20rpx 0; border-bottom:1rpx solid #f5f5f5; }
    .cavatar{ width:56rpx; height:56rpx; border-radius:50%; flex-shrink:0; }
    .cbody{ flex:1; margin-left:16rpx; }
    .cuser{ display:block; font-size:24rpx; color:#5a6b8c; font-weight:bold; }
    .ctext{ display:block; font-size:28rpx; color:#2a2a2a; line-height:40rpx; margin-top:8rpx; }
    .cfoot{ display:flex; flex-direction:row; justify-content:space-between; align-items:center; margin-top:10rpx; }
    .ctime{ font-size:22rpx; color:#b5b5b5; }
    .clike{ font-size:22rpx; color:#8a8a8a; }
    .bar{ height:100rpx; background-color:#ffffff; border-top:1rpx solid #e6e6e6; display:flex; flex-direction:row; align-items:center; padding:0 20rpx; margin-top:16rpx; }
    .input{ flex:1; height:64rpx; background-color:#f2f3f5; border-radius:32rpx; display:flex; align-items:center; padding:0 24rpx; margin-right:20rpx; }
    .itext{ font-size:26rpx; color:#9a9a9a; }
    .act{ display:flex; flex-direction:column; align-items:center; justify-content:center; width:88rpx; }
    .acnt{ font-size:20rpx; color:#8a8a8a; margin-top:4rpx; }
    "#;
    let data = json!({"comments":[
        {"u":"industry_watcher","c":"融合是关键，单纯上系统解决不了产线的实际问题。","t":"1 小时前","l":216},
        {"u":"制造业老张","c":"我们厂去年做了改造，良率确实提升了，但人才招不到。","t":"2 小时前","l":154},
        {"u":"data_fan","c":"希望数据定价机制能尽快明确，现在交易顾虑比较多。","t":"3 小时前","l":97}
    ]});
    render("37_news_article", 0xF4F5F6, wxml, wxss, data);
}

// 38. 视频频道（封面 + 时长 + 播放角标）
fn news_video_list() {
    let wxml = r##"
    <view class="page">
        <view class="nav">
            <text class="logo">视频</text>
            <view class="search"><icon type="search" size="14" color="#9a9a9a"></icon><text class="stext">搜索短视频</text></view>
        </view>
        <view class="chips">
            <view class="chip chipon"><text class="chtext chton">推荐</text></view>
            <view class="chip"><text class="chtext">体育</text></view>
            <view class="chip"><text class="chtext">娱乐</text></view>
            <view class="chip"><text class="chtext">科技</text></view>
        </view>
        <view class="vcard" wx:for="{{videos}}" wx:key="id">
            <view class="cwrap">
                <image class="cover" src="{{item.cover}}" mode="aspectFill" />
                <view class="dur"><text class="durt">{{item.dur}}</text></view>
                <view class="play"><text class="ptext">▶</text></view>
            </view>
            <text class="vtitle">{{item.title}}</text>
            <view class="meta"><text class="src">{{item.author}}</text><view class="dot"></view><text class="cmt">{{item.plays}}次播放</text><view class="dot"></view><text class="cmt">{{item.cmt}}评论</text></view>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f4f5f6; }
    .nav{ background-color:#d43c33; padding:16rpx 24rpx; display:flex; flex-direction:row; align-items:center; }
    .logo{ font-size:34rpx; color:#ffffff; font-weight:bold; margin-right:16rpx; }
    .search{ flex:1; background-color:#ffffff; border-radius:30rpx; height:60rpx; display:flex; flex-direction:row; align-items:center; padding:0 20rpx; }
    .stext{ font-size:26rpx; color:#9a9a9a; margin-left:10rpx; }
    .chips{ display:flex; flex-direction:row; background-color:#ffffff; padding:18rpx 24rpx; border-bottom:1rpx solid #ededed; }
    .chip{ background-color:#f2f3f5; border-radius:26rpx; padding:10rpx 26rpx; margin-right:16rpx; }
    .chipon{ background-color:#fdecea; }
    .chtext{ font-size:26rpx; color:#5a5a5a; }
    .chton{ color:#d43c33; font-weight:bold; }
    .vcard{ background-color:#ffffff; padding:24rpx; border-bottom:1rpx solid #ededed; }
    .cwrap{ position:relative; width:100%; height:340rpx; border-radius:10rpx; overflow:hidden; }
    .cover{ width:100%; height:100%; }
    .dur{ position:absolute; right:16rpx; bottom:16rpx; background-color:rgba(0,0,0,0.65); border-radius:6rpx; padding:4rpx 12rpx; }
    .durt{ font-size:20rpx; color:#ffffff; }
    .play{ position:absolute; left:290rpx; top:126rpx; width:88rpx; height:88rpx; border-radius:44rpx; background-color:rgba(0,0,0,0.45); display:flex; align-items:center; justify-content:center; }
    .ptext{ font-size:32rpx; color:#ffffff; }
    .vtitle{ display:block; font-size:32rpx; line-height:44rpx; color:#1a1a1a; font-weight:bold; margin-top:18rpx; }
    .meta{ display:flex; flex-direction:row; align-items:center; margin-top:14rpx; }
    .src{ font-size:22rpx; color:#8a8a8a; }
    .dot{ width:6rpx; height:6rpx; border-radius:50%; background-color:#cfcfcf; margin:0 12rpx; }
    .cmt{ font-size:22rpx; color:#8a8a8a; }
    "#;
    let data = json!({"videos":[
        {"id":1,"title":"实拍：国产大飞机完成高原试飞全过程","author":"航空观察","cover":"doc/gallery/assets/p_phone.jpg","dur":"03:24","plays":"86.2万","cmt":1263},
        {"id":2,"title":"三分钟看懂新能源车电池技术路线之争","author":"汽车实验室","cover":"doc/gallery/assets/p_watch.jpg","dur":"02:58","plays":"42.7万","cmt":486}
    ]});
    render("38_news_video", 0xF4F5F6, wxml, wxss, data);
}

// 39. 我的（统计 + 菜单 + 开关/滑块设置）
fn news_mine() {
    let wxml = r##"
    <view class="page">
        <view class="uh">
            <image class="uavatar" src="doc/gallery/assets/avatar1.jpg" mode="aspectFill" />
            <view class="uinfo"><text class="uname">头条读者</text><text class="udesc">已连续阅读 128 天</text></view>
            <icon type="close" size="14" color="rgba(255,255,255,0.6)"></icon>
        </view>
        <view class="stat">
            <view class="sitem" wx:for="{{stats}}" wx:key="l"><text class="sv">{{item.v}}</text><text class="sl">{{item.l}}</text></view>
        </view>
        <view class="menu">
            <text class="mhead">我的服务</text>
            <view class="mitem" wx:for="{{menus}}" wx:key="n">
                <icon type="{{item.i}}" size="20" color="{{item.c}}"></icon>
                <text class="mtext">{{item.n}}</text>
                <text wx:if="{{item.b}}" class="mbadge">{{item.b}}</text>
                <text class="marrow">></text>
            </view>
        </view>
        <view class="menu">
            <text class="mhead">阅读设置</text>
            <view class="srow"><text class="slabel">夜间模式</text><switch checked="false"></switch></view>
            <view class="srow"><text class="slabel">非 Wi-Fi 自动播放</text><switch checked="true"></switch></view>
            <view class="slrow"><text class="slabel">正文字号</text><slider value="3" min="1" max="5" step="1" show-value="true"></slider></view>
        </view>
        <view class="lwrap"><button class="lbtn">退出登录</button></view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f4f5f6; }
    .uh{ background-color:#d43c33; padding:44rpx 28rpx; display:flex; flex-direction:row; align-items:center; }
    .uavatar{ width:116rpx; height:116rpx; border-radius:58rpx; flex-shrink:0; }
    .uinfo{ flex:1; margin-left:24rpx; }
    .uname{ display:block; font-size:36rpx; color:#ffffff; font-weight:bold; }
    .udesc{ display:block; font-size:24rpx; color:rgba(255,255,255,0.85); margin-top:10rpx; }
    .stat{ background-color:#ffffff; margin:20rpx; border-radius:14rpx; padding:26rpx 0; display:flex; flex-direction:row; justify-content:space-around; }
    .sitem{ display:flex; flex-direction:column; align-items:center; min-width:120rpx; }
    .sv{ font-size:38rpx; color:#1a1a1a; font-weight:bold; }
    .sl{ font-size:22rpx; color:#9a9a9a; margin-top:8rpx; }
    .menu{ background-color:#ffffff; margin:0 20rpx 20rpx; border-radius:14rpx; padding:8rpx 24rpx 4rpx; }
    .mhead{ display:block; font-size:26rpx; color:#9a9a9a; padding:16rpx 0 8rpx; }
    .mitem{ display:flex; flex-direction:row; align-items:center; padding:26rpx 0; border-bottom:1rpx solid #f5f5f5; }
    .mtext{ flex:1; font-size:30rpx; color:#2a2a2a; margin-left:18rpx; }
    .mbadge{ font-size:20rpx; color:#ffffff; background-color:#d43c33; border-radius:18rpx; padding:2rpx 12rpx; margin-right:12rpx; }
    .marrow{ font-size:26rpx; color:#c8c8c8; }
    .srow{ display:flex; flex-direction:row; align-items:center; justify-content:space-between; padding:22rpx 0; border-bottom:1rpx solid #f5f5f5; }
    .slabel{ font-size:30rpx; color:#2a2a2a; }
    .slrow{ padding:22rpx 0 8rpx; }
    .lwrap{ padding:20rpx; }
    .lbtn{ background-color:#ffffff; color:#d43c33; border-radius:46rpx; font-size:30rpx; }
    "#;
    let data = json!({
        "stats":[{"l":"关注","v":42},{"l":"收藏","v":186},{"l":"历史","v":"1.2万"},{"l":"评论","v":73}],
        "menus":[
            {"n":"我的收藏","i":"success","c":"#d43c33","b":""},
            {"n":"阅读历史","i":"waiting","c":"#f0a020","b":""},
            {"n":"消息通知","i":"info","c":"#3a7bd5","b":"9"},
            {"n":"意见反馈","i":"warn","c":"#7b61ff","b":""}
        ]
    });
    render("39_news_mine", 0xF4F5F6, wxml, wxss, data);
}
