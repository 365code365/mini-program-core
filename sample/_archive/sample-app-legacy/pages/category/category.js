// 分类页面 - 外卖风格
Page({
  data: {
    categories: [
      { id: 0, name: '热销', count: 0 },
      { id: 1, name: '主食', count: 0 },
      { id: 2, name: '小吃', count: 0 },
      { id: 3, name: '饮品', count: 0 },
      { id: 4, name: '甜点', count: 0 },
      { id: 5, name: '套餐', count: 0 },
      { id: 6, name: '早餐', count: 0 },
      { id: 7, name: '夜宵', count: 0 },
      { id: 8, name: '沙拉', count: 0 },
      { id: 9, name: '沙拉1', count: 0 },
      { id: 10, name: '沙拉2', count: 0 },
      { id: 11, name: '沙拉4', count: 0 },
      { id: 12, name: '沙拉5', count: 0 },
      { id: 13, name: '沙拉6', count: 0 },
      { id: 14, name: '沙拉1', count: 0 },
      { id: 15, name: '汤品1', count: 0 }
    ],
    currentCategory: 0,
    currentCategoryName: '热销',
    // 直接初始化 products，不依赖 onLoad
    products: [
      { id: 101, name: '招牌炒饭', desc: '蛋炒饭配火腿肠', price: 18, icon: 'success', color: '#FF6B35', quantity: 0 },
      { id: 102, name: '红烧牛肉面', desc: '大块牛肉 劲道面条', price: 26, icon: 'info', color: '#4A90D9', quantity: 0 },
      { id: 103, name: '鸡腿饭', desc: '香酥鸡腿配米饭', price: 22, icon: 'waiting', color: '#52C41A', quantity: 0 }
    ],
    allProducts: {
      0: [
        { id: 101, name: '招牌炒饭', desc: '蛋炒饭配火腿肠', price: 18, icon: 'success', color: '#FF6B35', quantity: 0 },
        { id: 102, name: '红烧牛肉面', desc: '大块牛肉 劲道面条', price: 26, icon: 'info', color: '#4A90D9', quantity: 0 },
        { id: 103, name: '鸡腿饭', desc: '香酥鸡腿配米饭', price: 22, icon: 'waiting', color: '#52C41A', quantity: 0 }
      ],
      1: [
        { id: 201, name: '扬州炒饭', desc: '经典扬州风味', price: 16, icon: 'success', color: '#FF6B35', quantity: 0 },
        { id: 202, name: '蛋炒饭', desc: '简单美味', price: 12, icon: 'info', color: '#4A90D9', quantity: 0 },
        { id: 203, name: '牛肉拌面', desc: '秘制酱料', price: 24, icon: 'waiting', color: '#52C41A', quantity: 0 }
      ],
      2: [
        { id: 301, name: '炸鸡翅', desc: '外酥里嫩 6只装', price: 28, icon: 'success', color: '#FF6B35', quantity: 0 },
        { id: 302, name: '薯条', desc: '金黄酥脆', price: 12, icon: 'info', color: '#4A90D9', quantity: 0 },
        { id: 303, name: '鸡米花', desc: '香脆可口', price: 15, icon: 'waiting', color: '#52C41A', quantity: 0 }
      ],
      3: [
        { id: 401, name: '可乐', desc: '冰爽解渴', price: 6, icon: 'success', color: '#8B4513', quantity: 0 },
        { id: 402, name: '柠檬茶', desc: '清新柠檬', price: 8, icon: 'info', color: '#FFD700', quantity: 0 },
        { id: 403, name: '奶茶', desc: '香浓丝滑', price: 12, icon: 'waiting', color: '#D2691E', quantity: 0 }
      ],
      4: [
        { id: 501, name: '蛋挞', desc: '酥皮蛋挞 2个', price: 10, icon: 'success', color: '#FFD700', quantity: 0 },
        { id: 502, name: '布丁', desc: '焦糖布丁', price: 8, icon: 'info', color: '#FFA500', quantity: 0 }
      ],
      5: [
        { id: 601, name: '单人套餐A', desc: '炒饭+饮料', price: 25, icon: 'success', color: '#FF6B35', quantity: 0 },
        { id: 602, name: '双人套餐', desc: '两份主食+小吃', price: 58, icon: 'info', color: '#4A90D9', quantity: 0 }
      ],
      6: [
        { id: 701, name: '豆浆油条', desc: '经典早餐', price: 8, icon: 'success', color: '#FFD700', quantity: 0 },
        { id: 702, name: '皮蛋瘦肉粥', desc: '暖胃养生', price: 12, icon: 'info', color: '#52C41A', quantity: 0 }
      ],
      7: [
        { id: 801, name: '烧烤拼盘', desc: '多种烤串', price: 38, icon: 'success', color: '#FF6B35', quantity: 0 },
        { id: 802, name: '小龙虾', desc: '麻辣鲜香', price: 68, icon: 'info', color: '#DC143C', quantity: 0 }
      ],
      8: [
        { id: 901, name: '凯撒沙拉', desc: '新鲜蔬菜', price: 22, icon: 'success', color: '#32CD32', quantity: 0 },
        { id: 902, name: '水果沙拉', desc: '时令水果', price: 18, icon: 'info', color: '#FF69B4', quantity: 0 }
      ],
      9: [
        { id: 1001, name: '番茄蛋汤', desc: '家常美味', price: 10, icon: 'success', color: '#FF6347', quantity: 0 },
        { id: 1002, name: '紫菜蛋花汤', desc: '清淡爽口', price: 8, icon: 'info', color: '#8B008B', quantity: 0 }
      ]
    },
    totalCount: 0,
    totalPrice: '0.00',
    cartItems: [],
    showCartPopup: false
  },

  onLoad: function() {
    console.log('📂 分类页加载');
    // 从本地存储恢复购物车
    this.loadCartFromStorage();
    // 初始化显示第一个分类
    this.setData({
      products: this.data.allProducts[0]
    });
  },

  // 从本地存储加载购物车
  loadCartFromStorage: function() {
    var cartData = wx.getStorageSync('categoryCart');
    if (cartData) {
      console.log('📦 恢复购物车数据');
      // 恢复商品数量到allProducts
      var allProducts = this.data.allProducts;
      var categories = this.data.categories;

      for (var catId in allProducts) {
        var catCount = 0;
        for (var i = 0; i < allProducts[catId].length; i++) {
          var product = allProducts[catId][i];
          if (cartData[product.id]) {
            product.quantity = cartData[product.id];
            catCount += product.quantity;
          }
        }
        // 更新分类角标
        for (var j = 0; j < categories.length; j++) {
          if (categories[j].id == catId) {
            categories[j].count = catCount;
          }
        }
      }

      this.setData({
        allProducts: allProducts,
        categories: categories,
        products: allProducts[this.data.currentCategory]
      });
      this.updateCartInfo();
    }
  },

  // 保存购物车到本地存储
  saveCartToStorage: function() {
    var cartData = {};
    var allProducts = this.data.allProducts;
    for (var catId in allProducts) {
      for (var i = 0; i < allProducts[catId].length; i++) {
        var product = allProducts[catId][i];
        if (product.quantity > 0) {
          cartData[product.id] = product.quantity;
        }
      }
    }
    wx.setStorageSync('categoryCart', cartData);
  },

  // 选择分类
  onSelectCategory: function(e) {
    var id = e.currentTarget.dataset.id;
    var categories = this.data.categories;
    var name = '';
    for (var i = 0; i < categories.length; i++) {
      if (categories[i].id == id) {
        name = categories[i].name;
        break;
      }
    }
    console.log('📂 选择分类:', name);
    this.setData({
      currentCategory: id,
      currentCategoryName: name,
      products: this.data.allProducts[id] || []
    });
  },

  // 增加商品数量
  onPlus: function(e) {
    var id = e.currentTarget.dataset.id;
    this.updateProductQuantity(id, 1);
  },

  // 减少商品数量
  onMinus: function(e) {
    var id = e.currentTarget.dataset.id;
    this.updateProductQuantity(id, -1);
  },

  // 购物车弹窗中增加
  onCartPlus: function(e) {
    var id = e.currentTarget.dataset.id;
    this.updateProductQuantity(id, 1);
  },

  // 购物车弹窗中减少
  onCartMinus: function(e) {
    var id = e.currentTarget.dataset.id;
    this.updateProductQuantity(id, -1);
  },

  // 更新商品数量
  updateProductQuantity: function(productId, delta) {
    var allProducts = this.data.allProducts;
    var categories = this.data.categories;
    var found = false;

    // 遍历所有分类找到商品
    for (var catId in allProducts) {
      for (var i = 0; i < allProducts[catId].length; i++) {
        var product = allProducts[catId][i];
        if (product.id == productId) {
          product.quantity = Math.max(0, product.quantity + delta);
          found = true;

          // 更新分类角标
          var catCount = 0;
          for (var j = 0; j < allProducts[catId].length; j++) {
            catCount += allProducts[catId][j].quantity;
          }
          for (var k = 0; k < categories.length; k++) {
            if (categories[k].id == catId) {
              categories[k].count = catCount;
            }
          }
          break;
        }
      }
      if (found) break;
    }

    this.setData({
      allProducts: allProducts,
      categories: categories,
      products: allProducts[this.data.currentCategory]
    });

    this.updateCartInfo();
    this.saveCartToStorage();
  },

  // 更新购物车信息
  updateCartInfo: function() {
    var allProducts = this.data.allProducts;
    var totalCount = 0;
    var totalPrice = 0;
    var cartItems = [];

    for (var catId in allProducts) {
      for (var i = 0; i < allProducts[catId].length; i++) {
        var product = allProducts[catId][i];
        if (product.quantity > 0) {
          totalCount += product.quantity;
          totalPrice += product.price * product.quantity;
          cartItems.push({
            id: product.id,
            name: product.name,
            price: product.price,
            quantity: product.quantity
          });
        }
      }
    }

    this.setData({
      totalCount: totalCount,
      totalPrice: totalPrice.toFixed(2),
      cartItems: cartItems
    });
  },

  // 显示购物车弹窗
  onShowCart: function() {
    if (this.data.totalCount > 0) {
      this.setData({ showCartPopup: true });
    }
  },

  // 隐藏购物车弹窗
  onHideCart: function() {
    this.setData({ showCartPopup: false });
  },

  // 清空购物车
  onClearCart: function() {
    var allProducts = this.data.allProducts;
    var categories = this.data.categories;

    // 重置所有商品数量
    for (var catId in allProducts) {
      for (var i = 0; i < allProducts[catId].length; i++) {
        allProducts[catId][i].quantity = 0;
      }
      // 重置分类角标
      for (var j = 0; j < categories.length; j++) {
        if (categories[j].id == catId) {
          categories[j].count = 0;
        }
      }
    }

    this.setData({
      allProducts: allProducts,
      categories: categories,
      products: allProducts[this.data.currentCategory],
      totalCount: 0,
      totalPrice: '0.00',
      cartItems: [],
      showCartPopup: false
    });

    wx.removeStorageSync('categoryCart');
    wx.showToast({ title: '已清空', icon: 'success' });
  },

  // 去结算
  onCheckout: function() {
    if (this.data.totalCount > 0) {
      wx.showModal({
        title: '确认订单',
        content: '共' + this.data.totalCount + '件商品，合计¥' + this.data.totalPrice,
        success: function(res) {
          if (res.confirm) {
            wx.showToast({ title: '下单成功', icon: 'success' });
          }
        }
      });
    }
  }
});
