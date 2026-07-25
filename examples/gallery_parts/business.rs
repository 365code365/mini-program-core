// 更多商业级场景（40-51）：物流跟踪 / 直播带货 / 外卖点餐 / 会员中心 /
// 支付确认 / 评价晒单 / 搜索结果 / 消息中心 / 签到打卡 / 行情看板 /
// 酒店预订 / 运动健康
//
// 这批场景刻意覆盖真实 App 里高频出现的版式：时间轴、悬浮直播间、
// 左右分栏点餐、渐变会员卡、金额收银台、星级评价、搜索联想、会话列表、
// 日历签到、涨跌行情、日期区间与价格、环形与柱状数据。

// 40. 物流跟踪（时间轴）
fn logistics_track() {
    let wxml = r##"
    <view class="page">
        <view class="head">
            <view class="hrow"><text class="status">运输中</text><text class="eta">预计明天 18:00 前送达</text></view>
            <view class="hrow2"><text class="cno">顺丰速运 SF7382910365</text><text class="copy">复制</text></view>
        </view>
        <view class="card">
            <view class="node" wx:for="{{steps}}" wx:key="t">
                <view class="lcol">
                    <view class="dot {{index == 0 ? 'don' : ''}}"></view>
                    <view wx:if="{{index < 4}}" class="line"></view>
                </view>
                <view class="rcol">
                    <text class="stext {{index == 0 ? 'son' : ''}}">{{item.d}}</text>
                    <text class="stime">{{item.t}}</text>
                </view>
            </view>
        </view>
        <view class="card row">
            <image class="pimg" src="doc/gallery/assets/p_earbuds.jpg" mode="aspectFill" />
            <view class="pinfo"><text class="pname">无线蓝牙耳机 Pro</text><text class="pspec">黑色 · 1 件</text></view>
            <text class="pprice">¥199</text>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f5f6f8; padding-bottom:20rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .head{ background:linear-gradient(135deg, #2b7cff, #0a58d0); padding:36rpx 28rpx; }
    .hrow{ display:flex; flex-direction:row; align-items:baseline; }
    .status{ font-size:40rpx; color:#ffffff; font-weight:bold; }
    .eta{ font-size:24rpx; color:rgba(255,255,255,0.9); margin-left:20rpx; }
    .hrow2{ display:flex; flex-direction:row; align-items:center; justify-content:space-between; margin-top:18rpx; }
    .cno{ font-size:24rpx; color:rgba(255,255,255,0.9); }
    .copy{ font-size:22rpx; color:#ffffff; border:1rpx solid rgba(255,255,255,0.7); border-radius:20rpx; padding:4rpx 16rpx; }
    .card{ background-color:#ffffff; border-radius:16rpx; margin:20rpx; padding:28rpx; }
    .node{ display:flex; flex-direction:row; }
    .lcol{ width:40rpx; display:flex; flex-direction:column; align-items:center; }
    .dot{ width:16rpx; height:16rpx; border-radius:8rpx; background-color:#d8d8d8; margin-top:8rpx; }
    .don{ width:22rpx; height:22rpx; border-radius:11rpx; background-color:#2b7cff; margin-top:4rpx; }
    .line{ width:2rpx; height:74rpx; background-color:#ececec; }
    .rcol{ flex:1; padding-bottom:26rpx; }
    .stext{ display:block; font-size:28rpx; color:#666666; line-height:38rpx; }
    .son{ color:#1a1a1a; font-weight:bold; }
    .stime{ display:block; font-size:22rpx; color:#b0b0b0; margin-top:8rpx; }
    .pimg{ width:120rpx; height:120rpx; border-radius:12rpx; }
    .pinfo{ flex:1; margin-left:20rpx; }
    .pname{ display:block; font-size:28rpx; color:#1a1a1a; }
    .pspec{ display:block; font-size:22rpx; color:#999999; margin-top:10rpx; }
    .pprice{ font-size:30rpx; color:#ff3b30; font-weight:bold; }
    "#;
    let data = json!({"steps":[
        {"d":"【上海市】快件已到达 浦东新区营业点，正在派送","t":"07-24 09:12"},
        {"d":"【上海市】快件已到达 上海转运中心","t":"07-24 02:40"},
        {"d":"【杭州市】快件已发出，下一站 上海转运中心","t":"07-23 21:08"},
        {"d":"【杭州市】顺丰速运 已收取快件","t":"07-23 17:35"},
        {"d":"商家已发货，等待快递揽收","t":"07-23 15:02"}
    ]});
    render("40_logistics", 0xF5F6F8, wxml, wxss, data);
}

// 41. 直播带货
fn live_shopping() {
    let wxml = r##"
    <view class="page">
        <image class="bg" src="doc/gallery/assets/p_lipstick.jpg" mode="aspectFill" />
        <view class="top row">
            <view class="anchor row">
                <image class="aimg" src="doc/gallery/assets/avatar1.jpg" mode="aspectFill" />
                <view class="acol"><text class="aname">美妆小仙女</text><text class="afans">12.6万人在看</text></view>
                <view class="follow"><text class="ftext">关注</text></view>
            </view>
            <view class="viewers"><text class="vtext">3.2万</text></view>
        </view>
        <view class="mid">
            <view class="badge"><text class="btext">🔥 秒杀中</text></view>
            <view class="chat" wx:for="{{chats}}" wx:key="u">
                <text class="cu">{{item.u}}：</text><text class="cm">{{item.m}}</text>
            </view>
        </view>
        <view class="goods row">
            <image class="gimg" src="doc/gallery/assets/p_lipstick.jpg" mode="aspectFill" />
            <view class="gcol">
                <text class="gname">丝绒雾面口红 正装 3.5g</text>
                <view class="prow row"><text class="gp">¥89</text><text class="go">¥159</text><text class="gtag">直播价</text></view>
            </view>
            <view class="buy"><text class="btxt">马上抢</text></view>
        </view>
        <view class="bar row">
            <view class="inp"><text class="itext">说点什么…</text></view>
            <view class="ico"><icon type="success" size="20" color="#ffffff"></icon></view>
            <view class="ico"><icon type="info" size="20" color="#ffffff"></icon></view>
            <view class="cart"><text class="ctext">12</text></view>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ height:1334rpx; background-color:#111111; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .bg{ position:absolute; left:0; top:0; width:750rpx; height:1334rpx; }
    .top{ justify-content:space-between; padding:28rpx 24rpx; }
    .anchor{ background-color:rgba(0,0,0,0.45); border-radius:40rpx; padding:8rpx 16rpx 8rpx 8rpx; }
    .aimg{ width:64rpx; height:64rpx; border-radius:32rpx; }
    .acol{ display:flex; flex-direction:column; margin:0 20rpx 0 14rpx; }
    .aname{ font-size:26rpx; color:#ffffff; font-weight:bold; }
    .afans{ font-size:20rpx; color:rgba(255,255,255,0.85); margin-top:4rpx; }
    .follow{ background-color:#ff2d55; border-radius:24rpx; padding:8rpx 22rpx; }
    .ftext{ font-size:22rpx; color:#ffffff; }
    .viewers{ background-color:rgba(0,0,0,0.45); border-radius:24rpx; padding:8rpx 18rpx; }
    .vtext{ font-size:22rpx; color:#ffffff; }
    .mid{ margin-top:470rpx; padding:0 24rpx; }
    .badge{ background:linear-gradient(90deg,#ff9500,#ff3b30); border-radius:24rpx; padding:8rpx 20rpx; align-self:flex-start; margin-bottom:20rpx; }
    .btext{ font-size:22rpx; color:#ffffff; font-weight:bold; }
    .chat{ background-color:rgba(0,0,0,0.35); border-radius:20rpx; padding:10rpx 18rpx; margin-bottom:12rpx; align-self:flex-start; }
    .cu{ font-size:24rpx; color:#9fd0ff; }
    .cm{ font-size:24rpx; color:#ffffff; }
    .goods{ background-color:rgba(255,255,255,0.96); border-radius:20rpx; margin:24rpx; padding:20rpx; }
    .gimg{ width:130rpx; height:130rpx; border-radius:14rpx; }
    .gcol{ flex:1; margin-left:20rpx; }
    .gname{ display:block; font-size:28rpx; color:#1a1a1a; }
    .prow{ margin-top:14rpx; align-items:baseline; }
    .gp{ font-size:40rpx; color:#ff2d55; font-weight:bold; }
    .go{ font-size:24rpx; color:#999999; text-decoration:line-through; margin-left:12rpx; }
    .gtag{ font-size:20rpx; color:#ffffff; background-color:#ff2d55; border-radius:6rpx; padding:4rpx 10rpx; margin-left:14rpx; }
    .buy{ background:linear-gradient(135deg,#ff9500,#ff2d55); border-radius:32rpx; padding:20rpx 28rpx; }
    .btxt{ font-size:28rpx; color:#ffffff; font-weight:bold; }
    .bar{ padding:0 24rpx; }
    .inp{ flex:1; height:72rpx; border-radius:36rpx; background-color:rgba(255,255,255,0.22); display:flex; align-items:center; padding:0 26rpx; }
    .itext{ font-size:26rpx; color:rgba(255,255,255,0.9); }
    .ico{ width:72rpx; height:72rpx; border-radius:36rpx; background-color:rgba(255,255,255,0.22); display:flex; align-items:center; justify-content:center; margin-left:16rpx; }
    .cart{ width:72rpx; height:72rpx; border-radius:36rpx; background-color:#ff2d55; display:flex; align-items:center; justify-content:center; margin-left:16rpx; }
    .ctext{ font-size:26rpx; color:#ffffff; font-weight:bold; }
    "#;
    let data = json!({"chats":[
        {"u":"小美","m":"这个色号显白吗？"},
        {"u":"甜甜圈","m":"已经下单啦，第三支"},
        {"u":"路过的风","m":"主播讲讲持久度"}
    ]});
    render_screen("41_live_shopping", 0x111111, wxml, wxss, data);
}

// 42. 外卖点餐（左右分栏 + 购物袋）
fn food_order() {
    let wxml = r##"
    <view class="page">
        <view class="shop row">
            <image class="logo" src="doc/gallery/assets/p_cake.jpg" mode="aspectFill" />
            <view class="scol">
                <text class="sname">巷子口·手作烘焙</text>
                <view class="row"><text class="score">4.9分</text><text class="sales">月售 2361</text><text class="time">28分钟</text></view>
            </view>
        </view>
        <view class="body row">
            <view class="cats">
                <view class="cat {{index == 1 ? 'caton' : ''}}" wx:for="{{cats}}" wx:key="n">
                    <text class="ctext {{index == 1 ? 'cton' : ''}}">{{item.n}}</text>
                    <view wx:if="{{item.b}}" class="cbadge"><text class="cbt">{{item.b}}</text></view>
                </view>
            </view>
            <view class="dishes">
                <text class="dhead">热销推荐</text>
                <view class="dish row" wx:for="{{dishes}}" wx:key="n">
                    <image class="dimg" src="{{item.img}}" mode="aspectFill" />
                    <view class="dcol">
                        <text class="dname">{{item.n}}</text>
                        <text class="ddesc">{{item.d}}</text>
                        <view class="row bl"><text class="dp">¥{{item.p}}</text><text class="dsale">已售{{item.s}}</text></view>
                    </view>
                    <view class="plus"><text class="ptext">+</text></view>
                </view>
            </view>
        </view>
        <view class="bag row">
            <view class="bicon"><text class="bnum">3</text></view>
            <view class="bcol"><text class="btotal">¥86.00</text><text class="bfee">另需配送费 ¥5</text></view>
            <view class="checkout"><text class="cktext">去结算</text></view>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f5f6f8; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .bl{ align-items:baseline; }
    .shop{ background-color:#ffffff; padding:24rpx; }
    .logo{ width:110rpx; height:110rpx; border-radius:14rpx; }
    .scol{ flex:1; margin-left:20rpx; }
    .sname{ display:block; font-size:32rpx; color:#1a1a1a; font-weight:bold; margin-bottom:12rpx; }
    .score{ font-size:22rpx; color:#ff9500; margin-right:20rpx; }
    .sales{ font-size:22rpx; color:#999999; margin-right:20rpx; }
    .time{ font-size:22rpx; color:#999999; }
    .body{ align-items:flex-start; }
    .cats{ width:180rpx; background-color:#f0f1f3; }
    .cat{ display:flex; flex-direction:row; align-items:center; justify-content:space-between; padding:30rpx 18rpx; }
    .caton{ background-color:#ffffff; }
    .ctext{ font-size:26rpx; color:#555555; }
    .cton{ color:#ff6b35; font-weight:bold; }
    .cbadge{ background-color:#ff3b30; border-radius:16rpx; padding:2rpx 10rpx; }
    .cbt{ font-size:18rpx; color:#ffffff; }
    .dishes{ flex:1; background-color:#ffffff; padding:20rpx 24rpx; }
    .dhead{ display:block; font-size:26rpx; color:#999999; margin-bottom:12rpx; }
    .dish{ padding:20rpx 0; border-bottom:1rpx solid #f5f5f5; align-items:flex-start; }
    .dimg{ width:140rpx; height:140rpx; border-radius:12rpx; }
    .dcol{ flex:1; margin-left:18rpx; }
    .dname{ display:block; font-size:28rpx; color:#1a1a1a; font-weight:bold; }
    .ddesc{ display:block; font-size:22rpx; color:#999999; margin-top:8rpx; line-height:32rpx; }
    .dp{ font-size:32rpx; color:#ff3b30; font-weight:bold; margin-top:12rpx; }
    .dsale{ font-size:20rpx; color:#bbbbbb; margin-left:14rpx; }
    .plus{ width:48rpx; height:48rpx; border-radius:24rpx; background-color:#ff6b35; display:flex; align-items:center; justify-content:center; margin-top:80rpx; }
    .ptext{ font-size:32rpx; color:#ffffff; }
    .bag{ background-color:#2b2b2b; padding:18rpx 24rpx; margin-top:16rpx; }
    .bicon{ width:88rpx; height:88rpx; border-radius:44rpx; background-color:#ff6b35; display:flex; align-items:center; justify-content:center; }
    .bnum{ font-size:30rpx; color:#ffffff; font-weight:bold; }
    .bcol{ flex:1; margin-left:20rpx; }
    .btotal{ display:block; font-size:34rpx; color:#ffffff; font-weight:bold; }
    .bfee{ display:block; font-size:20rpx; color:#bbbbbb; margin-top:6rpx; }
    .checkout{ background-color:#07c160; border-radius:40rpx; padding:22rpx 42rpx; }
    .cktext{ font-size:30rpx; color:#ffffff; font-weight:bold; }
    "#;
    let data = json!({
        "cats":[{"n":"热销","b":"12"},{"n":"面包","b":""},{"n":"蛋糕","b":"3"},{"n":"饮品","b":""},{"n":"套餐","b":""}],
        "dishes":[
            {"n":"手工黄油可颂","d":"48层酥皮 每日现烤","p":"18","s":312,"img":"doc/gallery/assets/p_cake.jpg"},
            {"n":"轻乳酪芝士蛋糕","d":"低糖配方 冷藏更佳","p":"32","s":198,"img":"doc/gallery/assets/p_coffee.jpg"},
            {"n":"手冲单品咖啡","d":"耶加雪菲 中浅烘","p":"26","s":455,"img":"doc/gallery/assets/p_thermos.jpg"}
        ]
    });
    render("42_food_order", 0xF5F6F8, wxml, wxss, data);
}

// 43. 会员中心（渐变会员卡 + 权益）
fn member_center() {
    let wxml = r##"
    <view class="page">
        <view class="card">
            <view class="crow"><text class="lv">黄金会员</text><text class="exp">有效期至 2026-07-31</text></view>
            <text class="cname">尊享 8 项专属权益</text>
            <view class="prow"><view class="pbar"><view class="pfill"></view></view><text class="ptext">2400 / 5000 成长值</text></view>
            <view class="renew"><text class="rtext">立即续费</text></view>
        </view>
        <view class="block">
            <text class="bhead">我的权益</text>
            <view class="grid">
                <view class="gitem" wx:for="{{rights}}" wx:key="n">
                    <icon type="{{item.i}}" size="26" color="{{item.c}}"></icon>
                    <text class="gname">{{item.n}}</text>
                </view>
            </view>
        </view>
        <view class="block">
            <text class="bhead">会员任务</text>
            <view class="task" wx:for="{{tasks}}" wx:key="n">
                <view class="tcol"><text class="tname">{{item.n}}</text><text class="tdesc">{{item.d}}</text></view>
                <view class="tbtn {{item.done ? 'tdone' : ''}}"><text class="tbt {{item.done ? 'tbtd' : ''}}">{{item.done ? '已完成' : '去完成'}}</text></view>
            </view>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f5f6f8; padding:20rpx; }
    .card{ background:linear-gradient(135deg,#3a3f4b,#1c1f26); border-radius:20rpx; padding:32rpx; }
    .crow{ display:flex; flex-direction:row; align-items:baseline; justify-content:space-between; }
    .lv{ font-size:38rpx; color:#f7d67a; font-weight:bold; }
    .exp{ font-size:22rpx; color:rgba(255,255,255,0.65); }
    .cname{ display:block; font-size:26rpx; color:rgba(255,255,255,0.85); margin-top:16rpx; }
    .prow{ display:flex; flex-direction:row; align-items:center; margin-top:26rpx; }
    .pbar{ flex:1; height:12rpx; border-radius:6rpx; background-color:rgba(255,255,255,0.2); overflow:hidden; }
    .pfill{ width:288rpx; height:12rpx; border-radius:6rpx; background:linear-gradient(90deg,#f7d67a,#e0a94a); }
    .ptext{ font-size:20rpx; color:rgba(255,255,255,0.7); margin-left:16rpx; }
    .renew{ background:linear-gradient(90deg,#f7d67a,#e0a94a); border-radius:36rpx; padding:20rpx; margin-top:28rpx; display:flex; align-items:center; justify-content:center; }
    .rtext{ font-size:30rpx; color:#4a3611; font-weight:bold; }
    .block{ background-color:#ffffff; border-radius:16rpx; padding:24rpx; margin-top:20rpx; }
    .bhead{ display:block; font-size:30rpx; color:#1a1a1a; font-weight:bold; margin-bottom:16rpx; }
    .grid{ display:flex; flex-direction:row; flex-wrap:wrap; }
    .gitem{ width:25%; display:flex; flex-direction:column; align-items:center; padding:20rpx 0; }
    .gname{ font-size:22rpx; color:#666666; margin-top:12rpx; }
    .task{ display:flex; flex-direction:row; align-items:center; justify-content:space-between; padding:22rpx 0; border-bottom:1rpx solid #f5f5f5; }
    .tcol{ flex:1; }
    .tname{ display:block; font-size:28rpx; color:#1a1a1a; }
    .tdesc{ display:block; font-size:22rpx; color:#999999; margin-top:8rpx; }
    .tbtn{ background-color:#ff6b35; border-radius:28rpx; padding:12rpx 26rpx; }
    .tdone{ background-color:#f2f2f2; }
    .tbt{ font-size:24rpx; color:#ffffff; }
    .tbtd{ color:#999999; }
    "#;
    let data = json!({
        "rights":[
            {"n":"生日礼包","i":"success","c":"#ff6b35"},{"n":"免运特权","i":"waiting","c":"#3a7bd5"},
            {"n":"专属客服","i":"info","c":"#07c160"},{"n":"积分加倍","i":"warn","c":"#f0a020"},
            {"n":"退货无忧","i":"success","c":"#7b61ff"},{"n":"新品优先","i":"info","c":"#ff2d70"},
            {"n":"每月券包","i":"waiting","c":"#00b3a4"},{"n":"线下门店","i":"warn","c":"#e0a94a"}
        ],
        "tasks":[
            {"n":"每日签到","d":"连续签到额外奖励 +50","done":true},
            {"n":"完成一笔订单","d":"成长值 +200","done":false},
            {"n":"分享给好友","d":"成长值 +30","done":false}
        ]
    });
    render("43_member_center", 0xF5F6F8, wxml, wxss, data);
}

// 44. 支付确认（收银台）
fn payment_confirm() {
    let wxml = r##"
    <view class="page">
        <view class="amount">
            <text class="alabel">支付金额</text>
            <view class="arow"><text class="cur">¥</text><text class="anum">248.00</text></view>
            <text class="ashop">巷子口·手作烘焙</text>
        </view>
        <view class="block">
            <view class="prow" wx:for="{{methods}}" wx:key="n">
                <icon type="{{item.i}}" size="22" color="{{item.c}}"></icon>
                <view class="pcol"><text class="pname">{{item.n}}</text><text wx:if="{{item.d}}" class="pdesc">{{item.d}}</text></view>
                <icon wx:if="{{item.on}}" type="success" size="18" color="#07c160"></icon>
                <icon wx:else type="circle" size="18" color="#cccccc"></icon>
            </view>
        </view>
        <view class="block">
            <view class="drow"><text class="dl">商品金额</text><text class="dv">¥268.00</text></view>
            <view class="drow"><text class="dl">优惠券</text><text class="dv dred">-¥20.00</text></view>
            <view class="drow"><text class="dl">配送费</text><text class="dv">¥0.00</text></view>
        </view>
        <view class="bar"><button class="pay">确认支付 ¥248.00</button></view>
        <text class="tip">支付即表示同意《服务协议》</text>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f5f6f8; }
    .amount{ background-color:#ffffff; padding:60rpx 0 48rpx; display:flex; flex-direction:column; align-items:center; }
    .alabel{ font-size:26rpx; color:#999999; }
    .arow{ display:flex; flex-direction:row; align-items:baseline; margin-top:18rpx; }
    .cur{ font-size:36rpx; color:#1a1a1a; font-weight:bold; }
    .anum{ font-size:76rpx; color:#1a1a1a; font-weight:bold; }
    .ashop{ font-size:24rpx; color:#999999; margin-top:16rpx; }
    .block{ background-color:#ffffff; margin-top:20rpx; padding:8rpx 24rpx; }
    .prow{ display:flex; flex-direction:row; align-items:center; padding:26rpx 0; border-bottom:1rpx solid #f5f5f5; }
    .pcol{ flex:1; margin-left:18rpx; }
    .pname{ display:block; font-size:30rpx; color:#1a1a1a; }
    .pdesc{ display:block; font-size:22rpx; color:#ff6b35; margin-top:6rpx; }
    .drow{ display:flex; flex-direction:row; align-items:center; justify-content:space-between; padding:22rpx 0; border-bottom:1rpx solid #f7f7f7; }
    .dl{ font-size:28rpx; color:#666666; }
    .dv{ font-size:28rpx; color:#1a1a1a; }
    .dred{ color:#ff3b30; }
    .bar{ padding:36rpx 24rpx 12rpx; }
    .pay{ background-color:#07c160; color:#ffffff; border-radius:46rpx; font-size:32rpx; font-weight:bold; }
    .tip{ display:block; font-size:20rpx; color:#b0b0b0; text-align:center; margin-top:16rpx; }
    "#;
    let data = json!({"methods":[
        {"n":"微信支付","d":"推荐使用","i":"success","c":"#07c160","on":true},
        {"n":"支付宝","d":"","i":"info","c":"#1677ff","on":false},
        {"n":"银行卡","d":"招商银行(3921)","i":"waiting","c":"#f0a020","on":false}
    ]});
    render("44_payment", 0xF5F6F8, wxml, wxss, data);
}

// 45. 评价晒单（星级 + 图片）
fn review_list() {
    let wxml = r##"
    <view class="page">
        <view class="summary row">
            <view class="scol"><text class="sbig">4.9</text><text class="ssub">综合评分</text></view>
            <view class="bars">
                <view class="brow" wx:for="{{bars}}" wx:key="l">
                    <text class="bl">{{item.l}}</text>
                    <view class="btrack"><view class="bfill" style="width:{{item.w}}rpx"></view></view>
                    <text class="bp">{{item.p}}%</text>
                </view>
            </view>
        </view>
        <view class="chips row">
            <text class="chip chipon">全部 236</text><text class="chip">有图 88</text><text class="chip">好评 214</text><text class="chip">追评 12</text>
        </view>
        <view class="rv" wx:for="{{reviews}}" wx:key="u">
            <view class="row">
                <image class="uimg" src="doc/gallery/assets/avatar1.jpg" mode="aspectFill" />
                <view class="ucol"><text class="uname">{{item.u}}</text><text class="stars">{{item.s}}</text></view>
                <text class="date">{{item.d}}</text>
            </view>
            <text class="rtext">{{item.c}}</text>
            <view wx:if="{{item.pics}}" class="pics row">
                <image class="pic" src="doc/gallery/assets/p_sneakers.jpg" mode="aspectFill" />
                <image class="pic" src="doc/gallery/assets/p_backpack.jpg" mode="aspectFill" />
                <image class="pic" src="doc/gallery/assets/p_watch.jpg" mode="aspectFill" />
            </view>
            <text class="spec">规格：{{item.spec}}</text>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f5f6f8; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .summary{ background-color:#ffffff; padding:32rpx 24rpx; }
    .scol{ display:flex; flex-direction:column; align-items:center; width:180rpx; }
    .sbig{ font-size:72rpx; color:#ff9500; font-weight:bold; }
    .ssub{ font-size:22rpx; color:#999999; margin-top:6rpx; }
    .bars{ flex:1; }
    .brow{ display:flex; flex-direction:row; align-items:center; padding:6rpx 0; }
    .bl{ font-size:22rpx; color:#666666; width:70rpx; }
    .btrack{ flex:1; height:10rpx; border-radius:5rpx; background-color:#f0f0f0; margin:0 14rpx; overflow:hidden; }
    .bfill{ height:10rpx; border-radius:5rpx; background-color:#ff9500; }
    .bp{ font-size:20rpx; color:#999999; width:60rpx; text-align:right; }
    .chips{ background-color:#ffffff; padding:20rpx 24rpx; flex-wrap:wrap; margin-top:2rpx; }
    .chip{ font-size:24rpx; color:#666666; background-color:#f5f5f5; border-radius:24rpx; padding:10rpx 22rpx; margin-right:16rpx; }
    .chipon{ color:#ff6b35; background-color:#fff1eb; }
    .rv{ background-color:#ffffff; padding:24rpx; margin-top:16rpx; }
    .uimg{ width:64rpx; height:64rpx; border-radius:32rpx; }
    .ucol{ flex:1; margin-left:16rpx; }
    .uname{ display:block; font-size:26rpx; color:#333333; }
    .stars{ display:block; font-size:22rpx; color:#ff9500; margin-top:6rpx; }
    .date{ font-size:22rpx; color:#b0b0b0; }
    .rtext{ display:block; font-size:28rpx; color:#333333; line-height:42rpx; margin-top:18rpx; }
    .pics{ margin-top:16rpx; }
    .pic{ width:170rpx; height:170rpx; border-radius:10rpx; margin-right:12rpx; }
    .spec{ display:block; font-size:22rpx; color:#999999; margin-top:16rpx; }
    "#;
    let data = json!({
        "bars":[{"l":"5 星","w":420,"p":88},{"l":"4 星","w":48,"p":9},{"l":"3 星","w":12,"p":2},{"l":"2 星","w":4,"p":1}],
        "reviews":[
            {"u":"跑步的猫","s":"★★★★★","d":"07-20","c":"鞋子很轻，跑了两次感觉回弹不错，鞋码正常，日常穿也好搭。","pics":true,"spec":"蓝白 / 42"},
            {"u":"匿名用户","s":"★★★★☆","d":"07-18","c":"整体满意，只是鞋盒有点压扁，物流速度快。","pics":false,"spec":"黑色 / 41"}
        ]
    });
    render("45_reviews", 0xF5F6F8, wxml, wxss, data);
}

// 46. 搜索结果（联想 + 筛选）
fn search_result() {
    let wxml = r##"
    <view class="page">
        <view class="sbar row">
            <view class="sinput row"><icon type="search" size="14" color="#999999"></icon><text class="skey">蓝牙耳机</text><icon type="close" size="12" color="#bbbbbb"></icon></view>
            <text class="scancel">搜索</text>
        </view>
        <view class="filters row">
            <text class="f fon">综合</text><text class="f">销量</text><text class="f">价格 ↑</text><text class="f">好评</text><text class="f">筛选 ▾</text>
        </view>
        <view class="hot row">
            <text class="hlabel">大家还在搜</text>
            <text class="htag" wx:for="{{hots}}" wx:key="*this">{{item}}</text>
        </view>
        <view class="list">
            <view class="item row" wx:for="{{items}}" wx:key="n">
                <image class="iimg" src="{{item.img}}" mode="aspectFill" />
                <view class="icol">
                    <text class="iname">{{item.n}}</text>
                    <view class="row"><text class="itag" wx:for="{{item.tags}}" wx:for-item="tg" wx:key="*this">{{tg}}</text></view>
                    <view class="row bl"><text class="ip">¥{{item.p}}</text><text class="isale">{{item.s}}人付款</text></view>
                </view>
            </view>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f5f6f8; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .bl{ align-items:baseline; }
    .sbar{ background-color:#ffffff; padding:18rpx 24rpx; }
    .sinput{ flex:1; height:66rpx; border-radius:33rpx; background-color:#f2f3f5; padding:0 20rpx; justify-content:space-between; }
    .skey{ flex:1; font-size:28rpx; color:#333333; margin-left:12rpx; }
    .scancel{ font-size:28rpx; color:#ff6b35; margin-left:20rpx; }
    .filters{ background-color:#ffffff; padding:18rpx 24rpx; justify-content:space-between; border-top:1rpx solid #f5f5f5; }
    .f{ font-size:26rpx; color:#666666; }
    .fon{ color:#ff6b35; font-weight:bold; }
    .hot{ background-color:#ffffff; padding:18rpx 24rpx; flex-wrap:wrap; margin-top:2rpx; }
    .hlabel{ font-size:22rpx; color:#999999; margin-right:16rpx; }
    .htag{ font-size:22rpx; color:#666666; background-color:#f5f5f5; border-radius:20rpx; padding:8rpx 18rpx; margin:6rpx 12rpx 6rpx 0; }
    .list{ margin-top:16rpx; }
    .item{ background-color:#ffffff; padding:22rpx 24rpx; border-bottom:1rpx solid #f5f5f5; align-items:flex-start; }
    .iimg{ width:190rpx; height:190rpx; border-radius:12rpx; }
    .icol{ flex:1; margin-left:20rpx; }
    .iname{ display:block; font-size:28rpx; color:#1a1a1a; line-height:40rpx; }
    .itag{ font-size:18rpx; color:#ff3b30; border:1rpx solid #ffd0cb; border-radius:4rpx; padding:2rpx 8rpx; margin:14rpx 10rpx 0 0; }
    .ip{ font-size:36rpx; color:#ff3b30; font-weight:bold; margin-top:16rpx; }
    .isale{ font-size:20rpx; color:#b0b0b0; margin-left:16rpx; }
    "#;
    let data = json!({
        "hots":["降噪耳机","蓝牙5.3","运动耳机","开放式"],
        "items":[
            {"n":"无线蓝牙耳机 Pro 主动降噪 长续航","p":"199","s":"2.3万","img":"doc/gallery/assets/p_earbuds.jpg","tags":["包邮","30天无忧"]},
            {"n":"运动挂耳式蓝牙耳机 IPX7 防水","p":"149","s":"8621","img":"doc/gallery/assets/p_watch.jpg","tags":["新品"]},
            {"n":"开放式空气传导耳机 不入耳","p":"329","s":"1245","img":"doc/gallery/assets/p_phone2.jpg","tags":["旗舰","分期免息"]}
        ]
    });
    render("46_search_result", 0xF5F6F8, wxml, wxss, data);
}

// 47. 消息中心（会话列表）
fn message_center() {
    let wxml = r##"
    <view class="page">
        <view class="head row"><text class="htitle">消息</text><text class="hread">全部已读</text></view>
        <view class="quick row">
            <view class="q" wx:for="{{quicks}}" wx:key="n">
                <view class="qico" style="background-color:{{item.c}}"><icon type="{{item.i}}" size="20" color="#ffffff"></icon></view>
                <text class="qname">{{item.n}}</text>
            </view>
        </view>
        <view class="conv row" wx:for="{{convs}}" wx:key="n">
            <image class="cimg" src="doc/gallery/assets/avatar1.jpg" mode="aspectFill" />
            <view class="ccol">
                <view class="crow"><text class="cname">{{item.n}}</text><text class="ctime">{{item.t}}</text></view>
                <view class="crow"><text class="cmsg">{{item.m}}</text><view wx:if="{{item.u}}" class="cbadge"><text class="cbt">{{item.u}}</text></view></view>
            </view>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f5f6f8; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .head{ background-color:#ffffff; padding:24rpx; justify-content:space-between; }
    .htitle{ font-size:36rpx; color:#1a1a1a; font-weight:bold; }
    .hread{ font-size:24rpx; color:#999999; }
    .quick{ background-color:#ffffff; padding:24rpx 0 28rpx; justify-content:space-around; border-bottom:1rpx solid #f2f2f2; }
    .q{ display:flex; flex-direction:column; align-items:center; width:150rpx; }
    .qico{ width:88rpx; height:88rpx; border-radius:44rpx; display:flex; align-items:center; justify-content:center; }
    .qname{ font-size:22rpx; color:#666666; margin-top:12rpx; }
    .conv{ background-color:#ffffff; padding:22rpx 24rpx; border-bottom:1rpx solid #f5f5f5; }
    .cimg{ width:96rpx; height:96rpx; border-radius:14rpx; }
    .ccol{ flex:1; margin-left:20rpx; }
    .crow{ display:flex; flex-direction:row; align-items:center; justify-content:space-between; }
    .cname{ font-size:30rpx; color:#1a1a1a; font-weight:bold; }
    .ctime{ font-size:20rpx; color:#b0b0b0; }
    .cmsg{ flex:1; font-size:26rpx; color:#999999; margin-top:12rpx; }
    .cbadge{ background-color:#ff3b30; border-radius:20rpx; padding:2rpx 12rpx; margin-top:12rpx; }
    .cbt{ font-size:20rpx; color:#ffffff; }
    "#;
    let data = json!({
        "quicks":[
            {"n":"订单","i":"success","c":"#ff6b35"},{"n":"物流","i":"waiting","c":"#3a7bd5"},
            {"n":"优惠","i":"warn","c":"#ff2d70"},{"n":"客服","i":"info","c":"#07c160"}
        ],
        "convs":[
            {"n":"官方客服","m":"您的售后申请已受理，预计 24 小时内处理","t":"09:24","u":"2"},
            {"n":"物流助手","m":"包裹已到达浦东新区营业点，正在派送","t":"08:10","u":""},
            {"n":"活动通知","m":"你关注的商品降价了，快来看看","t":"昨天","u":"5"},
            {"n":"巷子口·手作烘焙","m":"新品可颂上线，会员立减 5 元","t":"周二","u":""}
        ]
    });
    render("47_message_center", 0xF5F6F8, wxml, wxss, data);
}

// 48. 签到打卡（日历格 + 奖励）
fn checkin_calendar() {
    let wxml = r##"
    <view class="page">
        <view class="hero">
            <text class="days">已连续签到 7 天</text>
            <text class="hint">再签 3 天可得 88 积分礼包</text>
            <view class="cbtn"><text class="cbtext">立即签到</text></view>
        </view>
        <view class="card">
            <view class="wrow"><text class="wd" wx:for="{{weeks}}" wx:key="*this">{{item}}</text></view>
            <view class="grid">
                <view class="cell {{item.s}}" wx:for="{{cells}}" wx:key="d">
                    <text class="ct {{item.s == 'on' ? 'cton' : ''}}">{{item.d}}</text>
                </view>
            </view>
        </view>
        <view class="card">
            <text class="rhead">签到奖励</text>
            <view class="rrow" wx:for="{{rewards}}" wx:key="n">
                <view class="rico"><text class="rd">{{item.d}}</text></view>
                <view class="rcol"><text class="rn">{{item.n}}</text><text class="rdesc">{{item.t}}</text></view>
                <view class="rbtn {{item.got ? 'rgot' : ''}}"><text class="rbt {{item.got ? 'rbtg' : ''}}">{{item.got ? '已领取' : '待解锁'}}</text></view>
            </view>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f5f6f8; padding-bottom:20rpx; }
    .hero{ background:linear-gradient(135deg,#ff8a5b,#ff3b30); padding:44rpx 28rpx; display:flex; flex-direction:column; align-items:center; }
    .days{ font-size:40rpx; color:#ffffff; font-weight:bold; }
    .hint{ font-size:24rpx; color:rgba(255,255,255,0.9); margin-top:14rpx; }
    .cbtn{ background-color:#ffffff; border-radius:40rpx; padding:20rpx 60rpx; margin-top:26rpx; }
    .cbtext{ font-size:30rpx; color:#ff3b30; font-weight:bold; }
    .card{ background-color:#ffffff; border-radius:16rpx; margin:20rpx; padding:24rpx; }
    .wrow{ display:flex; flex-direction:row; justify-content:space-around; margin-bottom:12rpx; }
    .wd{ font-size:22rpx; color:#999999; width:88rpx; text-align:center; }
    .grid{ display:flex; flex-direction:row; flex-wrap:wrap; }
    .cell{ width:88rpx; height:88rpx; display:flex; align-items:center; justify-content:center; margin:6rpx 6rpx; border-radius:44rpx; }
    .on{ background-color:#ff3b30; }
    .off{ background-color:#f7f7f7; }
    .ct{ font-size:26rpx; color:#666666; }
    .cton{ color:#ffffff; font-weight:bold; }
    .rhead{ display:block; font-size:30rpx; color:#1a1a1a; font-weight:bold; margin-bottom:12rpx; }
    .rrow{ display:flex; flex-direction:row; align-items:center; padding:20rpx 0; border-bottom:1rpx solid #f5f5f5; }
    .rico{ width:72rpx; height:72rpx; border-radius:36rpx; background-color:#fff1eb; display:flex; align-items:center; justify-content:center; }
    .rd{ font-size:24rpx; color:#ff6b35; font-weight:bold; }
    .rcol{ flex:1; margin-left:20rpx; }
    .rn{ display:block; font-size:28rpx; color:#1a1a1a; }
    .rdesc{ display:block; font-size:22rpx; color:#999999; margin-top:6rpx; }
    .rbtn{ background-color:#ff6b35; border-radius:26rpx; padding:10rpx 22rpx; }
    .rgot{ background-color:#f2f2f2; }
    .rbt{ font-size:22rpx; color:#ffffff; }
    .rbtg{ color:#999999; }
    "#;
    let cells: Vec<serde_json::Value> = (1..=28)
        .map(|d| json!({"d": d, "s": if d <= 7 { "on" } else { "off" }}))
        .collect();
    let data = json!({
        "weeks":["一","二","三","四","五","六","日"],
        "cells": cells,
        "rewards":[
            {"d":"3天","n":"10 积分","t":"已发放至账户","got":true},
            {"d":"7天","n":"无门槛券 5 元","t":"已发放至卡包","got":true},
            {"d":"10天","n":"88 积分礼包","t":"再签 3 天解锁","got":false}
        ]
    });
    render("48_checkin", 0xF5F6F8, wxml, wxss, data);
}

// 49. 行情看板（涨跌 + 迷你柱图）
fn market_board() {
    let wxml = r##"
    <view class="page">
        <view class="idx row">
            <view class="i" wx:for="{{indexes}}" wx:key="n">
                <text class="in">{{item.n}}</text>
                <text class="iv {{item.up ? 'up' : 'down'}}">{{item.v}}</text>
                <text class="ic {{item.up ? 'up' : 'down'}}">{{item.c}}</text>
            </view>
        </view>
        <view class="card">
            <view class="chead row"><text class="ctitle">自选</text><text class="cedit">编辑</text></view>
            <view class="srow row" wx:for="{{stocks}}" wx:key="code">
                <view class="scol"><text class="sname">{{item.n}}</text><text class="scode">{{item.code}}</text></view>
                <view class="bars row">
                    <view class="bar" style="height:{{item.b1}}rpx; background-color:{{item.up ? '#ff3b30' : '#07c160'}}"></view>
                    <view class="bar" style="height:{{item.b2}}rpx; background-color:{{item.up ? '#ff3b30' : '#07c160'}}"></view>
                    <view class="bar" style="height:{{item.b3}}rpx; background-color:{{item.up ? '#ff3b30' : '#07c160'}}"></view>
                    <view class="bar" style="height:{{item.b4}}rpx; background-color:{{item.up ? '#ff3b30' : '#07c160'}}"></view>
                    <view class="bar" style="height:{{item.b5}}rpx; background-color:{{item.up ? '#ff3b30' : '#07c160'}}"></view>
                </view>
                <view class="pcol"><text class="pv {{item.up ? 'up' : 'down'}}">{{item.p}}</text><view class="pc {{item.up ? 'bgup' : 'bgdown'}}"><text class="pct">{{item.pct}}</text></view></view>
            </view>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#12141a; padding-bottom:20rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .idx{ justify-content:space-around; padding:32rpx 0; }
    .i{ display:flex; flex-direction:column; align-items:center; }
    .in{ font-size:22rpx; color:#8b93a3; }
    .iv{ font-size:36rpx; font-weight:bold; margin-top:10rpx; }
    .ic{ font-size:20rpx; margin-top:8rpx; }
    .up{ color:#ff4d4f; }
    .down{ color:#22c55e; }
    .card{ background-color:#1b1e26; border-radius:16rpx; margin:0 20rpx; padding:24rpx; }
    .chead{ justify-content:space-between; margin-bottom:8rpx; }
    .ctitle{ font-size:30rpx; color:#e8eaed; font-weight:bold; }
    .cedit{ font-size:24rpx; color:#8b93a3; }
    .srow{ padding:22rpx 0; border-bottom:1rpx solid #262a33; justify-content:space-between; }
    .scol{ width:220rpx; }
    .sname{ display:block; font-size:28rpx; color:#e8eaed; }
    .scode{ display:block; font-size:20rpx; color:#6b7280; margin-top:8rpx; }
    .bars{ align-items:flex-end; height:60rpx; }
    .bar{ width:10rpx; border-radius:3rpx; margin-right:6rpx; }
    .pcol{ display:flex; flex-direction:column; align-items:flex-end; width:170rpx; }
    .pv{ font-size:30rpx; font-weight:bold; }
    .pc{ border-radius:6rpx; padding:4rpx 12rpx; margin-top:8rpx; }
    .bgup{ background-color:#ff4d4f; }
    .bgdown{ background-color:#22c55e; }
    .pct{ font-size:20rpx; color:#ffffff; }
    "#;
    let data = json!({
        "indexes":[
            {"n":"上证指数","v":"3218.46","c":"+0.82%","up":true},
            {"n":"深证成指","v":"10126.7","c":"+1.14%","up":true},
            {"n":"创业板指","v":"2038.55","c":"-0.36%","up":false}
        ],
        "stocks":[
            {"n":"贵州茅台","code":"600519","p":"1682.30","pct":"+1.26%","up":true,"b1":18,"b2":30,"b3":24,"b4":44,"b5":56},
            {"n":"宁德时代","code":"300750","p":"198.65","pct":"-0.84%","up":false,"b1":48,"b2":36,"b3":40,"b4":24,"b5":18},
            {"n":"中国平安","code":"601318","p":"46.72","pct":"+2.05%","up":true,"b1":14,"b2":22,"b3":34,"b4":40,"b5":52}
        ]
    });
    render("49_market_board", 0x12141A, wxml, wxss, data);
}

// 50. 酒店预订（日期区间 + 房型）
fn hotel_booking() {
    let wxml = r##"
    <view class="page">
        <image class="banner" src="doc/gallery/assets/banner.jpg" mode="aspectFill" />
        <view class="info">
            <text class="hname">外滩·江景精选酒店</text>
            <view class="row"><text class="star">★★★★★</text><text class="score">4.8 分</text><text class="cnt">2361 条点评</text></view>
            <text class="addr">黄浦区中山东一路 88 号 · 距外滩 200m</text>
        </view>
        <view class="dates row">
            <view class="d"><text class="dl">入住</text><text class="dv">07-25 周五</text></view>
            <view class="nights"><text class="nt">2 晚</text></view>
            <view class="d"><text class="dl">离店</text><text class="dv">07-27 周日</text></view>
        </view>
        <view class="rooms">
            <view class="room" wx:for="{{rooms}}" wx:key="n">
                <image class="rimg" src="{{item.img}}" mode="aspectFill" />
                <view class="rcol">
                    <text class="rn">{{item.n}}</text>
                    <text class="rd">{{item.d}}</text>
                    <view class="row"><text class="rtag" wx:for="{{item.tags}}" wx:for-item="tg" wx:key="*this">{{tg}}</text></view>
                </view>
                <view class="rright">
                    <text class="rp">¥{{item.p}}</text>
                    <view class="rbtn"><text class="rbt">订</text></view>
                </view>
            </view>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f5f6f8; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .banner{ width:100%; height:380rpx; }
    .info{ background-color:#ffffff; padding:24rpx; }
    .hname{ display:block; font-size:36rpx; color:#1a1a1a; font-weight:bold; margin-bottom:14rpx; }
    .star{ font-size:22rpx; color:#ff9500; margin-right:14rpx; }
    .score{ font-size:24rpx; color:#ff6b35; font-weight:bold; margin-right:14rpx; }
    .cnt{ font-size:22rpx; color:#999999; }
    .addr{ display:block; font-size:24rpx; color:#666666; margin-top:16rpx; line-height:34rpx; }
    .dates{ background-color:#ffffff; margin-top:16rpx; padding:24rpx; justify-content:space-around; }
    .d{ display:flex; flex-direction:column; align-items:center; }
    .dl{ font-size:22rpx; color:#999999; }
    .dv{ font-size:30rpx; color:#1a1a1a; font-weight:bold; margin-top:10rpx; }
    .nights{ background-color:#f2f3f5; border-radius:22rpx; padding:8rpx 20rpx; }
    .nt{ font-size:22rpx; color:#666666; }
    .rooms{ margin-top:16rpx; }
    .room{ background-color:#ffffff; padding:22rpx 24rpx; border-bottom:1rpx solid #f5f5f5; display:flex; flex-direction:row; align-items:center; }
    .rimg{ width:170rpx; height:130rpx; border-radius:12rpx; }
    .rcol{ flex:1; margin-left:20rpx; }
    .rn{ display:block; font-size:28rpx; color:#1a1a1a; font-weight:bold; }
    .rd{ display:block; font-size:22rpx; color:#999999; margin-top:8rpx; }
    .rtag{ font-size:18rpx; color:#07c160; border:1rpx solid #b7eb8f; border-radius:4rpx; padding:2rpx 8rpx; margin:12rpx 10rpx 0 0; }
    .rright{ display:flex; flex-direction:column; align-items:flex-end; }
    .rp{ font-size:34rpx; color:#ff3b30; font-weight:bold; }
    .rbtn{ background-color:#ff6b35; border-radius:24rpx; padding:10rpx 26rpx; margin-top:14rpx; }
    .rbt{ font-size:24rpx; color:#ffffff; }
    "#;
    let data = json!({"rooms":[
        {"n":"高级江景大床房","d":"38㎡ · 双早 · 免费取消","p":"899","img":"doc/gallery/assets/p_phone.jpg","tags":["含双早","可取消"]},
        {"n":"行政套房","d":"62㎡ · 双早 · 行政酒廊","p":"1680","img":"doc/gallery/assets/p_watch.jpg","tags":["酒廊","延迟退房"]},
        {"n":"标准双床房","d":"32㎡ · 无早 · 特惠价","p":"659","img":"doc/gallery/assets/p_backpack.jpg","tags":["特惠"]}
    ]});
    render("50_hotel_booking", 0xF5F6F8, wxml, wxss, data);
}

// 51. 运动健康（环形进度 + 数据块）
fn health_dashboard() {
    let wxml = r##"
    <view class="page">
        <view class="hero">
            <view class="ring">
                <view class="ringbg"></view>
                <view class="rcenter"><text class="steps">9,268</text><text class="slabel">步</text></view>
            </view>
            <text class="goal">目标 10,000 步 · 完成 93%</text>
        </view>
        <view class="cards row">
            <view class="c" wx:for="{{metrics}}" wx:key="l">
                <icon type="{{item.i}}" size="22" color="{{item.c}}"></icon>
                <text class="cv">{{item.v}}</text>
                <text class="cl">{{item.l}}</text>
            </view>
        </view>
        <view class="block">
            <text class="bhead">本周运动</text>
            <view class="week row">
                <view class="wcol" wx:for="{{week}}" wx:key="d">
                    <view class="wtrack"><view class="wfill" style="height:{{item.h}}rpx"></view></view>
                    <text class="wd">{{item.d}}</text>
                </view>
            </view>
        </view>
        <view class="block">
            <text class="bhead">今日计划</text>
            <view class="plan" wx:for="{{plans}}" wx:key="n">
                <view class="pico" style="background-color:{{item.c}}"><icon type="{{item.i}}" size="18" color="#ffffff"></icon></view>
                <view class="pcol"><text class="pn">{{item.n}}</text><text class="pd">{{item.d}}</text></view>
                <progress percent="{{item.p}}" show-info="true" activeColor="{{item.c}}"></progress>
            </view>
        </view>
    </view>"##;
    let wxss = r#"
    .page{ background-color:#f5f6f8; padding-bottom:20rpx; }
    .row{ display:flex; flex-direction:row; align-items:center; }
    .hero{ background:linear-gradient(135deg,#00c6a9,#0a94ff); padding:44rpx 0 38rpx; display:flex; flex-direction:column; align-items:center; }
    .ring{ width:260rpx; height:260rpx; border-radius:130rpx; background-color:rgba(255,255,255,0.22); display:flex; align-items:center; justify-content:center; }
    .ringbg{ position:absolute; width:220rpx; height:220rpx; border-radius:110rpx; border:14rpx solid rgba(255,255,255,0.85); }
    .rcenter{ display:flex; flex-direction:column; align-items:center; }
    .steps{ font-size:56rpx; color:#ffffff; font-weight:bold; }
    .slabel{ font-size:24rpx; color:rgba(255,255,255,0.9); margin-top:4rpx; }
    .goal{ font-size:24rpx; color:#ffffff; margin-top:26rpx; }
    .cards{ justify-content:space-around; background-color:#ffffff; margin:20rpx; border-radius:16rpx; padding:26rpx 0; }
    .c{ display:flex; flex-direction:column; align-items:center; width:170rpx; }
    .cv{ font-size:32rpx; color:#1a1a1a; font-weight:bold; margin-top:12rpx; }
    .cl{ font-size:20rpx; color:#999999; margin-top:6rpx; }
    .block{ background-color:#ffffff; border-radius:16rpx; margin:0 20rpx 20rpx; padding:24rpx; }
    .bhead{ display:block; font-size:30rpx; color:#1a1a1a; font-weight:bold; margin-bottom:18rpx; }
    .week{ justify-content:space-between; align-items:flex-end; }
    .wcol{ display:flex; flex-direction:column; align-items:center; width:88rpx; }
    .wtrack{ width:32rpx; height:170rpx; border-radius:16rpx; background-color:#f0f1f3; display:flex; flex-direction:column; justify-content:flex-end; overflow:hidden; }
    .wfill{ width:32rpx; border-radius:16rpx; background:linear-gradient(180deg,#00c6a9,#0a94ff); }
    .wd{ font-size:20rpx; color:#999999; margin-top:12rpx; }
    .plan{ display:flex; flex-direction:row; align-items:center; padding:20rpx 0; border-bottom:1rpx solid #f5f5f5; }
    .pico{ width:64rpx; height:64rpx; border-radius:32rpx; display:flex; align-items:center; justify-content:center; }
    .pcol{ width:220rpx; margin-left:18rpx; }
    .pn{ display:block; font-size:28rpx; color:#1a1a1a; }
    .pd{ display:block; font-size:20rpx; color:#999999; margin-top:6rpx; }
    "#;
    let data = json!({
        "metrics":[
            {"l":"公里","v":"6.8","i":"success","c":"#00c6a9"},
            {"l":"千卡","v":"412","i":"warn","c":"#ff6b35"},
            {"l":"活跃分钟","v":"58","i":"waiting","c":"#0a94ff"},
            {"l":"心率","v":"72","i":"info","c":"#ff2d70"}
        ],
        "week":[{"d":"一","h":90},{"d":"二","h":130},{"d":"三","h":70},{"d":"四","h":150},{"d":"五","h":110},{"d":"六","h":160},{"d":"日","h":140}],
        "plans":[
            {"n":"晨跑 5 公里","d":"已完成 4.6 公里","p":92,"i":"success","c":"#00c6a9"},
            {"n":"力量训练","d":"胸背 · 40 分钟","p":60,"i":"warn","c":"#ff6b35"},
            {"n":"拉伸放松","d":"未开始","p":0,"i":"waiting","c":"#0a94ff"}
        ]
    });
    render("51_health_dashboard", 0xF5F6F8, wxml, wxss, data);
}
