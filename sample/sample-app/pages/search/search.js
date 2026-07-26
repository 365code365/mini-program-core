// 搜索：历史（storage）/ 热搜 / 排序结果
var CATALOG = [
  { id: 101, name: '无线蓝牙耳机 Pro', price: 199, sales: 2341, image: '/assets/p_earbuds.jpg', tip: '降噪 · 40小时续航' },
  { id: 102, name: '智能运动手表', price: 599, sales: 1856, image: '/assets/p_watch.jpg', tip: '心率 · GPS' },
  { id: 103, name: '潮流运动鞋', price: 329, sales: 3120, image: '/assets/p_sneakers.jpg', tip: '轻量缓震' },
  { id: 104, name: '时尚双肩包', price: 259, sales: 1580, image: '/assets/p_backpack.jpg', tip: '大容量防泼水' },
  { id: 105, name: '精品咖啡礼盒', price: 89, sales: 5621, image: '/assets/p_coffee.jpg', tip: '中深烘焙 12 包' },
  { id: 106, name: '旗舰智能手机', price: 3999, sales: 890, image: '/assets/p_phone.jpg', tip: '100W 快充' },
  { id: 107, name: '轻奢口红套装', price: 269, sales: 4210, image: '/assets/p_lipstick.jpg', tip: '三色礼盒' },
  { id: 108, name: '保温杯 500ml', price: 129, sales: 2760, image: '/assets/p_thermos.jpg', tip: '316 内胆' },
  { id: 109, name: '手工蛋糕礼盒', price: 158, sales: 980, image: '/assets/p_cake.jpg', tip: '当日现做' }
];

Page({
  data: {
    keyword: '',
    searched: false,
    history: [],
    hotWords: [
      { word: '蓝牙耳机' }, { word: '手表' }, { word: '运动鞋' },
      { word: '咖啡' }, { word: '手机' }, { word: '口红' }, { word: '保温杯' }
    ],
    ranking: [],
    results: [],
    sortKey: 'default'
  },

  onLoad: function () {
    var ranking = CATALOG.slice().sort(function (a, b) { return b.sales - a.sales; }).slice(0, 5);
    this.setData({ history: getApp().getSearchHistory(), ranking: ranking });
  },

  onInput: function (e) {
    this.setData({ keyword: e.detail.value });
  },

  onClear: function () {
    this.setData({ keyword: '', searched: false, results: [] });
  },

  onTapTag: function (e) {
    this.setData({ keyword: e.currentTarget.dataset.k });
    this.doSearch();
  },

  onSearch: function () {
    this.doSearch();
  },

  doSearch: function () {
    var kw = (this.data.keyword || '').trim();
    if (!kw) {
      wx.showToast({ title: '请输入关键词', icon: 'none' });
      return;
    }
    var history = getApp().pushSearchHistory(kw);
    var hits = CATALOG.filter(function (p) {
      return p.name.indexOf(kw) >= 0 || p.tip.indexOf(kw) >= 0;
    });
    this.setData({ searched: true, history: history, results: hits });
    this.applySort();
    console.log('🔍 搜索「' + kw + '」命中 ' + hits.length + ' 条');
  },

  onSort: function (e) {
    this.setData({ sortKey: e.currentTarget.dataset.k });
    this.applySort();
  },

  applySort: function () {
    var key = this.data.sortKey;
    var list = this.data.results.slice();
    if (key === 'price') { list.sort(function (a, b) { return a.price - b.price; }); }
    if (key === 'sales') { list.sort(function (a, b) { return b.sales - a.sales; }); }
    this.setData({ results: list });
  },

  onClearHistory: function () {
    this.setData({ history: getApp().clearSearchHistory() });
    wx.showToast({ title: '已清空', icon: 'none' });
  },

  onProductTap: function (e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  },

  onBack: function () {
    wx.navigateBack();
  }
});
