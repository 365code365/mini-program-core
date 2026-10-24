const app = getApp();
const data = require('../../utils/data.js');

Page({
  data: {
    activeCategory: 1,
    activeName: '数码',
    scrollTop: 0,
    categories: [
      { id: 1, name: '数码' },
      { id: 2, name: '服饰' },
      { id: 3, name: '美妆' },
      { id: 4, name: '食品' },
      { id: 5, name: '家居' },
      { id: 6, name: '母婴' },
      { id: 7, name: '运动' },
      { id: 8, name: '箱包' }
    ],
    subcategories: [],
    products: []
  },

  onLoad: function(options) {
    const categoryId = options.id ? parseInt(options.id) : 1;
    this.selectCategory(categoryId);
  },

  onShow: function() {
    // 首页的分类入口经 switchTab 过来，参数放在 globalData 里
    const pending = app.globalData.activeCategory;
    if (pending) {
      app.globalData.activeCategory = null;
      this.selectCategory(parseInt(pending));
    }
  },

  // 切换分类
  onCategoryChange: function(e) {
    this.selectCategory(parseInt(e.currentTarget.dataset.id));
  },

  selectCategory: function(categoryId) {
    const cat = this.data.categories.find(c => c.id === categoryId) || this.data.categories[0];
    this.setData({
      activeCategory: cat.id,
      activeName: cat.name,
      scrollTop: 0,
      subcategories: this.getSubcategories(cat.id),
      products: data.pick(data.categoryProducts[cat.id] || [])
    });
  },

  // 子分类：用商品图做缩略图
  getSubcategories: function(categoryId) {
    const img = '/assets/products/';
    const subData = {
      1: [['手机', 'phone'], ['5G新机', 'phone2'], ['耳机', 'earbuds'], ['手表', 'watch']],
      2: [['运动鞋', 'sneakers'], ['双肩包', 'backpack']],
      3: [['口红', 'lipstick']],
      4: [['咖啡', 'coffee'], ['蛋糕', 'cake']],
      5: [['杯壶', 'thermos'], ['咖啡', 'coffee']],
      6: [['水杯', 'thermos'], ['甜点', 'cake']],
      7: [['跑鞋', 'sneakers'], ['手表', 'watch'], ['水杯', 'thermos']],
      8: [['双肩包', 'backpack']]
    };
    return (subData[categoryId] || []).map((s, i) => ({ id: categoryId * 100 + i, name: s[0], image: img + s[1] + '.jpg' }));
  },

  // 子分类点击
  onSubCategoryTap: function(e) {
    wx.showToast({ title: '加载中...', icon: 'loading' });
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
