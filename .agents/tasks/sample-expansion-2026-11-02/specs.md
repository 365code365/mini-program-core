# 子任务分工（每个子任务只做自己那一节；通用约定见 brief.md）

新增小程序：每个 ≥7 页、有自定义 tabBar（bike 可例外）、业务闭环完整、数据真实。「必须覆盖」里的每项都要在界面上有真实用途，不是摆设。

---

## N1

1. `sample/insure`「安心保」（金融·保险）：首页（险种分类、热销产品 swiper）、产品列表（筛选）、产品详情（保障责任表、rich-text 条款、常见问题折叠）、
   保费测算（年龄 / 保额 / 保障期 → 实时算保费）、投保填写、支付结果、我的保单（列表 + 详情 + 电子保单）、理赔申请（上传材料 + 进度时间轴）、我的。
   必须覆盖：`form` + `bindsubmit` / `bindreset`（e.detail.value 取值）、`label` 包 radio/checkbox 点文字也能选、`radio-group`、`checkbox-group`（健康告知）、
   `picker mode="multiSelector"`（职业类别两级联动，bindcolumnchange 改第二列）、`picker mode="date"`（生日）、`input type="idcard"` / `type="digit"` 与身份证校验位校验、
   `rich-text`（nodes 用 HTML 字符串）、`page-container`（半屏弹出保障详情，bind:clickoverlay 关闭）、`wx.showModal({editable:true})`（填写推荐码）、
   `wx.requestPayment` + fail 降级为模拟收银台页、`wx.navigateTo` 的 `events` / `EventChannel`（测算页把结果回传给详情页）、`canvas type="2d"`（保费随年龄变化折线图，SelectorQuery `.fields({node:true,size:true})` 拿 node）。
2. `sample/auto-care`「车管家」（汽车·养护）：首页（我的爱车卡片、保养提醒、服务宫格）、门店（列表 + `map` 门店分布）、门店详情、服务项目选择、预约（日期 + 时段）、
   订单列表 / 详情（待服务→服务中→已完成→评价）、爱车档案（添加车辆：品牌车型选择、车牌输入）、违章查询结果、我的。
   必须覆盖：`map`（markers、`bindmarkertap`、`include-points`；`wx.createMapContext().moveToLocation`，失败降级）、`wx.getLocation` / `wx.chooseLocation` / `wx.openLocation`（fail 回退默认坐标并提示）、
   `wx.makePhoneCall`（fail 时 showModal 显示号码 + `wx.setClipboardData`）、`wx.scanCode`（扫 VIN，fail 弹手动输入）、`picker mode="time"` + `mode="selector"`、
   `swiper` 的 `display-multiple-items` / `previous-margin` / `next-margin`（多车卡片）、`scroll-view scroll-x` 品牌 + 字母索引、`wx.showActionSheet`、
   `movable-area` + `movable-view`（订单详情里可拖动的「联系门店」悬浮按钮，direction="all"）、storage 存车辆与订单。

## N2

1. `sample/salon`「型格美业」（生活服务·美发美甲预约）：首页（门店信息、热门项目、作品集 swiper）、项目列表（分类 tab）、项目详情、发型师列表 / 主页（作品、评价）、
   预约（项目→发型师→日期时段→确认）、预约结果、我的预约（取消 / 改约）、会员卡（储值、积分、等级进度）、我的。
   必须覆盖：`scroll-view scroll-x` + `scroll-into-view` + `scroll-with-animation`、`picker-view` + `picker-view-column`（嵌入式时段选择，value 是 number 数组）、
   `draggable-sheet`（预约确认的可拖拽底部面板：initial-child-size / min-child-size / max-child-size / snap）、`wx.createAnimation`（预约成功对勾缩放，export()）、
   `this.animate(selector, keyframes, duration, cb)`、`wx.previewImage`、`wx.requestSubscribeMessage`（fail 降级）、`wx.addPhoneCalendar`（fail 降级）、
   `progress`（会员等级，active）、`icon` 组件（success / waiting / warn）、评分星级自定义组件（properties + observers + triggerEvent）。
2. `sample/home-service`「好管家」（生活服务·家政保洁维修）：首页（服务宫格、限时优惠、阿姨推荐）、服务详情（价格表、服务标准、须知）、下单（地址、上门时间、时长、备注、现场照片）、
   地址管理（列表 / 新增 / 编辑 / 设默认、region picker）、订单列表（待上门 / 服务中 / 待评价 / 已完成）、订单详情（进度、师傅、联系、取消原因）、评价（星级 + 标签 + 文字 + 图片）、我的（优惠券、发票）。
   必须覆盖自定义组件全家桶：多 slot（`options:{multipleSlots:true}`）、`behaviors`（表单校验 mixin）、`relations`（`<form-group>` / `<form-item>`，linked 回调注册）、`externalClasses`、
   `observers`（多字段 + 通配 `**`）、`lifetimes` / `pageLifetimes`、`this.selectComponent` / `selectAllComponents`、`triggerEvent`（detail + bubbles/composed）；
   另外：`textarea auto-height` + 字数计数、`picker mode="region"`、`wx.chooseMedia`（fail 降级示例照片）、`wx.uploadFile`（fail 本地保存）、`checkbox` 自定义颜色、`switch`、
   `wx.showLoading` / `hideLoading`、`wx.navigateBack({delta:2})`、`wx.redirectTo`。

## N3

1. `sample/secondhand`「闲转」（购物·二手闲置）：首页（分类、双列瀑布流）、搜索（历史、热搜、防抖联想、排序筛选）、宝贝详情（图集、卖家、留言）、发布闲置、
   私信列表 + 聊天页（文本、出价卡片、交易状态）、下单 / 确认收货、我的发布（上架 / 下架 / 擦亮 / 删除）、我的收藏、我的。
   必须覆盖：`wx.createIntersectionObserver(this).relativeToViewport().observe`（曝光统计 + 图片渐显）、`onReachBottom` 分页 + `onPullDownRefresh` / `wx.stopPullDownRefresh`、
   `image lazy-load` + `widthFix` / `aspectFill` + `binderror` 占位、`bindlongpress`（ActionSheet）、`button open-type="share"` + `onShareAppMessage` + `onShareTimeline` + `wx.showShareMenu`、
   `wx.chooseMedia`（fail 降级）、`slider`（价格筛选）、`input confirm-type="search"` + `bindconfirm`、`wx.pageScrollTo` + onPageScroll 显隐回顶按钮、自定义 tabBar 未读红点、`wx.nextTick`。
2. `sample/scenic`「游山河」（文旅·景区门票导览）：首页、景区详情（宣传片 video、开放时间、须知、评价）、门票预订（日期 + 票种 + 游客 + 步进器）、支付结果、电子票（canvas 二维码）、
   语音导览（景点列表 + 讲解）、游览路线地图、订单 / 我的。
   必须覆盖：`video`（controls、poster、`wx.createVideoContext` play/pause/seek、bindtimeupdate、bindended）、`audio` 组件或 InnerAudioContext（python 合成 wav，每段 ≤ 60KB）、
   `wx.getBackgroundAudioManager`、`map` 的 `polyline` + `markers` + `callout`、`swiper vertical`、`wx.createCanvasContext` 画简化二维码 + `wx.canvasToTempFilePath` + `wx.saveImageToPhotosAlbum`（fail 降级）、
   `wx.setScreenBrightness` / `getScreenBrightness` + `wx.setKeepScreenOn`（离开恢复）、`picker mode="date"` 的 start/end（30 天内）。

## N4

1. `sample/bike`「青桔骑行」（出行·共享单车）：首页全屏地图（车辆 markers、定位、刷新、扫码开锁）、开锁（扫码 / 手动车号）、骑行中（计时、里程、费用累加、临时锁车、结束）、
   行程结算、行程记录（列表 + 详情轨迹）、钱包（余额、充值、骑行卡）、故障上报、我的。
   必须覆盖：`map` 全屏 + `cover-view` / `cover-image` 控件、`markers` + `bindregionchange`、`wx.createMapContext().getCenterLocation` / `moveToLocation`（fail 降级）、
   `wx.scanCode`（fail 降级为 `input type="number"` 弹层）、`wx.vibrateShort({type:'heavy'})` / `wx.vibrateLong`、`setInterval` 计时（onHide 记时间戳、onShow 补算）、
   `wx.getLocation`（fail 默认坐标）、`wx.startCompass` + `wx.onCompassChange`（fail 忽略）、`wx.getNetworkType` + `wx.onNetworkStatusChange`（弱网横幅）、`wx.requestPayment`（fail 模拟收银台）、`checkbox-group`。
2. `sample/property`「邻里物业」（生活服务·社区物业）：首页（公告轮播、服务宫格、待缴提醒）、公告（列表 + rich-text 详情）、报事报修 + 工单进度、物业缴费（明细、多选合并支付、记录）、
   访客通行（访客二维码）、社区投票（单选 / 多选、结果百分比）、活动报名、我的房屋（小区→楼栋→房号三级 picker）、我的。
   必须覆盖：`wx.createCanvasContext` 画访客码 + `wx.canvasToTempFilePath` + `wx.previewImage`、`progress`（show-info + active）、`picker mode="multiSelector"` 三列联动、
   `wx.makePhoneCall`（fail 复制）、`rich-text`（**数组形式**节点树）、`wx.showModal` 的 `confirmColor` / `cancelText`、`wx.showToast({icon:'none'})`、
   `swiper` autoplay + circular + bindchange 联动自定义指示器、checkbox 全选联动合计、`wx.setNavigationBarTitle` / `setNavigationBarColor`、`wx.getStorageInfoSync` + 清缓存。

## N5

1. `sample/gov-service`「云川办事」（政务·一网通办，虚构「云川市」）：首页（热门事项、按主题 / 部门分类）、事项详情（条件、材料、流程、时限、收费）、
   在线申报（多步向导：申请人→事项信息→材料→确认，每步校验、可回退保留、可保存草稿）、办件进度（时间轴）、预约取号、证照夹、我的办件、我的。
   必须覆盖：`wx.getFileSystemManager()` 的 `writeFile` / `readFile` / `access` / `unlink`（草稿 `${wx.env.USER_DATA_PATH}/draft.json`）、`wx.chooseMessageFile`（fail 降级）、
   `wx.downloadFile` + `wx.openDocument`（fail 降级）、`match-media`（`min-width="600"` 宽屏双栏 / 窄屏单栏）、`picker mode="region"`、`input type="idcard"`、
   `wx.setNavigationBarTitle`（每步改标题）、`wx.enableAlertBeforeUnload` / `disableAlertBeforeUnload`、`wx.reLaunch`、`icon`、步骤条自定义组件。
2. `sample/campus`「云川大学」（教育·校园服务）：首页（今日课程、通知、宫格）、课程表（周视图 7×12 网格、滑动切周、课程详情）、成绩（学期 multiSelector、GPA）、
   图书馆（检索、借阅、续借）、校园卡（余额、流水、充值）、空教室、通知详情、我的。
   必须覆盖：WXS（外链 `<wxs module src>` + 页内 `<wxs module>`）、`<template name>` + `<import>` + `<include>`、`display:grid`（grid-template-columns、grid-row 跨行）、
   `swiper` 受控 `current` + bindchange、`picker mode="multiSelector"`、`switch`、`wx.setClipboardData`、`onPullDownRefresh` / `wx.startPullDownRefresh`、
   自定义 tabBar 红点、`wx.getSystemInfoSync` / `wx.getWindowInfo`、`hover-class` + `hover-stay-time`。

## N6

1. `sample/museum`「云川博物院」（文旅·博物馆导览，虚构）：首页（`navigationStyle: custom` 沉浸头图、今日开放、展览推荐）、展厅、展品详情（大图、rich-text、讲解）、展览日历、
   参观预约（日期 + 时段 + 人数 + 实名）、预约凭证、我的收藏、我的。
   必须覆盖：`navigation-bar` 内置组件或自绘导航栏 + `wx.getMenuButtonBoundingClientRect`、`page-meta`（page-style / background-text-style / scroll-top）、`root-portal`（全屏看图层）、
   内置 `audio` 组件 + InnerAudioContext（python 合成 wav ≤ 60KB）、`scroll-view` 的 `refresher-enabled` + `bindrefresherrefresh` + `refresher-triggered`、
   `swiper` previous/next-margin、`wx.previewImage`、`onPageScroll` 导航栏渐变（节流）、`wx.setBackgroundColor` / `setBackgroundTextStyle`、`text selectable` / `user-select`、`image show-menu-by-longpress`。
2. `sample/pharmacy`「康源大药房」（医疗健康·网上药店）：首页（搜索、科室、常备药、资讯）、分类（左右双向联动）、药品详情（说明书折叠、规格、处方药标识）、购物车、
   处方上传与问诊（处方药必须先上传处方 / 问诊）、订单确认 + 结果、用药提醒（新增 / 编辑，今日待服）、我的。
   必须覆盖：`selectAll().boundingClientRect().exec()` 锚点 + `scroll-into-view` + `bindscroll` 联动、`this.animate` / `wx.createAnimation` 加购抛物线小球、步进器组件（`model:value` 或 triggerEvent）、
   `picker mode="time"`、`checkbox-group`（每周几）、`wx.addPhoneRepeatCalendar`（fail 降级）、`wx.chooseImage`（fail 降级）、`wx.showModal` confirmText、
   `wx.batchSetStorageSync` / `batchGetStorageSync`、`wx.getStorage` / `setStorage` Promise 风格、空购物车态。

## N7

1. `sample/baby`「宝贝成长记」（生活服务·母婴育儿）：首页（宝宝卡片、今日汇总、快捷记录）、喂养记录（母乳计时左右侧 / 奶粉毫升）、睡眠记录、生长曲线（对照 WHO 百分位）、
   疫苗计划、成长相册（时间轴 + 拍照）、育儿知识（按月龄、详情）、宝宝资料、我的。
   必须覆盖：`canvas type="2d"`（生长曲线，按 dpr 缩放）、`camera` + `wx.createCameraContext().takePhoto`（降级 chooseImage → 示例照片）、`wx.getRecorderManager` + InnerAudioContext 回放、
   `slider`（step 10、show-value）、`picker mode="date"` + `mode="time"`、`setInterval` 计时（onHide 保留时间戳）、`wx.batchSetStorageSync`、`wx.getStorageInfoSync`、
   `wx.vibrateShort`、`wx.showActionSheet`、`wx.setNavigationBarTitle`。
2. `sample/decor`「好宅装修」（生活服务·家装）：首页（风格、案例瀑布流）、案例详情（图集、户型、预算、设计师）、设计师列表 / 主页、预算计算器（分项预算饼图）、
   户型布置（户型图上拖动家具、缩放）、预约量房、装修日记 / 进度、我的。
   必须覆盖：`movable-area` + `movable-view`（`direction="all"`、`out-of-bounds`、`bindchange` 持久化 x/y、`scale` + scale-min/max）、
   Skyline 手势：`pan-gesture-handler` / `scale-gesture-handler` / `double-tap-gesture-handler` 至少两个（worklet 回调 `'worklet';`）、
   `wx.worklet.shared` + `applyAnimatedStyle`（不支持时退回 setData 并写进报告）、`canvas type="2d"` 饼图、slider + switch + radio-group 联动算价、`progress`、IntersectionObserver 渐显、`hover-class`。

## N8

1. `sample/legal`「律安心」（生活服务·法律咨询，标注「仅供参考，不构成法律意见」）：首页（咨询分类、热门问答、律师推荐）、律师列表（筛选）、律师主页、图文咨询下单 + 支付、
   咨询会话（文本、图片、律师回复模拟、结束后评价）、合同模板库、合同编辑（富文本）、法律计算器（工伤 / N+1 / 诉讼费）、我的。
   必须覆盖：`editor` + `EditorContext`（`select('#editor').context()`：insertText、format('bold')、setContents、getContents、undo/redo，存 storage）、
   聊天输入 `adjust-position="{{false}}"` + `wx.onKeyboardHeightChange` 上推、`cursor-spacing`、`confirm-type="send"` + `confirm-hold`、`scroll-into-view` 滚到最新、
   `wx.getClipboardData` / `setClipboardData`、`textarea show-confirm-bar="{{false}}"`、`wx.requestPayment`（fail 模拟收银台）、`picker` range-key、`radio-group`、`wx.showActionSheet`、`rich-text`、`wx.chooseImage`（fail 降级）。
2. `sample/office-oa`「云协同」（工具·企业 OA）：首页（待办数、日程、快捷入口）、审批中心（我发起 / 待我审批 / 已处理）、发起审批（请假 / 报销 / 用印动态表单）、审批详情（节点、同意 / 驳回 + 意见）、
   考勤打卡（定位范围、月统计）、会议室预订（时间轴网格选时段）、通讯录（A-Z 索引、搜索、部门）、员工详情、我的。
   必须覆盖：`wx.createWorker('workers/stats.js')`（app.json `"workers": "workers"`；不支持时主线程 fallback 并写进报告）、索引栏 `bindtouchstart/move/end` 读 `e.touches[0].clientY` + `scroll-into-view` + 大字母提示、
   `wx.getLocation`（fail 降级）、`wx.startWifi` + `wx.getConnectedWifi`（fail 忽略）、`wx.getSystemInfoSync` / `getWindowInfo` / `onWindowResize`、
   动态表单 `<block wx:for>` + `wx:if/elif`、`picker mode="date" fields="month"`、`wx.showModal({editable:true, placeholderText})`、navigateTo events 回传。

## N9

1. `sample/farm`「田园直供」（购物·农产品产地直发）：首页（产地故事、时令推荐、拼团、直播入口）、商品详情（产地视频、图集、SKU、拼团价 / 单买价）、拼团（开团 / 参团、倒计时、差 N 人）、
   直播间（live-player，不支持时封面 + 回放 video）、购物车、确认订单（地址、配送时间）、地址管理、订单列表 / 详情（物流、确认收货、售后）、我的。
   必须覆盖：`live-player`（src / mode="live" / autoplay / bindstatechange / binderror，`wx.createLivePlayerContext`，降级）、`video`（object-fit="cover"、show-center-play-btn）、
   page-container 或自绘半屏 SKU（互斥、无库存置灰）、多个倒计时共用一个定时器（onUnload 清理）、`wx.chooseAddress`（fail 自建地址簿）、
   swiper indicator-dots + indicator-active-color + easing-function、scroll-x 横滑、`wx.showToast({icon:'success'})`、`image mode` ≥4 种、`text` 的 `space` / `decode`。
2. `sample/fuel`「易加油」（汽车·加油站）：首页（附近油站：距离 / 价格排序、油号筛选、地图模式）、油站详情（价格、优惠、设施、导航）、加油下单（油枪号、自定义数字键盘、算升数与优惠）、
   支付结果、加油记录（月度统计）、油卡 / 优惠券、电子发票（抬头管理）、油价走势、我的。
   必须覆盖：`wx.getLocation`（fail 回退）+ haversine 距离排序、`map` + markers（label / callout 价格气泡）+ `wx.openLocation`（fail 降级）、
   自定义数字键盘（fixed 底部面板，小数点 / 两位小数 / 上限，hover-class）、`picker` 油号、`wx.chooseInvoiceTitle`（fail 自建抬头）、`wx.getNetworkType` + `onNetworkStatusChange`、
   `canvas type="2d"` 走势、`wx.vibrateShort`、`wx.requestPayment`（fail 模拟收银台）、`radio-group`、`input type="digit"`。

## N10

1. `sample/live-show`「现场」（娱乐·演出票务，虚构艺人）：首页（城市、分类、开售日历）、演出详情（海报、场次、票档、须知、开售倒计时 + 提醒）、
   选座（座位图拖动 + 缩放、点选 / 取消、最多 4 张、已售置灰、分区图例）、确认订单（观演人实名）、支付结果、电子票（二维码、出示调亮）、票夹、想看、我的。
   必须覆盖：`movable-area` + `movable-view`（scale + scale-min/max、bindscale、direction="all"，座位命中要准）、IntersectionObserver（tab 吸顶高亮）、
   `onPageScroll` + `wx.setNavigationBarColor`（节流）、swiper 3D 卡片（current 放大）、`wx.requestSubscribeMessage`（fail 本地标记）、`wx.setScreenBrightness` + `setKeepScreenOn`、
   canvas 票码、CSS `@keyframes` 呼吸按钮、`position: sticky` 吸顶、`wx.showLoading` 模拟排队、`wx.showModal`。
2. `sample/sports-venue`「约场」（体育·场馆预订）：首页（项目、附近场馆）、场馆详情、场地预订（7 天日期条 + 场地 × 时段网格多选，三态，连续时段合并计价，高峰价）、
   确认订单 + 支付结果、入场码、约球广场（发起 / 加入组局，人数进度）、会员卡（次卡 / 储值）、我的订单、我的。
   必须覆盖：`grid-view`（type="masonry"/"aligned"、cross-axis-count、main-axis-gap、cross-axis-gap；不支持时降级 flex 并写进报告）、
   `display:grid` 或二维 `scroll-view` 时段网格 + 冻结行列、scroll-x 日期条、`progress`、`wx.getLocation`（fail 回退）、`wx.makePhoneCall`（fail 复制）、
   `wx.requestPayment`（fail 模拟收银台）、`wx.showActionSheet`、`button open-type="share"` + `onShareAppMessage`、`wx.startPullDownRefresh` / onPullDownRefresh、`wx.pageScrollTo({selector})`。

## N11

1. `sample/meditate`「静心」（医疗健康·冥想助眠）：首页（按时段问候、推荐、连续打卡）、课程分类、课程详情（章节、解锁）、播放页（封面旋转、可拖进度、循环、定时关闭、
   白噪音混音：雨 / 海浪 / 篝火三路音量 slider 同时播）、呼吸练习（4-7-8 圆圈缩放 + 文案）、睡眠记录（周柱状图）、心情日记、我的（深色模式、统计）。
   必须覆盖：多个 InnerAudioContext 同播（loop、volume、seek、onTimeUpdate、onEnded、destroy；python 合成可无缝循环的音频，每段 ≤ 80KB）、`wx.getBackgroundAudioManager`、
   CSS `@keyframes` + `animation-play-state`、`this.animate` / `wx.createAnimation`、`slider`（block-size、activeColor、bindchanging）、`wx.setKeepScreenOn`、
   `wx.onThemeChange` + `"darkmode": true` + theme.json（或自管理深色主题，至少其一并说明）、`wx.createCanvasContext` 柱状图、`picker mode="time"`、`wx.onAppHide` / `onAppShow`、setTimeout 定时关闭。
2. `sample/laundry`「净衣到家」（生活服务·上门洗衣）：首页（服务分类、价目入口、banner）、价目表（分类 tab + 步进器 + 合计栏）、下单（地址、picker-view 日期 + 时段、备注、优惠券）、支付结果、
   订单列表（待取件 / 清洗中 / 待送回 / 已完成）、订单详情（状态自动推进、衣物清单、质检照片）、评价、优惠券、我的。
   必须覆盖：`picker-view` 两列联动（今天过去的时段不可选）、`wx.connectSocket` / SocketTask（onOpen/onMessage/onError/onClose，连不上降级本地 setInterval 推送，无未捕获错误）、
   `wx.scanCode`（fail 手动输入）、半星评分组件（properties + triggerEvent）、`textarea`、`wx.showModal`、`wx.showActionSheet`、`checkbox`、`wx.redirectTo`、`wx.navigateBack`、空态 / 加载态、`position: sticky` 吸顶 tab。

## N12

1. `sample/flight`「云翼机票」（出行·机票，虚构航司、真实三字码）：首页（单程 / 往返、城市互换动画、日期、舱位）、城市选择（热门 + 字母分组 + 索引 + 拼音 / 三字码搜索）、
   日期选择（月历、往返区间、每日低价）、航班列表（排序、筛选、7 天低价条）、航班详情 + 舱位（退改规则）、乘机人管理（证件校验）、填写订单（乘机人、联系人、保险）、支付结果、订单详情（行程单、退改签）、我的。
   必须覆盖：navigateTo `events` + `EventChannel`（城市 / 日期回传）、`canvas type="2d"` 低价走势、`scroll-into-view` 字母索引、`wx.showActionSheet`、`page-container` 筛选面板、
   `picker` 证件类型、`input type="idcard"` + maxlength、`wx.vibrateShort`、`this.animate`、搜索历史 storage、`wx.addPhoneCalendar`（fail 降级）、`wx.setClipboardData`、`wx.requestPayment`（fail 模拟收银台）。
2. `sample/vocab`「词力」（教育·背单词）：首页（进度环、打卡天数）、选词书、学习页（卡片：单词 / 音标 / 释义 / 例句；认识 / 模糊 / 不认识；左右滑）、复习（1/2/4/7/15 天间隔）、
   测验（四选一、拼写、错题本）、打卡日历、学习统计（柱状图）、生词本、我的（每日计划量）。
   必须覆盖：`horizontal-drag-gesture-handler` 或 `pan-gesture-handler` 卡片滑动（worklet；不支持时 touch 事件实现并写进报告）、`swiper bindanimationfinish`、InnerAudioContext（合成 wav ≤ 30KB）、
   `wx.vibrateShort`、`progress` + 自绘进度环（conic-gradient 或 canvas）、`wx.getRandomValues`、`wx.requestSubscribeMessage`（fail 降级）、`slider`、`picker mode="time"`、
   自定义 tabBar 角标、`wx.showModal`、`wx.nextTick`。

---

# 小游戏（每款都要好玩、有开始 / 结束 / 重来、最高分存档；sample-game/<id>/）

## G1
- `play-tetris`「俄罗斯方块」（竖屏）：7 种方块、SRS 旋转 + 踢墙、硬降 / 软降、暂存（hold）、下一块预览、消行动画、等级提速、幽灵块。
  覆盖：触摸（点击旋转、左右滑移动、下滑硬降、长按软降）+ 键盘（`wx.onKeyDown`：方向键 / 空格 / C）、`wx.vibrateShort`（消行）、`wx.setPreferredFramesPerSecond`、存档 + 最高分。
- `play-gomoku`「五子棋」（竖屏）：15×15，人机对战（AI 用评分 + α-β 剪枝，3 档难度）、悔棋、胜负判定 + 连线高亮。
  覆盖：`wx.createWorker`（AI 搜索放 Worker，game.json 配 `"workers"`；不支持时主线程分帧计算并写进报告）、2D canvas（`getContext('2d')`）、`wx.showModal`（认输确认）。

## G2
- `play-match3`「萌果消消乐」（竖屏）：8×8 交换消除、L / T 型特效块、连锁、步数关卡（≥10 关）、目标（收集 N 个某色）、星级评价。
  覆盖：补间动画系统（下落、交换回弹、消除缩放）、WebAudio 预合成音效（连击音高递增）、`wx.getOpenDataContext` + 开放数据域排行榜（参考 `sample-game/g-opendata`，不支持时本地排行并写进报告）。
- `play-memory`「记忆翻牌」（竖屏）：4×4 / 4×5 / 6×6 三档，翻牌动画（scaleX 模拟 3D 翻转）、计时、步数、连对加分。
  覆盖：`wx.loadSubpackage`（卡面主题放分包，game.json `subpackages`，参考 `sample-game/g-subpkg`）、`wx.createInnerAudioContext`（背景音乐循环，onHide 暂停）。

## G3
- `play-shooter`「星际战机」（竖屏）：拖动战机、自动射击、3 种敌机 + Boss、道具（双发 / 护盾 / 炸弹）、粒子爆炸、无尽波次。
  覆盖：WebGL 批量绘制（拷 play-snake 的 gl2d.js，几百个对象 60fps）、对象池（不在帧里分配）、InnerAudioContext BGM、WebAudio 音效、`wx.onHide` 暂停 / `onShow` 恢复。
- `play-fruit`「切水果」（竖屏）：抛物线水果、滑动刀光轨迹、切开两半飞散 + 果汁粒子、炸弹、连切奖励、经典 / 限时两模式。
  覆盖：`wx.onTouchMove` 高频轨迹（线段与圆相交判定）、WebAudio、`wx.vibrateShort`（切到炸弹 vibrateLong）。

## G4
- `play-minesweeper`「扫雷」（竖屏）：初级 / 中级 / 高级、首次点击必不是雷、点开空白自动展开、长按插旗、双击已开数字快速开周围、计时、可缩放平移大棋盘。
  覆盖：2D canvas、长按判定（自己算时长）、双指缩放（多点触控 `touches`）、`wx.getFileSystemManager` 存档（`USER_DATA_PATH` 下 json）。
- `play-sudoku`「数独」（竖屏）：生成唯一解题目（挖洞 + 求解器校验）、4 档难度、候选数笔记模式、冲突高亮、提示 3 次、撤销、计时。
  覆盖：键盘输入（`wx.onKeyDown` 数字键）、`wx.showModal` / `wx.showToast`、storage 存进度。

## G5
- `play-racing`「极速狂飙」（横屏 landscape）：伪 3D 公路（分段投影）、换道躲车、加速带、金币、油量。
  覆盖：`wx.startAccelerometer` + `wx.onAccelerometerChange` 倾斜转向（不可用时屏幕左右半边按压转向）、`deviceOrientation: landscape`、`wx.vibrateLong`（撞车）、safeArea。
- `play-hockey`「桌上冰球」（横屏 landscape）：双人同屏（左右各一只手同时拖球拍）+ 人机模式、物理碰撞（圆与圆、圆与墙）、7 分制。
  覆盖：多点触控（按 `identifier` 跟踪两根手指）、固定步长物理、WebAudio 碰撞音。

## G6
- `play-piano`「钢琴块」（竖屏）：四轨下落黑块、点按发音（WebAudio 合成钢琴音色：多谐波 + 包络）、内置 ≥5 首曲谱（公版曲目：小星星、欢乐颂、致爱丽丝片段…）、越来越快、漏按 / 点错结束。
  覆盖：WebAudio（OscillatorNode / GainNode 包络 或 预合成 AudioBuffer、`decodeAudioData`）、精确时序（按音频时钟 `currentTime` 对齐）。
- `play-bubble`「泡泡龙」（竖屏）：六边形网格、瞄准线（含墙面反弹预测）、三连消除 + 悬空掉落、下压行、关卡。
  覆盖：`wx.createRewardedVideoAd`（看广告复活，create / onError 失败时直接给复活或隐藏按钮）、`wx.shareAppMessage` + `wx.onShareAppMessage`、`wx.showShareMenu`。

## G7
- `play-3d-maze`「3D 迷宫」（竖屏）：three.js 随机生成迷宫（DFS），第一人称或俯视追尾相机、虚拟摇杆、小地图、计时、宝石收集。
  覆盖：three.js（参考 `sample-game/play-3d-stack` 的 libs + adapter）、`wx.startDeviceMotionListening` / `onDeviceMotionChange` 或陀螺仪倾斜控制（不可用时摇杆）、性能（合并几何体，drawcall 少）。
- `play-tower`「塔防守卫」（竖屏）：固定路线、4 种塔（箭 / 炮 / 冰 / 电，各 3 级升级）、10 波敌人（含飞行 / 护甲 / Boss）、金币经济、倍速。
  覆盖：寻路 / 索敌（四叉树或网格分桶）、2D canvas 或 gl2d、`WXWebAssembly`（参考 `sample-game/g-wasm`，用小 wasm 模块做伤害 / 距离计算；做不了就写进报告）、存档关卡进度。

---

# 已有小程序「把功能做完」（E 组）

对每个分到的包：先 `python3 tools/sample-smoke.py <id>` 拿基线；逐页用无头截图看界面上有哪些入口，再读那一页的源码找：
空的 / 缺的 `bindtap`、「开发中 / 敬请期待 / 即将上线」提示、点了没反应的入口、只有列表没有详情、只有下单没有订单、没有空态 / 校验、刷新后状态丢失。
把它们**都做成真实功能**（需要的话新增页面，每个包一般新增 1~4 页），并补上同行业该有的关键能力。
**保持现有页面的路由、元素 id / class、data 字段名不变**（e2e 测试靠它们找元素）；改动后 `grep -l '"<id>"' tests/*.rs` 命中的测试必须全过。
联网示例（movie-online / music-online / photo-wall / mini-hub）只能用服务器已有的 `/api/demo/...` 接口（看各自 `utils/api.js`），不改 `tools/mini-store/`。
报告里逐包列：发现的未完成点 → 怎么补的、新增页面、验证结果（smoke + 走查 + e2e）。

> **2026-11-02 20:52 用户决定**：下面 ~~划掉~~ 的 14 个冻结，不改（名单见 `.kiro/steering/00-scan-policy.md`「已固化」一节）。
> 分组里只做没划掉的；E8 整组取消。

- E1：calendar、kanban、notes、focus-todo、survey
- E2：~~music~~、~~music-online~~、~~movie-online~~、cinema、~~reader~~
- E3：train、ride、parking、ev-charge、hotel、travel
- E4：food-delivery、restaurant、~~recipe~~、fresh-mart、flower
- E5：hospital、fitness、pet、house、jobs
- E6：~~stocks~~、ledger、life-pay、express、~~weather~~
- E7：course、~~quiz-game~~、~~game-2048~~、~~marathon~~、~~toolbox~~
- E8：~~smart-home~~、~~mini-hub~~、~~photo-wall~~（整组取消）

不在 E 组里的：sample-app / news-app（逐像素基线）、tea-app（uni-app 编译产物）、real-sample（官方 demo）、touch-probe（探针）、
coffee（另一个会话正在改）、bank / chat / forum（顿挫 / 键盘测试的固定输入）、zz-* / _archive。
