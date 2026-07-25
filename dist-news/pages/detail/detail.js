// 头条新闻 · 文章详情
Page({
  data: {
    liked: false,
    collected: false,
    followed: false,
    likeCount: 1286,
    article: {
      title: '数字经济规模再创新高，实体产业融合成为主线',
      author: '新华观察',
      avatar: '/assets/avatar1.jpg',
      time: '2 小时前',
      readCount: '12.8万',
      cover: '/assets/banner.jpg',
      tags: ['数字经济', '产业升级', '独家'],
      body: '记者从相关部门获悉，上半年数字经济核心产业增加值同比增长明显，占国内生产总值比重进一步提升。\n\n从结构上看，数字技术与实体产业的融合成为增长主线：智能制造改造项目数量同比增加，工业互联网平台连接设备规模持续扩大，一批中小企业通过轻量化改造实现了产能与良率的同步提升。\n\n业内人士分析认为，下一阶段的重点将从"建平台"转向"用平台"，数据要素的流通与定价机制、算力资源的跨区域调度，都会成为政策关注的方向。\n\n多位受访企业负责人表示，融合发展带来的效率提升已在订单交付周期上体现，但复合型人才短缺仍是主要制约因素。'
    },
    related: [
      { id: 3001, title: '工业互联网平台连接设备数突破新高', source: '产业观察', comments: 132 },
      { id: 3002, title: '数据要素市场化配置改革试点扩围', source: '政策解读', comments: 88 },
      { id: 3003, title: '算力调度网络建设进入快车道', source: '科技前沿', comments: 205 }
    ],
    comments: [
      { id: 1, user: 'industry_watcher', avatar: '/assets/avatar1.jpg', text: '融合是关键，单纯上系统解决不了产线的实际问题。', time: '1 小时前', likes: 216 },
      { id: 2, user: '制造业老张', avatar: '/assets/avatar1.jpg', text: '我们厂去年做了改造，良率确实提升了，但人才招不到。', time: '2 小时前', likes: 154 },
      { id: 3, user: 'data_fan', avatar: '/assets/avatar1.jpg', text: '希望数据定价机制能尽快明确，现在交易顾虑比较多。', time: '3 小时前', likes: 97 }
    ]
  },

  onLoad: function (query) {
    console.log('📰 文章详情加载，id=' + (query && query.id ? query.id : 'default'));
  },

  onLike: function () {
    var liked = !this.data.liked;
    this.setData({
      liked: liked,
      likeCount: this.data.likeCount + (liked ? 1 : -1)
    });
  },

  onCollect: function () {
    this.setData({ collected: !this.data.collected });
    wx.showToast({ title: this.data.collected ? '已收藏' : '已取消收藏', icon: 'none' });
  },

  onToggleFollow: function () {
    this.setData({ followed: !this.data.followed });
  },

  onShare: function () {
    wx.showToast({ title: '分享面板', icon: 'none' });
  },

  onWriteComment: function () {
    wx.showToast({ title: '评论输入框', icon: 'none' });
  },

  onLikeComment: function (e) {
    var id = Number(e.currentTarget.dataset.id);
    var list = this.data.comments.map(function (c) {
      if (c.id === id) { c.likes = c.likes + 1; }
      return c;
    });
    this.setData({ comments: list });
  },

  onOpenRelated: function (e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  }
});
