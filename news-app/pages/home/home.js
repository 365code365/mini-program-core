// 头条新闻 · 首页信息流
var ALL_FEEDS = {
  1: [
    { id: 101, title: '国产大飞机再获百架订单 交付节奏明显加快', source: '经济观察', comments: 328, thumb: '/assets/p_phone.jpg', hot: true },
    { id: 102, title: '多地推出住房消费新政 首套房贷利率下调', source: '财经快讯', comments: 156, thumb: '/assets/p_backpack.jpg', hot: false },
    { id: 103, title: '人工智能进课堂：一线教师的真实体验', source: '教育前沿', comments: 92, thumb: '/assets/p_watch.jpg', hot: false },
    { id: 104, title: '暑运客流创新高 铁路部门加开夜间高铁', source: '出行观察', comments: 241, thumb: '/assets/p_sneakers.jpg', hot: true }
  ],
  2: [
    { id: 201, title: '新能源车渗透率突破关键节点 供应链加速重构', source: '汽车纵横', comments: 415, thumb: '/assets/p_phone2.jpg', hot: true },
    { id: 202, title: '芯片产业链回暖 封测厂订单排至年底', source: '半导体观察', comments: 187, thumb: '/assets/p_earbuds.jpg', hot: false },
    { id: 203, title: '折叠屏手机价格下探 千元档竞争白热化', source: '数码前线', comments: 264, thumb: '/assets/p_phone.jpg', hot: false }
  ],
  3: [
    { id: 301, title: '联赛焦点战：主队补时绝杀 现场万人沸腾', source: '体育直播间', comments: 892, thumb: '/assets/p_sneakers.jpg', hot: true },
    { id: 302, title: '游泳世锦赛综述：中国队再添两金', source: '竞技体育', comments: 351, thumb: '/assets/p_thermos.jpg', hot: false }
  ],
  4: [
    { id: 401, title: '暑期档票房破百亿 现实题材成最大赢家', source: '娱乐观察', comments: 673, thumb: '/assets/p_cake.jpg', hot: true },
    { id: 402, title: '综艺节目转型：从流量到内容的回归', source: '视听前沿', comments: 128, thumb: '/assets/p_lipstick.jpg', hot: false }
  ]
};

var CHANNEL_TOP = {
  1: '国务院发布新一轮稳增长政策，涉及六大领域',
  2: '工信部：加快推进新型工业化重点项目落地',
  3: '奥运会资格赛收官，中国代表团名单公布',
  4: '国家电影局发布暑期档观影数据报告'
};

Page({
  data: {
    searchHint: '搜索热点新闻',
    currentChannel: 1,
    channels: [
      { id: 1, name: '推荐' },
      { id: 2, name: '科技' },
      { id: 3, name: '体育' },
      { id: 4, name: '娱乐' },
      { id: 5, name: '财经' },
      { id: 6, name: '军事' },
      { id: 7, name: '国际' }
    ],
    topNews: { title: '国务院发布新一轮稳增长政策，涉及六大领域' },
    bigCard: {
      id: 1001,
      title: '数字经济规模再创新高，实体产业融合成为主线',
      cover: '/assets/banner.jpg',
      tag: '独家',
      source: '新华观察',
      comments: 1286
    },
    tripleCard: {
      id: 1002,
      title: '城市更新样本：老街区如何焕发新活力',
      images: ['/assets/p_coffee.jpg', '/assets/p_cake.jpg', '/assets/p_lipstick.jpg'],
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
    list: ALL_FEEDS[1]
  },

  onLoad: function () {
    console.log('📰 首页加载，当前频道：推荐');
  },

  onSelectChannel: function (e) {
    var id = Number(e.currentTarget.dataset.id);
    var list = ALL_FEEDS[id] || ALL_FEEDS[1];
    var top = CHANNEL_TOP[id] || CHANNEL_TOP[1];
    this.setData({
      currentChannel: id,
      list: list,
      topNews: { title: top }
    });
  },

  onOpenArticle: function (e) {
    var id = e.currentTarget.dataset.id;
    wx.navigateTo({ url: '/pages/detail/detail?id=' + id });
  },

  onSearchTap: function () {
    wx.showToast({ title: '搜索功能演示', icon: 'none' });
  },

  onMoreHot: function () {
    wx.showToast({ title: '查看完整热榜', icon: 'none' });
  }
});
