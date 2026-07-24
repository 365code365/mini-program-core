// 首页 - 性能测试版本
Page({
  data: {
    showCoupon: false,
    coupons: [
      { id: 1, amount: 10, min: 39, name: '新人无门槛券', tip: '仅限新用户' },
      { id: 2, amount: 30, min: 99, name: '满99减30', tip: '全场通用' },
      { id: 3, amount: 50, min: 199, name: '满199减50', tip: '全场通用' }
    ],
    hotProducts: [
      { id: 101, name: '无线蓝牙耳机', price: 199, image: '/assets/p_earbuds.jpg' },
      { id: 102, name: '智能运动手表', price: 599, image: '/assets/p_watch.jpg' },
      { id: 103, name: '潮流运动鞋', price: 329, image: '/assets/p_sneakers.jpg' },
      { id: 104, name: '时尚双肩包', price: 259, image: '/assets/p_backpack.jpg' },
      { id: 105, name: '精品咖啡礼盒', price: 89, image: '/assets/p_coffee.jpg' },
      { id: 106, name: '旗舰智能手机', price: 3999, image: '/assets/p_phone.jpg' }
    ],
    newProducts: [
      { id: 201, name: '轻薄笔记本电脑', desc: '14英寸高性能 i7处理器', price: 4999 },
      { id: 202, name: '降噪耳机Pro', desc: '40小时续航 主动降噪', price: 899 },
      { id: 203, name: '智能音箱', desc: '语音助手 智能家居控制', price: 299 },
      { id: 204, name: '4K显示器', desc: '27英寸 IPS面板', price: 1999 },
      { id: 205, name: '机械键盘RGB', desc: '青轴 全键无冲', price: 459 },
      { id: 206, name: '游戏鼠标', desc: '16000DPI 可编程按键', price: 299 },
      { id: 207, name: '固态硬盘1TB', desc: 'NVMe协议 读取3500MB/s', price: 599 },
      { id: 208, name: '内存条32GB', desc: 'DDR4 3200MHz', price: 699 },
      { id: 209, name: '散热器风冷', desc: '6热管 静音设计', price: 199 },
      { id: 210, name: '电竞椅', desc: '人体工学 可调节扶手', price: 1299 },
      { id: 211, name: '桌面音响', desc: '2.1声道 蓝牙5.0', price: 399 },
      { id: 212, name: '摄像头1080P', desc: '自动对焦 内置麦克风', price: 199 },
      { id: 213, name: 'USB扩展坞', desc: 'Type-C 10合1', price: 259 },
      { id: 214, name: '无线充电器', desc: '15W快充 兼容多设备', price: 99 },
      { id: 215, name: '平板支架', desc: '铝合金 可折叠', price: 79 },
      { id: 216, name: '蓝牙适配器', desc: '5.0版本 即插即用', price: 39 },
      { id: 217, name: '网线Cat6', desc: '10米 千兆网络', price: 29 },
      { id: 218, name: '鼠标垫超大', desc: '900x400mm 防滑底', price: 49 },
      { id: 219, name: '屏幕挂灯', desc: '护眼 无频闪', price: 149 },
      { id: 220, name: '桌面收纳盒', desc: '多功能 大容量', price: 59 },
      { id: 221, name: '智能手环', desc: '心率监测 睡眠分析', price: 199 },
      { id: 222, name: '运动耳机', desc: '防水IPX7 挂耳式', price: 149 },
      { id: 223, name: '移动电源20000mAh', desc: '双向快充 LED显示', price: 159 },
      { id: 224, name: '数据线套装', desc: 'Type-C/Lightning/Micro', price: 39 },
      { id: 225, name: '手机支架', desc: '车载/桌面两用', price: 29 },
      { id: 226, name: '蓝牙音箱迷你', desc: '便携 防水', price: 99 },
      { id: 227, name: '电子书阅读器', desc: '6英寸墨水屏', price: 699 },
      { id: 228, name: '智能门锁', desc: '指纹/密码/APP', price: 1299 },
      { id: 229, name: '空气净化器', desc: 'HEPA滤网 静音', price: 899 },
      { id: 230, name: '加湿器', desc: '大容量 静音', price: 199 }
    ]
  },

  onLoad: function() {
    console.log('🏠 首页加载');
    // 为新品列表补充真实图片（复用已有商品图，循环使用）
    var imgs = [
      '/assets/p_phone.jpg', '/assets/p_phone2.jpg', '/assets/p_earbuds.jpg',
      '/assets/p_watch.jpg', '/assets/p_sneakers.jpg', '/assets/p_backpack.jpg',
      '/assets/p_coffee.jpg', '/assets/p_cake.jpg', '/assets/p_lipstick.jpg', '/assets/p_thermos.jpg'
    ];
    var list = this.data.newProducts.map(function(p, i) {
      p.image = imgs[i % imgs.length];
      return p;
    });
    this.setData({ newProducts: list });
    // 进入首页弹出新人优惠券（每次会话仅弹一次）
    if (!wx.getStorageSync('couponPopupShown')) {
      this.setData({ showCoupon: true });
      wx.setStorageSync('couponPopupShown', true);
    }
  },

  onCloseCoupon: function() {
    this.setData({ showCoupon: false });
  },
  onNoop: function() {},
  onClaimCoupons: function() {
    this.setData({ showCoupon: false });
    wx.showToast({ title: '领取成功', icon: 'success' });
  },
  onGoCoupon: function() {
    wx.navigateTo({ url: '/pages/coupon/coupon' });
  },

  onReachBottom: function() {
    console.log('📜 触底事件触发 - onReachBottom');
    var self = this;
    var currentProducts = this.data.newProducts;
    var lastId = currentProducts[currentProducts.length - 1].id;
    
    // 生成新商品
    var moreProducts = [];
    var productNames = ['无线耳机', '智能手表', '平板电脑', '游戏手柄', '摄像头', '路由器', '移动硬盘', '显卡', 'CPU', '主板'];
    var productDescs = ['高性能 热销款', '新品上市 限时优惠', '爆款推荐', '品质保证', '厂家直销'];
    
    for (var i = 1; i <= 10; i++) {
      var newId = lastId + i;
      moreProducts.push({
        id: newId,
        name: productNames[(newId - 1) % productNames.length] + ' ' + newId,
        desc: productDescs[(newId - 1) % productDescs.length],
        price: Math.floor(Math.random() * 2000) + 99
      });
    }
    
    // 合并商品列表
    var allProducts = currentProducts.concat(moreProducts);
    this.setData({ newProducts: allProducts });
    console.log('📦 加载更多商品，当前总数:', allProducts.length);
    wx.showToast({ title: '加载了10件商品', icon: 'none' });
  },

  onPullDownRefresh: function() {
    console.log('🔄 下拉刷新触发 - onPullDownRefresh');
    // 可以在这里刷新数据
    wx.showToast({ title: '刷新中...', icon: 'loading' });
    // 模拟刷新完成
    setTimeout(function() {
      wx.stopPullDownRefresh();
      console.log('✅ 刷新完成');
    }, 1000);
  },

  onCategoryTap: function(e) {
    var id = e.currentTarget.dataset.id;
    console.log('📂 点击分类:', id);
    wx.switchTab({ url: '/pages/category/category' });
  },

  onProductTap: function(e) {
    var id = e.currentTarget.dataset.id;
    console.log('📦 查看商品:', id);
    wx.navigateTo({ url: '/pages/detail/detail?id=' + id });
  },

  onAddCart: function(e) {
    var product = e.currentTarget.dataset.product;
    console.log('🛒 加入购物车:', product.name);
    getApp().addToCart(product, 1);
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },

  onViewMore: function() {
    wx.switchTab({ url: '/pages/category/category' });
  },

  onCanvasTap: function() {
    console.log('🎨 进入 Canvas 示例');
    wx.navigateTo({ url: '/pages/canvas/canvas' });
  },

  onComponentsTap: function() {
    console.log('🧩 进入组件示例');
    wx.navigateTo({ url: '/pages/components/components' });
  }
});
