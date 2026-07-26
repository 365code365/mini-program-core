// 购物车：与 storage 联动，结算进入确认订单页
Page({
  data: {
    cartItems: [],
    allSelected: true,
    totalPrice: '0.00',
    selectedCount: 0
  },

  onShow: function () {
    this.loadCart();
  },

  loadCart: function () {
    var cart = getApp().getCart().map(function (item) {
      return {
        id: item.id,
        name: item.name,
        price: item.price,
        image: item.image || '/assets/p_earbuds.jpg',
        spec: item.spec || '默认',
        quantity: item.quantity,
        selected: item.selected !== false
      };
    });
    this.setData({ cartItems: cart });
    this.calculateTotal();
  },

  calculateTotal: function () {
    var total = 0;
    var count = 0;
    var allSelected = this.data.cartItems.length > 0;
    this.data.cartItems.forEach(function (item) {
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
      allSelected: allSelected
    });
  },

  sync: function () {
    getApp().saveCart(this.data.cartItems);
  },

  onToggleSelect: function (e) {
    var index = e.currentTarget.dataset.index;
    var items = this.data.cartItems;
    items[index].selected = !items[index].selected;
    this.setData({ cartItems: items });
    this.sync();
    this.calculateTotal();
  },

  onToggleAll: function () {
    var next = !this.data.allSelected;
    var items = this.data.cartItems.map(function (item) {
      item.selected = next;
      return item;
    });
    this.setData({ cartItems: items, allSelected: next });
    this.sync();
    this.calculateTotal();
  },

  onIncrease: function (e) {
    var index = e.currentTarget.dataset.index;
    var items = this.data.cartItems;
    items[index].quantity++;
    this.setData({ cartItems: items });
    this.sync();
    this.calculateTotal();
  },

  onDecrease: function (e) {
    var index = e.currentTarget.dataset.index;
    var items = this.data.cartItems;
    if (items[index].quantity > 1) {
      items[index].quantity--;
      this.setData({ cartItems: items });
      this.sync();
      this.calculateTotal();
    }
  },

  onDelete: function (e) {
    var index = e.currentTarget.dataset.index;
    var items = this.data.cartItems;
    var self = this;
    var name = items[index].name;
    wx.showModal({
      title: '删除商品',
      content: '确定从购物车移除「' + name + '」吗？',
      success: function (res) {
        if (res.confirm) {
          items.splice(index, 1);
          self.setData({ cartItems: items });
          self.sync();
          self.calculateTotal();
          wx.showToast({ title: '已移除', icon: 'none' });
        }
      }
    });
  },

  onItemTap: function (e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  },

  onCheckout: function () {
    if (this.data.selectedCount === 0) {
      wx.showToast({ title: '请选择商品', icon: 'none' });
      return;
    }
    var picked = this.data.cartItems.filter(function (i) { return i.selected; });
    wx.setStorageSync('checkoutItems', picked);
    wx.navigateTo({ url: '/pages/order/order?from=cart' });
  },

  goShopping: function () {
    wx.switchTab({ url: '/pages/index/index' });
  }
});
