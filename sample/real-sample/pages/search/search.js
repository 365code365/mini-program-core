const app = getApp();
const data = require('../../utils/data.js');

Page({
  data: {
    keyword: '',
    hasSearched: false,
    searchHistory: [],
    hotKeywords: ['蓝牙耳机', '运动手表', '手机', '运动鞋', '口红', '咖啡', '保温杯', '双肩包'],
    guessList: data.pick([105, 107, 110, 108]),
    results: []
  },

  onLoad: function(options) {
    this.loadSearchHistory();
  },

  // 加载搜索历史
  loadSearchHistory: function() {
    try {
      const history = wx.getStorageSync('searchHistory') || [];
      this.setData({ searchHistory: history });
    } catch (e) {
      console.error('加载搜索历史失败', e);
    }
  },

  // 保存搜索历史
  saveSearchHistory: function(keyword) {
    let history = this.data.searchHistory;
    // 移除已存在的相同关键词
    history = history.filter(item => item !== keyword);
    // 添加到开头
    history.unshift(keyword);
    // 最多保存10条
    if (history.length > 10) {
      history = history.slice(0, 10);
    }

    this.setData({ searchHistory: history });
    try {
      wx.setStorageSync('searchHistory', history);
    } catch (e) {
      console.error('保存搜索历史失败', e);
    }
  },

  // 输入
  onInput: function(e) {
    this.setData({ keyword: e.detail.value });
  },

  // 搜索
  onSearch: function() {
    const keyword = this.data.keyword.trim();
    if (!keyword) {
      wx.showToast({ title: '请输入搜索内容', icon: 'none' });
      return;
    }

    this.saveSearchHistory(keyword);
    this.performSearch(keyword);
  },

  // 执行搜索
  performSearch: function(keyword) {
    this.setData({ hasSearched: true });

    // 模拟搜索结果
    const allProducts = data.products;

    // 模糊搜索
    const results = allProducts.filter(p =>
      p.name.includes(keyword) || p.desc.includes(keyword)
    );

    this.setData({ results });
  },

  // 清空输入
  onClear: function() {
    this.setData({ keyword: '', hasSearched: false, results: [] });
  },

  // 取消
  onCancel: function() {
    wx.navigateBack();
  },

  // 清空历史
  onClearHistory: function() {
    wx.showModal({
      title: '确认清空',
      content: '确定要清空搜索历史吗？',
      success: (res) => {
        if (res.confirm) {
          this.setData({ searchHistory: [] });
          wx.setStorageSync('searchHistory', []);
        }
      }
    });
  },

  // 历史搜索点击
  onHistoryTap: function(e) {
    const keyword = e.currentTarget.dataset.keyword;
    this.setData({ keyword });
    this.performSearch(keyword);
  },

  // 热门搜索点击
  onHotTap: function(e) {
    const keyword = e.currentTarget.dataset.keyword;
    this.setData({ keyword });
    this.performSearch(keyword);
  },

  // 商品详情
  onProductTap: function(e) {
    const productId = e.currentTarget.dataset.id;
    wx.navigateTo({
      url: `/pages/detail/detail?id=${productId}`
    });
  },

  // 加入购物车
  onAddCart: function(e) {
    const product = e.currentTarget.dataset.product;
    app.addToCart(product, 1);
  }
});
