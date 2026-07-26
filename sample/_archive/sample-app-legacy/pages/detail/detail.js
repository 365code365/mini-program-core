// 商品详情页
Page({
  data: {
    product: null,
    specs: [
      { id: 1, name: '黑色', selected: true },
      { id: 2, name: '白色', selected: false },
      { id: 3, name: '蓝色', selected: false }
    ],
    selectedSpec: '黑色',
    quantity: 1,
    cartCount: 0
  },

  // 商品数据库（模拟）
  productDB: {
    // 热销商品
    101: { id: 101, name: '无线蓝牙耳机 Pro', desc: '高清音质 主动降噪 40小时超长续航', price: 199, originalPrice: 299, discount: '6.7折', sales: 2341, stock: 999 },
    102: { id: 102, name: '智能手表', desc: '心率监测 GPS定位 7天续航', price: 599, originalPrice: 799, discount: '7.5折', sales: 1856, stock: 500 },
    103: { id: 103, name: '便携充电宝', desc: '20000mAh 双向快充 LED显示', price: 129, originalPrice: 199, discount: '6.5折', sales: 5621, stock: 2000 },
    104: { id: 104, name: '机械键盘', desc: '青轴 RGB背光 全键无冲', price: 349, originalPrice: 499, discount: '7折', sales: 892, stock: 300 },
    105: { id: 105, name: '无线鼠标', desc: '静音设计 2.4G连接 长续航', price: 89, originalPrice: 129, discount: '6.9折', sales: 3421, stock: 1500 },
    106: { id: 106, name: '显示器支架', desc: '铝合金 可调节 护颈设计', price: 159, originalPrice: 229, discount: '6.9折', sales: 1234, stock: 800 },
    // 新品推荐
    201: { id: 201, name: '轻薄笔记本电脑', desc: '14英寸高性能 i7处理器 16GB内存', price: 4999, originalPrice: 5999, discount: '8.3折', sales: 456, stock: 100 },
    202: { id: 202, name: '降噪耳机Pro', desc: '40小时续航 主动降噪 Hi-Res认证', price: 899, originalPrice: 1299, discount: '6.9折', sales: 789, stock: 200 },
    203: { id: 203, name: '智能音箱', desc: '语音助手 智能家居控制 360°环绕音', price: 299, originalPrice: 399, discount: '7.5折', sales: 2341, stock: 500 },
    204: { id: 204, name: '4K显示器', desc: '27英寸 IPS面板 99% sRGB', price: 1999, originalPrice: 2499, discount: '8折', sales: 567, stock: 150 },
    205: { id: 205, name: '机械键盘RGB', desc: '青轴 全键无冲 PBT键帽', price: 459, originalPrice: 599, discount: '7.7折', sales: 1023, stock: 400 },
    206: { id: 206, name: '游戏鼠标', desc: '16000DPI 可编程按键 RGB灯效', price: 299, originalPrice: 399, discount: '7.5折', sales: 1567, stock: 600 },
    207: { id: 207, name: '固态硬盘1TB', desc: 'NVMe协议 读取3500MB/s 五年质保', price: 599, originalPrice: 799, discount: '7.5折', sales: 2345, stock: 1000 },
    208: { id: 208, name: '内存条32GB', desc: 'DDR4 3200MHz 终身质保', price: 699, originalPrice: 899, discount: '7.8折', sales: 890, stock: 500 },
    209: { id: 209, name: '散热器风冷', desc: '6热管 静音设计 兼容多平台', price: 199, originalPrice: 299, discount: '6.7折', sales: 678, stock: 300 },
    210: { id: 210, name: '电竞椅', desc: '人体工学 可调节扶手 头枕腰靠', price: 1299, originalPrice: 1699, discount: '7.6折', sales: 234, stock: 100 },
    211: { id: 211, name: '桌面音响', desc: '2.1声道 蓝牙5.0 木质箱体', price: 399, originalPrice: 499, discount: '8折', sales: 456, stock: 200 },
    212: { id: 212, name: '摄像头1080P', desc: '自动对焦 内置麦克风 即插即用', price: 199, originalPrice: 299, discount: '6.7折', sales: 1234, stock: 500 },
    213: { id: 213, name: 'USB扩展坞', desc: 'Type-C 10合1 4K输出', price: 259, originalPrice: 359, discount: '7.2折', sales: 789, stock: 400 },
    214: { id: 214, name: '无线充电器', desc: '15W快充 兼容多设备 防滑设计', price: 99, originalPrice: 149, discount: '6.6折', sales: 3456, stock: 1500 },
    215: { id: 215, name: '平板支架', desc: '铝合金 可折叠 多角度调节', price: 79, originalPrice: 119, discount: '6.6折', sales: 2345, stock: 1000 },
    216: { id: 216, name: '蓝牙适配器', desc: '5.0版本 即插即用 稳定连接', price: 39, originalPrice: 59, discount: '6.6折', sales: 4567, stock: 2000 },
    217: { id: 217, name: '网线Cat6', desc: '10米 千兆网络 纯铜线芯', price: 29, originalPrice: 49, discount: '5.9折', sales: 5678, stock: 3000 },
    218: { id: 218, name: '鼠标垫超大', desc: '900x400mm 防滑底 锁边设计', price: 49, originalPrice: 79, discount: '6.2折', sales: 3456, stock: 1500 },
    219: { id: 219, name: '屏幕挂灯', desc: '护眼 无频闪 色温可调', price: 149, originalPrice: 199, discount: '7.5折', sales: 1234, stock: 500 },
    220: { id: 220, name: '桌面收纳盒', desc: '多功能 大容量 简约设计', price: 59, originalPrice: 89, discount: '6.6折', sales: 2345, stock: 1000 },
    221: { id: 221, name: '智能手环', desc: '心率监测 睡眠分析 防水50米', price: 199, originalPrice: 299, discount: '6.7折', sales: 3456, stock: 800 },
    222: { id: 222, name: '运动耳机', desc: '防水IPX7 挂耳式 8小时续航', price: 149, originalPrice: 199, discount: '7.5折', sales: 2345, stock: 600 },
    223: { id: 223, name: '移动电源20000mAh', desc: '双向快充 LED显示 多口输出', price: 159, originalPrice: 229, discount: '6.9折', sales: 4567, stock: 1200 },
    224: { id: 224, name: '数据线套装', desc: 'Type-C/Lightning/Micro 三合一', price: 39, originalPrice: 59, discount: '6.6折', sales: 6789, stock: 3000 },
    225: { id: 225, name: '手机支架', desc: '车载/桌面两用 可旋转', price: 29, originalPrice: 49, discount: '5.9折', sales: 5678, stock: 2500 },
    226: { id: 226, name: '蓝牙音箱迷你', desc: '便携 防水IPX5 10小时续航', price: 99, originalPrice: 149, discount: '6.6折', sales: 3456, stock: 1000 },
    227: { id: 227, name: '电子书阅读器', desc: '6英寸墨水屏 护眼 超长续航', price: 699, originalPrice: 899, discount: '7.8折', sales: 567, stock: 200 },
    228: { id: 228, name: '智能门锁', desc: '指纹/密码/APP 防盗报警', price: 1299, originalPrice: 1699, discount: '7.6折', sales: 234, stock: 100 },
    229: { id: 229, name: '空气净化器', desc: 'HEPA滤网 静音 适用30㎡', price: 899, originalPrice: 1199, discount: '7.5折', sales: 456, stock: 150 },
    230: { id: 230, name: '加湿器', desc: '大容量5L 静音 智能恒湿', price: 199, originalPrice: 299, discount: '6.7折', sales: 1234, stock: 500 }
  },

  onLoad: function(options) {
    var id = parseInt(options.id) || 101;
    console.log('📦 商品详情页加载, id:', id);
    
    // 从商品数据库获取商品信息
    var product = this.productDB[id];
    if (!product) {
      // 如果找不到，生成一个默认商品
      product = {
        id: id,
        name: '商品 ' + id,
        desc: '优质商品 品质保证',
        price: Math.floor(Math.random() * 500) + 99,
        originalPrice: Math.floor(Math.random() * 800) + 199,
        discount: '7折',
        sales: Math.floor(Math.random() * 5000),
        stock: Math.floor(Math.random() * 1000) + 100
      };
    }
    
    // 添加商品详情
    product.detail = '【产品特点】\n• 高品质材料，经久耐用\n• 精心设计，使用便捷\n• 品牌保证，售后无忧\n• 快速发货，物流可追踪\n\n【包装清单】\n产品 x 1\n说明书 x 1\n保修卡 x 1';
    
    this.setData({ product: product });
    this.updateCartCount();
  },

  onShow: function() {
    this.updateCartCount();
  },

  updateCartCount: function() {
    var app = getApp();
    this.setData({ cartCount: app.globalData.cartCount || 0 });
  },

  onSelectSpec: function(e) {
    var index = e.currentTarget.dataset.index;
    var specs = this.data.specs.map(function(spec, i) {
      spec.selected = (i === index);
      return spec;
    });
    var selected = specs[index].name;
    console.log('🎨 选择规格:', selected);
    this.setData({ specs: specs, selectedSpec: selected });
  },

  onIncrease: function() {
    var qty = this.data.quantity;
    if (qty < this.data.product.stock) {
      this.setData({ quantity: qty + 1 });
    }
  },

  onDecrease: function() {
    var qty = this.data.quantity;
    if (qty > 1) {
      this.setData({ quantity: qty - 1 });
    }
  },

  onAddCart: function() {
    var self = this;
    var product = this.data.product;
    var quantity = this.data.quantity;
    
    // 显示加载中
    wx.showLoading({ title: '添加中...' });
    
    // 模拟网络请求延迟
    setTimeout(function() {
      wx.hideLoading();
      console.log('🛒 加入购物车:', product.name, 'x', quantity);
      getApp().addToCart(product, quantity);
      self.updateCartCount();
      wx.showToast({ title: '已加入购物车', icon: 'success' });
    }, 500);
  },

  onBuyNow: function() {
    var product = this.data.product;
    var quantity = this.data.quantity;
    
    // 显示确认对话框
    wx.showModal({
      title: '确认购买',
      content: '确定要购买 ' + product.name + ' x ' + quantity + ' 吗？\n总价：¥' + (product.price * quantity),
      success: function(res) {
        if (res.confirm) {
          // 用户点击确认
          wx.showLoading({ title: '提交订单...' });
          
          setTimeout(function() {
            wx.hideLoading();
            console.log('💳 立即购买:', product.name, 'x', quantity);
            wx.showToast({ title: '下单成功', icon: 'success' });
          }, 800);
        } else {
          console.log('用户取消购买');
        }
      }
    });
  },

  onGoHome: function() {
    wx.switchTab({ url: '/pages/index/index' });
  },

  onGoCart: function() {
    wx.switchTab({ url: '/pages/cart/cart' });
  }
});
