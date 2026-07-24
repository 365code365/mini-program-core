// 购物车页面
Page({
  data: {
    cartItems: [],
    allSelected: true,
    totalPrice: '0.00',
    selectedCount: 0
  },

  onLoad: function() {
    console.log('🛒 购物车页加载');
  },

  onShow: function() {
    this.loadCart();
  },

  loadCart: function() {
    var app = getApp();
    var cart = app.globalData.cart || [];
    var items = cart.map(function(item) {
      return {
        id: item.id,
        name: item.name,
        price: item.price,
        quantity: item.quantity,
        selected: true
      };
    });
    
    this.setData({ cartItems: items });
    this.calculateTotal();
  },

  calculateTotal: function() {
    var total = 0;
    var count = 0;
    var allSelected = true;
    
    this.data.cartItems.forEach(function(item) {
      if (item.selected) {
        total += item.price * item.quantity;
        count += item.quantity;
      } else {
        allSelected = false;
      }
    });
    
    this.setData({
      totalPrice: total.toFixed(2),
      selectedCount: count,
      allSelected: allSelected && this.data.cartItems.length > 0
    });
  },

  onToggleSelect: function(e) {
    var index = e.currentTarget.dataset.index;
    var items = this.data.cartItems;
    items[index].selected = !items[index].selected;
    this.setData({ cartItems: items });
    this.calculateTotal();
  },

  onToggleAll: function() {
    var newState = !this.data.allSelected;
    var items = this.data.cartItems.map(function(item) {
      item.selected = newState;
      return item;
    });
    this.setData({ cartItems: items, allSelected: newState });
    this.calculateTotal();
  },

  onIncrease: function(e) {
    var index = e.currentTarget.dataset.index;
    var items = this.data.cartItems;
    items[index].quantity++;
    this.setData({ cartItems: items });
    this.syncToGlobal();
    this.calculateTotal();
  },

  onDecrease: function(e) {
    var index = e.currentTarget.dataset.index;
    var items = this.data.cartItems;
    if (items[index].quantity > 1) {
      items[index].quantity--;
      this.setData({ cartItems: items });
      this.syncToGlobal();
      this.calculateTotal();
    }
  },

  onDelete: function(e) {
    var index = e.currentTarget.dataset.index;
    var items = this.data.cartItems;
    var deleted = items.splice(index, 1);
    console.log('🗑️ 删除商品:', deleted[0].name);
    this.setData({ cartItems: items });
    this.syncToGlobal();
    this.calculateTotal();
  },

  syncToGlobal: function() {
    var app = getApp();
    app.globalData.cart = this.data.cartItems.map(function(item) {
      return {
        id: item.id,
        name: item.name,
        price: item.price,
        quantity: item.quantity
      };
    });
    app.updateCartCount();
  },

  onCheckout: function() {
    if (this.data.selectedCount === 0) {
      wx.showToast({ title: '请选择商品', icon: 'none' });
      return;
    }
    console.log('💳 结算:', this.data.selectedCount, '件商品, 总计:', this.data.totalPrice);
    wx.showToast({ title: '订单提交成功', icon: 'success' });
  },

  goShopping: function() {
    wx.switchTab({ url: '/pages/index/index' });
  }
});
