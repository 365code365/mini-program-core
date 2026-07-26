// 头条新闻 · 文章详情（返回 / 字号设置生效 / 评论提交 / 收藏关注持久化）
var ARTICLES = {
  default: {
    id: 1001,
    title: '数字经济规模再创新高，实体产业融合成为主线',
    author: '新华观察',
    avatar: '/assets/avatar1.jpg',
    time: '2 小时前',
    readCount: '12.8万',
    cover: '/assets/banner.jpg',
    tags: ['数字经济', '产业升级', '独家'],
    body: '记者从相关部门获悉，上半年数字经济核心产业增加值同比增长明显，占国内生产总值比重进一步提升。\n\n从结构上看，数字技术与实体产业的融合成为增长主线：智能制造改造项目数量同比增加，工业互联网平台连接设备规模持续扩大，一批中小企业通过轻量化改造实现了产能与良率的同步提升。\n\n业内人士分析认为，下一阶段的重点将从"建平台"转向"用平台"，数据要素的流通与定价机制、算力资源的跨区域调度，都会成为政策关注的方向。\n\n多位受访企业负责人表示，融合发展带来的效率提升已在订单交付周期上体现，但复合型人才短缺仍是主要制约因素。'
  }
};

var TITLES = {
  101: { title: '国产大飞机再获百架订单 交付节奏明显加快', author: '经济观察' },
  102: { title: '多地推出住房消费新政 首套房贷利率下调', author: '财经快讯' },
  103: { title: '人工智能进课堂：一线教师的真实体验', author: '教育前沿' },
  104: { title: '暑运客流创新高 铁路部门加开夜间高铁', author: '出行观察' },
  201: { title: '新能源车渗透率突破关键节点 供应链加速重构', author: '汽车纵横' },
  301: { title: '联赛焦点战：主队补时绝杀 现场万人沸腾', author: '体育直播间' },
  401: { title: '暑期档票房破百亿 现实题材成最大赢家', author: '娱乐观察' },
  1002: { title: '城市更新样本：老街区如何焕发新活力', author: '城市观察' }
};

Page({
  data: {
    articleId: 1001,
    liked: false,
    collected: false,
    followed: false,
    likeCount: 1286,
    nightMode: false,
    fontLevel: 3,
    bodySize: 17,
    titleSize: 24,
    readPercent: 72,
    fontPanel: false,
    composing: false,
    draft: '',
    article: ARTICLES.default,
    related: [
      { id: 3001, title: '工业互联网平台连接设备数突破新高', source: '产业观察', comments: 132 },
      { id: 3002, title: '数据要素市场化配置改革试点扩围', source: '政策解读', comments: 88 },
      { id: 3003, title: '算力调度网络建设进入快车道', source: '科技前沿', comments: 205 }
    ],
    baseComments: [
      { id: 1, user: 'industry_watcher', avatar: '/assets/avatar1.jpg', text: '融合是关键，单纯上系统解决不了产线的实际问题。', time: '1 小时前', likes: 216 },
      { id: 2, user: '制造业老张', avatar: '/assets/avatar1.jpg', text: '我们厂去年做了改造，良率确实提升了，但人才招不到。', time: '2 小时前', likes: 154 },
      { id: 3, user: 'data_fan', avatar: '/assets/avatar1.jpg', text: '希望数据定价机制能尽快明确，现在交易顾虑比较多。', time: '3 小时前', likes: 97 }
    ],
    comments: []
  },

  onLoad: function (query) {
    var app = getApp();
    var id = Number(query && query.id ? query.id : 1001);
    var article = JSON.parse(JSON.stringify(ARTICLES.default));
    article.id = id;
    if (TITLES[id]) {
      article.title = TITLES[id].title;
      article.author = TITLES[id].author;
    }
    var settings = app.getSettings();
    this.setData({
      articleId: id,
      article: article,
      nightMode: !!settings.nightMode,
      fontLevel: settings.fontLevel || 3,
      bodySize: app.fontSizeOf(settings.fontLevel || 3),
      titleSize: app.fontSizeOf(settings.fontLevel || 3) + 7,
      collected: app.isFavorite(id),
      followed: app.isFollowed(article.author)
    });
    this.refreshComments();
    app.pushHistory(article);
    console.log('📰 文章详情加载，id=' + id);
  },

  refreshComments: function () {
    var mine = getApp().getComments(this.data.articleId);
    this.setData({ comments: mine.concat(this.data.baseComments) });
  },

  // ==================== 交互 ====================

  onLike: function () {
    var liked = !this.data.liked;
    this.setData({ liked: liked, likeCount: this.data.likeCount + (liked ? 1 : -1) });
  },

  onCollect: function () {
    var on = getApp().toggleFavorite(this.data.article);
    this.setData({ collected: on });
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' });
  },

  onToggleFollow: function () {
    var on = getApp().toggleFollow(this.data.article.author);
    this.setData({ followed: on });
    wx.showToast({ title: on ? '已关注 ' + this.data.article.author : '已取消关注', icon: 'none' });
  },

  onShare: function () {
    wx.showModal({
      title: '分享',
      content: '把《' + this.data.article.title + '》分享给好友？',
      success: function (res) {
        if (res.confirm) { wx.showToast({ title: '已分享', icon: 'success' }); }
      }
    });
  },

  // ==================== 评论 ====================

  onWriteComment: function () {
    this.setData({ composing: true });
  },

  onDraftInput: function (e) {
    this.setData({ draft: e.detail.value });
  },

  onSubmitComment: function () {
    var text = (this.data.draft || '').trim();
    if (!text) {
      wx.showToast({ title: '说点什么吧', icon: 'none' });
      return;
    }
    getApp().addComment(this.data.articleId, text);
    this.setData({ draft: '', composing: false });
    this.refreshComments();
    wx.showToast({ title: '评论已发布', icon: 'success' });
  },

  onLikeComment: function (e) {
    var id = Number(e.currentTarget.dataset.id);
    var list = this.data.comments.map(function (c) {
      if (c.id === id) { c.likes = c.likes + 1; }
      return c;
    });
    this.setData({ comments: list });
  },

  // ==================== 字号 ====================

  onOpenFontPanel: function () { this.setData({ fontPanel: true }); },
  onCloseFontPanel: function () { this.setData({ fontPanel: false }); },

  onFontChange: function (e) {
    var app = getApp();
    var level = Number(e.detail.value) || 3;
    app.saveSettings({ fontLevel: level });
    this.setData({
      fontLevel: level,
      bodySize: app.fontSizeOf(level),
      titleSize: app.fontSizeOf(level) + 7
    });
  },

  onOpenRelated: function (e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  },

  onBack: function () {
    wx.navigateBack();
  }
});
