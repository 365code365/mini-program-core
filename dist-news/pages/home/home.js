// 头条新闻 · 首页信息流（轮播 / 频道 / 下拉刷新 / 触底加载 / 收藏）
var ALL_FEEDS = {
  1: [
    { id: 101, title: '国产大飞机再获百架订单 交付节奏明显加快', source: '经济观察', comments: 328, thumb: '/assets/n_airplane.jpg', hot: true },
    { id: 102, title: '多地推出住房消费新政 首套房贷利率下调', source: '财经快讯', comments: 156, thumb: '/assets/n_housing.jpg', hot: false },
    { id: 103, title: '人工智能进课堂：一线教师的真实体验', source: '教育前沿', comments: 92, thumb: '/assets/n_ai_class.jpg', hot: false },
    { id: 104, title: '暑运客流创新高 铁路部门加开夜间高铁', source: '出行观察', comments: 241, thumb: '/assets/n_rail.jpg', hot: true }
  ],
  2: [
    { id: 201, title: '新能源车渗透率突破关键节点 供应链加速重构', source: '汽车纵横', comments: 415, thumb: '/assets/n_ev.jpg', hot: true },
    { id: 202, title: '芯片产业链回暖 封测厂订单排至年底', source: '半导体观察', comments: 187, thumb: '/assets/n_chip.jpg', hot: false },
    { id: 203, title: '折叠屏手机价格下探 千元档竞争白热化', source: '数码前线', comments: 264, thumb: '/assets/n_foldphone.jpg', hot: false }
  ],
  3: [
    { id: 301, title: '联赛焦点战：主队补时绝杀 现场万人沸腾', source: '体育直播间', comments: 892, thumb: '/assets/n_football.jpg', hot: true },
    { id: 302, title: '游泳世锦赛综述：中国队再添两金', source: '竞技体育', comments: 351, thumb: '/assets/n_swim.jpg', hot: false }
  ],
  4: [
    { id: 401, title: '暑期档票房破百亿 现实题材成最大赢家', source: '娱乐观察', comments: 673, thumb: '/assets/n_cinema.jpg', hot: true },
    { id: 402, title: '综艺节目转型：从流量到内容的回归', source: '视听前沿', comments: 128, thumb: '/assets/n_stage.jpg', hot: false }
  ],
  5: [
    { id: 501, title: '两市成交额回升 北向资金连续三日净买入', source: '市场观察', comments: 452, thumb: '/assets/n_stock.jpg', hot: true },
    { id: 502, title: '公募基金二季报出炉 重仓方向出现调整', source: '基金周报', comments: 231, thumb: '/assets/n_fund.jpg', hot: false }
  ],
  6: [
    { id: 601, title: '新型运输机完成高原起降科目训练', source: '军事观察', comments: 738, thumb: '/assets/n_military_air.jpg', hot: true },
    { id: 602, title: '海军舰艇编队完成远海联合演练', source: '国防在线', comments: 396, thumb: '/assets/n_navy.jpg', hot: false }
  ],
  7: [
    { id: 701, title: '多国联合发布气候行动倡议 明确阶段目标', source: '国际观察', comments: 517, thumb: '/assets/n_globe.jpg', hot: true },
    { id: 702, title: '全球供应链论坛开幕 聚焦区域协同', source: '环球财经', comments: 184, thumb: '/assets/n_port.jpg', hot: false }
  ]
};

// 每个频道的信息流一次给全：原文 + 两轮「更多」（从前是滚到底再追加）
function feedOf(id) {
  var base = ALL_FEEDS[id] || ALL_FEEDS[1];
  var out = base.slice();
  for (var round = 1; round <= 2; round++) {
    base.forEach(function (item, i) {
      out.push({ id: item.id * 10 + round * 100 + i, title: item.title, source: item.source,
                 comments: item.comments, thumb: item.thumb, hot: false });
    });
  }
  return out;
}

var CHANNEL_TOP = {
  1: { id: 9001, title: '国务院发布新一轮稳增长政策，涉及六大领域' },
  2: { id: 9002, title: '工信部：加快推进新型工业化重点项目落地' },
  3: { id: 9003, title: '奥运会资格赛收官，中国代表团名单公布' },
  4: { id: 9004, title: '国家电影局发布暑期档观影数据报告' },
  5: { id: 9005, title: '央行发布二季度金融统计数据报告' },
  6: { id: 9006, title: '国防部就联合演训答记者问' },
  7: { id: 9007, title: '联合国大会通过多边合作决议' }
};

Page({
  data: {
    searchHint: '搜索热点新闻',
    currentChannel: 1,
    channels: [
      { id: 1, name: '推荐' }, { id: 2, name: '科技' }, { id: 3, name: '体育' },
      { id: 4, name: '娱乐' }, { id: 5, name: '财经' }, { id: 6, name: '军事' }, { id: 7, name: '国际' }
    ],
    focusList: [
      { id: 201, tag: '独家', title: '新能源车渗透率突破关键节点 供应链加速重构', cover: '/assets/n_ev.jpg' },
      { id: 101, tag: '要闻', title: '国产大飞机再获百架订单 交付节奏明显加快', cover: '/assets/n_airplane.jpg' },
      { id: 301, tag: '体育', title: '联赛焦点战：主队补时绝杀 现场万人沸腾', cover: '/assets/n_football.jpg' }
    ],
    topNews: CHANNEL_TOP[1],
    bigCard: {
      id: 1001,
      title: '数字经济规模再创新高，实体产业融合成为主线',
      cover: '/assets/n_economy.jpg',
      tag: '独家',
      source: '新华观察',
      comments: 1286
    },
    tripleCard: {
      id: 1002,
      title: '城市更新样本：老街区如何焕发新活力',
      images: ['/assets/n_street1.jpg', '/assets/n_street2.jpg', '/assets/n_street3.jpg'],
      source: '城市观察',
      comments: 214
    },
    hotList: [
      { id: 2001, title: '多部门联合发布消费提振举措', heat: 128 },
      { id: 2002, title: '台风路径调整 沿海地区加强防御', heat: 96 },
      { id: 2003, title: '国产芯片良率提升引发关注', heat: 87 },
      { id: 2004, title: '暑期研学市场火爆背后的隐忧', heat: 65 },
      { id: 2005, title: '新职业目录发布 新增19个工种', heat: 43 }
    ],
    list: feedOf(1),
    favMap: {},
    refreshing: false,
    refreshedTip: '',

    // 自定义顶栏（app.navMetrics）：状态栏、导航条高、给胶囊留的右边距；headerHeight = 整块固定顶栏的高
    nav: { statusBar: 20, navHeight: 44, capsuleSpace: 96 },
    headerHeight: 106,
    channelAnchor: '',
    slideClass: ''
  },

  onLoad: function () {
    console.log('📰 首页加载，当前频道：推荐');
    var nav = getApp().navMetrics();
    // 频道栏 84rpx：按屏宽换成 px（顶栏是 fixed，下面垫的空白要和它一样高）
    var channelH = Math.round(84 * nav.windowWidth / 750);
    this.setData({ nav: nav, headerHeight: nav.statusBar + nav.navHeight + channelH });
    this.syncFav();
  },

  onShow: function () {
    this.syncFav();
  },

  syncFav: function () {
    var map = {};
    getApp().getFavorites().forEach(function (a) { map['a' + a.id] = true; });
    this.setData({ favMap: map });
  },

  onSelectChannel: function (e) {
    this.switchChannel(Number(e.currentTarget.dataset.id), '');
  },

  /// 切频道：换内容、频道栏把它滚到中间；dir 是左右滑进来的方向（'left' / 'right'），点选为空
  switchChannel: function (id, dir) {
    if (id === this.data.currentChannel) { return; }
    this.setData({
      currentChannel: id,
      channelAnchor: 'ch' + id,
      list: feedOf(id),
      topNews: CHANNEL_TOP[id] || CHANNEL_TOP[1],

      slideClass: dir ? 'slide-in-' + dir : ''
    });
    // 动画放完把类去掉：同一方向再滑一次时类名不变，不去掉的话动画不会重放
    if (dir) {
      var self = this;
      clearTimeout(this.slideTimer);
      this.slideTimer = setTimeout(function () { self.setData({ slideClass: '' }); }, 260);
    }
    // 往下翻过了才回到信息流开头（顶栏下面），没翻的时候不动
    if (this.scrollTop > 0) { wx.pageScrollTo({ scrollTop: 0, duration: 0 }); }
    console.log('📑 切换频道：' + id + (dir ? '（滑动）' : ''));
  },

  onPageScroll: function (e) {
    this.scrollTop = e.scrollTop;
  },

  // 信息流上左右滑切频道：横向位移够大、明显比纵向大，才算（竖着滑是翻页面）
  onFeedTouchStart: function (e) {
    var t = e.touches && e.touches[0];
    this.swipeFrom = t ? { x: t.clientX, y: t.clientY, at: Date.now() } : null;
  },

  onFeedTouchEnd: function (e) {
    var from = this.swipeFrom;
    var t = e.changedTouches && e.changedTouches[0];
    this.swipeFrom = null;
    if (!from || !t) { return; }
    var dx = t.clientX - from.x;
    var dy = t.clientY - from.y;
    if (Math.abs(dx) < 60 || Math.abs(dx) < Math.abs(dy) * 1.8 || Date.now() - from.at > 800) { return; }
    var ids = this.data.channels.map(function (c) { return c.id; });
    var i = ids.indexOf(this.data.currentChannel) + (dx < 0 ? 1 : -1);
    if (i < 0 || i >= ids.length) { return; }
    this.switchChannel(ids[i], dx < 0 ? 'left' : 'right');
  },

  onOpenArticle: function (e) {
    var id = e.currentTarget.dataset.id;
    wx.navigateTo({ url: '/pages/detail/detail?id=' + id });
  },

  onToggleFav: function (e) {
    var ds = e.currentTarget.dataset;
    var on = getApp().toggleFavorite({ id: Number(ds.id), title: ds.title, source: '头条' });
    this.syncFav();
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' });
  },

  onSearchTap: function () {
    wx.navigateTo({ url: '/pages/search/search' });
  },

  onMoreHot: function () {
    wx.navigateTo({ url: '/pages/hot/hot' });
  },

  onPullDownRefresh: function () {
    var self = this;
    this.setData({ refreshing: true, refreshedTip: '' });
    setTimeout(function () {
      // 打乱当前频道顺序模拟拉到新内容
      var list = self.data.list.slice();
      list.unshift(list.pop());
      self.setData({
        list: list,
        refreshing: false,
        refreshedTip: '已更新 ' + list.length + ' 条内容'
      });
      wx.stopPullDownRefresh();
      setTimeout(function () { self.setData({ refreshedTip: '' }); }, 1500);
    }, 700);
  },

  // 不在滚动中追加：甩到底时追加列表要整页重排 + 整条带重画，真机一次 57~77ms（tools/android-jank.sh
  // news-fastslow 量的，屏上就是「滑着滑着顿一下」）。信息流在切频道 / 首屏时一次给全（feedOf），这里只是兜底提示
  onReachBottom: function () {}
});
