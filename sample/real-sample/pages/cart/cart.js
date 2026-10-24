const app = getApp();

Page({
  data: {
    cart: [],
    allSelected: false,
    totalPrice: '0.00',
    selectedCount: 0
  },

  onLoad: function(options) {
    this.loadCart();
  },

  onShow: function() {
    this.loadCart();
  },

  // 加载购物车
  loadCart: function() {
    const cart = app.globalData.cart.map(item => ({ ...item }));
    // 先放进 data 再算合计：calculateTotal 读的是 this.data.cart
    this.setData({ cart });
    this.calculateTotal();
    this.refreshTabBar();
  },

  // 自定义 tabBar 的角标跟着购物车数量走
  refreshTabBar: function() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) {
      this.getTabBar().updateCartCount();
    }
  },

  // 选择商品
  onSelectItem: function(e) {
    const id = parseInt(e.currentTarget.dataset.id);
    const cart = this.data.cart;
    let allSelected = true;

    cart.forEach(item => {
      if (item.id === id) {
        item.selected = !item.selected;
      }
      if (!item.selected) {
        allSelected = false;
      }
    });

    this.setData({ cart, allSelected });
    this.calculateTotal();
    this.saveCart();
  },

  // 全选
  onSelectAll: function() {
    const allSelected = !this.data.allSelected;
    const cart = this.data.cart.map(item => ({
      ...item,
      selected: allSelected
    }));

    this.setData({ cart, allSelected });
    this.calculateTotal();
    this.saveCart();
  },

  // 修改数量
  onQuantityChange: function(e) {
    const id = parseInt(e.currentTarget.dataset.id);
    const type = e.currentTarget.dataset.type;
    const cart = this.data.cart;

    cart.forEach(item => {
      if (item.id === id) {
        if (type === 'plus') {
          item.quantity += 1;
        } else if (type === 'minus' && item.quantity > 1) {
          item.quantity -= 1;
        }
      }
    });

    this.setData({ cart });
    this.calculateTotal();
    this.saveCart();
  },

  // 删除商品
  onDeleteItem: function(e) {
    const id = parseInt(e.currentTarget.dataset.id);
    wx.showModal({
      title: '提示',
      content: '确定要删除该商品吗？',
      success: (res) => {
        if (res.confirm) {
          const cart = this.data.cart.filter(item => item.id !== id);
          this.setData({ cart });
          this.calculateTotal();
          this.saveCart();
          wx.showToast({ title: '删除成功', icon: 'success' });
        }
      }
    });
  },

  // 计算总价
  calculateTotal: function() {
    let total = 0;
    let count = 0;
    let allSelected = true;

    this.data.cart.forEach(item => {
      if (item.selected) {
        total += item.price * item.quantity;
        count += item.quantity;
      } else {
        allSelected = false;
      }
    });

    // 如果购物车为空，重置全选状态
    if (this.data.cart.length === 0) {
      allSelected = false;
    }

    this.setData({
      totalPrice: total.toFixed(2),
      selectedCount: count,
      allSelected
    });
  },

  // 保存购物车
  saveCart: function() {
    app.globalData.cart = this.data.cart;
    app.updateCartCount();
    app.saveToStorage();
    this.refreshTabBar();
  },

  // 结算
  onCheckout: function() {
    if (this.data.selectedCount === 0) {
      wx.showToast({ title: '请选择商品', icon: 'none' });
      return;
    }

    const selectedProducts = this.data.cart.filter(item => item.selected);
    const total = this.data.totalPrice;

    // 存储选中的商品到全局
    app.globalData.checkoutProducts = selectedProducts;
    app.globalData.checkoutTotal = total;

    wx.navigateTo({
      url: `/pages/address/address?from=checkout`
    });
  },

  // 去逛逛
  onGoShopping: function() {
    wx.switchTab({ url: '/pages/index/index' });
  }
});
