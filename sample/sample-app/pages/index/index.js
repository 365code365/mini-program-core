// 商城首页：轮播 / 金刚区 / 秒杀倒计时 / 入场优惠券 / 触底加载
var PRODUCT_IMAGES = [
  '/assets/p_phone.jpg', '/assets/p_phone2.jpg', '/assets/p_earbuds.jpg',
  '/assets/p_watch.jpg', '/assets/p_sneakers.jpg', '/assets/p_backpack.jpg',
  '/assets/p_coffee.jpg', '/assets/p_cake.jpg', '/assets/p_lipstick.jpg', '/assets/p_thermos.jpg'
];

Page({
  data: {
    searchPlaceholder: '搜索商品 / 品牌',
    bannerIndex: 0,
    banners: [
      { id: 1, tag: '限时特惠', title: '新品上市 全场8折', desc: '精选好物 品质保证', image: '/assets/banner.jpg', link: 101 },
      { id: 2, tag: '数码专场', title: '旗舰手机直降600', desc: '12期免息 顺丰包邮', image: '/assets/p_phone.jpg', link: 106 },
      { id: 3, tag: '会员日', title: '第二件半价', desc: '穿搭好物一站配齐', image: '/assets/p_sneakers.jpg', link: 103 }
    ],
    quickEntries: [
      { key: 'digital', name: '数码', icon: 'success_no_circle', color: '#FF6B35' },
      { key: 'clothes', name: '服饰', icon: 'info_no_circle', color: '#4A90D9' },
      { key: 'beauty', name: '美妆', icon: 'warn_no_circle', color: '#FF69B4' },
      { key: 'food', name: '食品', icon: 'clock', color: '#52C41A' },
      { key: 'coupon', name: '领券', icon: 'arrow_down', color: '#FAAD14' },
      { key: 'orders', name: '订单', icon: 'success_no_circle', color: '#722ed1' },
      { key: 'search', name: '搜索', icon: 'search', color: '#13c2c2' },
      { key: 'showcase', name: '能力', icon: 'star', color: '#EB2F96' },
      { key: 'canvas', name: '画板', icon: 'plus', color: '#2F54EB' },
      { key: 'components', name: '组件', icon: 'success_no_circle', color: '#FA541C' }
    ],
    countdown: { h: '02', m: '00', s: '00' },
    seckillProducts: [],
    showCoupon: false,
    coupons: [
      { id: 1, amount: 10, min: 39, name: '新人无门槛券', tip: '仅限新用户' },
      { id: 2, amount: 30, min: 99, name: '满99减30', tip: '全场通用' },
      { id: 3, amount: 50, min: 199, name: '满199减50', tip: '全场通用' }
    ],
    hotProducts: [
      { id: 101, name: '无线蓝牙耳机', price: 199, image: '/assets/p_earbuds.jpg', badge: '爆款' },
      { id: 102, name: '智能运动手表', price: 599, image: '/assets/p_watch.jpg', badge: '新品' },
      { id: 103, name: '潮流运动鞋', price: 329, image: '/assets/p_sneakers.jpg', badge: '' },
      { id: 104, name: '时尚双肩包', price: 259, image: '/assets/p_backpack.jpg', badge: '' },
      { id: 105, name: '精品咖啡礼盒', price: 89, image: '/assets/p_coffee.jpg', badge: '直降' },
      { id: 106, name: '旗舰智能手机', price: 3999, image: '/assets/p_phone.jpg', badge: '12期免息' }
    ],
    newProducts: [],
    loadingMore: false
  },

  onLoad: function () {
    console.log('🏠 首页加载');
    this.setData({
      newProducts: this.buildProducts(201, 20),
      seckillProducts: this.buildSeckill()
    });
    this.startCountdown();
  },

  onShow: function () {
    // 每次进入首页都弹新人礼包（关闭后 800ms 内不重复弹，避免连点闪动）
    var self = this;
    setTimeout(function () { self.setData({ showCoupon: true }); }, 200);
  },

  onUnload: function () {
    if (this.timer) { clearInterval(this.timer); this.timer = null; }
  },

  // ==================== 数据构造 ====================

  buildProducts: function (startId, count) {
    var names = ['轻薄笔记本电脑', '降噪耳机Pro', '智能音箱', '4K显示器', '机械键盘RGB',
      '游戏鼠标', '固态硬盘1TB', '内存条32GB', '散热器风冷', '电竞椅',
      '桌面音响', '摄像头1080P', 'USB扩展坞', '无线充电器', '平板支架',
      '蓝牙适配器', '屏幕挂灯', '桌面收纳盒', '智能手环', '运动耳机'];
    var descs = ['高性能 热销款', '新品上市 限时优惠', '爆款推荐 好评如潮', '品质保证 顺丰包邮', '厂家直销 假一赔十'];
    var list = [];
    for (var i = 0; i < count; i++) {
      var id = startId + i;
      list.push({
        id: id,
        name: names[i % names.length],
        desc: descs[i % descs.length],
        price: 99 + ((i * 137) % 1900),
        sold: 100 + ((i * 311) % 4800),
        image: PRODUCT_IMAGES[i % PRODUCT_IMAGES.length]
      });
    }
    return list;
  },

  buildSeckill: function () {
    var base = [
      { id: 101, price: 149, originalPrice: 299, stock: 12, image: '/assets/p_earbuds.jpg' },
      { id: 102, price: 499, originalPrice: 799, stock: 8, image: '/assets/p_watch.jpg' },
      { id: 105, price: 59, originalPrice: 129, stock: 26, image: '/assets/p_coffee.jpg' },
      { id: 103, price: 259, originalPrice: 429, stock: 5, image: '/assets/p_sneakers.jpg' },
      { id: 104, price: 199, originalPrice: 359, stock: 31, image: '/assets/p_backpack.jpg' }
    ];
    return base.map(function (p) {
      p.percent = Math.max(8, Math.min(96, 100 - p.stock * 2));
      return p;
    });
  },

  // ==================== 秒杀倒计时（真实走时） ====================

  startCountdown: function () {
    var self = this;
    var end = Date.now() + 2 * 3600 * 1000;
    var tick = function () {
      var left = Math.max(0, end - Date.now());
      var total = Math.floor(left / 1000);
      var pad = function (n) { return n < 10 ? '0' + n : '' + n; };
      self.setData({
        countdown: {
          h: pad(Math.floor(total / 3600)),
          m: pad(Math.floor((total % 3600) / 60)),
          s: pad(total % 60)
        }
      });
    };
    tick();
    if (this.timer) { clearInterval(this.timer); }
    this.timer = setInterval(tick, 1000);
  },

  // ==================== 交互 ====================

  onBannerChange: function (e) {
    this.setData({ bannerIndex: e.detail.current });
  },

  onBannerTap: function (e) {
    var id = e.currentTarget.dataset.id;
    wx.navigateTo({ url: '/pages/detail/detail?id=' + id });
  },

  onQuickTap: function (e) {
    var key = e.currentTarget.dataset.key;
    var routes = {
      coupon: '/pages/coupon/coupon',
      orders: '/pages/orders/orders',
      search: '/pages/search/search',
      showcase: '/pages/showcase/showcase',
      canvas: '/pages/canvas/canvas',
      components: '/pages/components/components'
    };
    if (routes[key]) {
      wx.navigateTo({ url: routes[key] });
    } else {
      wx.switchTab({ url: '/pages/category/category' });
    }
  },

  onGoSearch: function () {
    wx.navigateTo({ url: '/pages/search/search' });
  },

  onGoOrders: function () {
    wx.navigateTo({ url: '/pages/orders/orders' });
  },

  onCloseCoupon: function () {
    this.setData({ showCoupon: false });
  },

  onNoop: function () {},

  onClaimCoupons: function () {
    this.setData({ showCoupon: false });
    wx.setStorageSync('couponClaimed', true);
    wx.showToast({ title: '领取成功', icon: 'success' });
  },

  onProductTap: function (e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  },

  onAddCart: function (e) {
    var product = e.currentTarget.dataset.product;
    getApp().addToCart(product, 1);
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },

  onViewMore: function () {
    wx.switchTab({ url: '/pages/category/category' });
  },

  // ==================== 下拉刷新 / 触底加载 ====================

  onPullDownRefresh: function () {
    var self = this;
    wx.showToast({ title: '刷新中...', icon: 'loading' });
    setTimeout(function () {
      self.setData({ seckillProducts: self.buildSeckill() });
      self.startCountdown();
      wx.stopPullDownRefresh();
      console.log('✅ 刷新完成');
    }, 800);
  },

  onReachBottom: function () {
    if (this.data.loadingMore) { return; }
    var self = this;
    var current = this.data.newProducts;
    this.setData({ loadingMore: true });
    setTimeout(function () {
      var next = self.buildProducts(current[current.length - 1].id + 1, 10);
      self.setData({ newProducts: current.concat(next), loadingMore: false });
      console.log('📦 加载更多，共 ' + (current.length + next.length) + ' 件');
    }, 400);
  }
});
