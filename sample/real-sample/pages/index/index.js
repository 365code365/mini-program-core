const app = getApp();
const data = require('../../utils/data.js');

Page({
  data: {
    banners: [
      { id: 1, tag: '限时特惠', title: '新品上市 全场8折', desc: '降噪耳机直降200元', image: '/assets/banner_1.jpg', productId: 101 },
      { id: 2, tag: '会员专享', title: '会员日 特惠狂欢', desc: '智能手表 领券再减', image: '/assets/banner_2.jpg', productId: 102 },
      { id: 3, tag: '美妆节', title: '大牌口红 低至6折', desc: '买二送一 先到先得', image: '/assets/banner_3.jpg', productId: 107 }
    ],
    categories: [
      { id: 1, name: '数码', icon: '/assets/icons/cat_digital.png' },
      { id: 2, name: '服饰', icon: '/assets/icons/cat_fashion.png' },
      { id: 3, name: '美妆', icon: '/assets/icons/cat_beauty.png' },
      { id: 4, name: '食品', icon: '/assets/icons/cat_food.png' },
      { id: 5, name: '家居', icon: '/assets/icons/cat_home.png' },
      { id: 6, name: '母婴', icon: '/assets/icons/cat_baby.png' },
      { id: 7, name: '运动', icon: '/assets/icons/cat_sport.png' },
      { id: 8, name: '箱包', icon: '/assets/icons/cat_bag.png' }
    ],
    flashProducts: data.pick([101, 102, 107, 110, 108]),
    hotProducts: data.pick([103, 105, 109, 106]),
    newProducts: data.pick([104, 108, 110])
  },

  onLoad: function(options) {
    console.log('首页加载');
  },

  // 跳转搜索
  onSearchTap: function() {
    wx.navigateTo({ url: '/pages/search/search' });
  },

  // 分类点击（分类是 tabBar 页面，只能 switchTab，带不了参数，先记到全局）
  onCategoryTap: function(e) {
    app.globalData.activeCategory = e.currentTarget.dataset.id;
    wx.switchTab({ url: '/pages/category/category' });
  },

  onBannerTap: function(e) {
    wx.navigateTo({ url: `/pages/detail/detail?id=${e.currentTarget.dataset.id}` });
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
  },

  // 查看全部
  onViewAll: function() {
    wx.switchTab({ url: '/pages/category/category' });
  }
});
