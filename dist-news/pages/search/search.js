// 头条新闻 · 搜索（历史 / 热搜榜 / 结果排序）
var LIBRARY = [
  { id: 1001, kind: 'news', title: '数字经济规模再创新高，实体产业融合成为主线', source: '新华观察', comments: 1286, heat: 210, thumb: '/assets/banner.jpg' },
  { id: 101, kind: 'news', title: '国产大飞机再获百架订单 交付节奏明显加快', source: '经济观察', comments: 328, heat: 128, thumb: '/assets/p_phone.jpg' },
  { id: 102, kind: 'news', title: '多地推出住房消费新政 首套房贷利率下调', source: '财经快讯', comments: 156, heat: 96, thumb: '/assets/p_backpack.jpg' },
  { id: 103, kind: 'news', title: '人工智能进课堂：一线教师的真实体验', source: '教育前沿', comments: 92, heat: 74, thumb: '/assets/p_watch.jpg' },
  { id: 201, kind: 'news', title: '新能源车渗透率突破关键节点 供应链加速重构', source: '汽车纵横', comments: 415, heat: 165, thumb: '/assets/p_phone2.jpg' },
  { id: 301, kind: 'news', title: '联赛焦点战：主队补时绝杀 现场万人沸腾', source: '体育直播间', comments: 892, heat: 188, thumb: '/assets/p_sneakers.jpg' },
  { id: 501, kind: 'video', title: '实拍：国产大飞机完成高原试飞全过程', source: '航空观察', comments: 1263, heat: 143, thumb: '/assets/p_phone.jpg' },
  { id: 502, kind: 'video', title: '三分钟看懂新能源车电池技术路线之争', source: '汽车实验室', comments: 486, heat: 118, thumb: '/assets/p_watch.jpg' },
  { id: 1002, kind: 'news', title: '城市更新样本：老街区如何焕发新活力', source: '城市观察', comments: 214, heat: 88, thumb: '/assets/p_coffee.jpg' }
];

Page({
  data: {
    keyword: '',
    searched: false,
    history: [],
    hotSearch: [
      { word: '数字经济', heat: 210, flag: '爆' },
      { word: '大飞机', heat: 128, flag: '热' },
      { word: '新能源车', heat: 165, flag: '' },
      { word: '房贷利率', heat: 96, flag: '' },
      { word: '联赛绝杀', heat: 188, flag: '' },
      { word: '人工智能', heat: 74, flag: '新' }
    ],
    results: [],
    sortKey: 'rel'
  },

  onLoad: function () {
    this.setData({ history: getApp().getSearchHistory() });
  },

  onInput: function (e) { this.setData({ keyword: e.detail.value }); },
  onClear: function () { this.setData({ keyword: '', searched: false, results: [] }); },
  onTapTag: function (e) {
    this.setData({ keyword: e.currentTarget.dataset.k });
    this.doSearch();
  },
  onSearch: function () { this.doSearch(); },

  doSearch: function () {
    var kw = (this.data.keyword || '').trim();
    if (!kw) {
      wx.showToast({ title: '请输入关键词', icon: 'none' });
      return;
    }
    var hits = LIBRARY.filter(function (a) {
      return a.title.indexOf(kw) >= 0 || a.source.indexOf(kw) >= 0;
    });
    this.setData({
      searched: true,
      results: hits,
      history: getApp().pushSearchHistory(kw)
    });
    this.applySort();
    console.log('🔍 搜索「' + kw + '」命中 ' + hits.length + ' 条');
  },

  onSort: function (e) {
    this.setData({ sortKey: e.currentTarget.dataset.k });
    this.applySort();
  },

  applySort: function () {
    var list = this.data.results.slice();
    if (this.data.sortKey === 'hot') {
      list.sort(function (a, b) { return b.heat - a.heat; });
    }
    this.setData({ results: list });
  },

  onClearHistory: function () {
    this.setData({ history: getApp().clearSearchHistory() });
    wx.showToast({ title: '已清空', icon: 'none' });
  },

  onOpen: function (e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  },

  onGoHot: function () {
    wx.navigateTo({ url: '/pages/hot/hot' });
  },

  onBack: function () {
    wx.navigateBack();
  }
});
