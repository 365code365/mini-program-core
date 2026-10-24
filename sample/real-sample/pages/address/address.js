const app = getApp();

Page({
  data: {
    addresses: [],
    isCheckout: false,
    checkoutTotal: '0.00',
    checkoutCount: 0
  },

  onLoad: function(options) {
    const isCheckout = options.from === 'checkout';
    const checkoutTotal = app.globalData.checkoutTotal || '0.00';
    const checkoutCount = app.globalData.checkoutProducts?.reduce((sum, p) => sum + p.quantity, 0) || 0;

    this.setData({
      isCheckout,
      checkoutTotal,
      checkoutCount
    });

    this.loadAddresses();
  },

  onShow: function() {
    this.loadAddresses();
  },

  // 加载地址
  loadAddresses: function() {
    this.setData({ addresses: app.globalData.addresses });
  },

  // 选择地址
  onSelectAddress: function(e) {
    if (this.data.isCheckout) {
      const id = parseInt(e.currentTarget.dataset.id);
      const address = this.data.addresses.find(a => a.id === id);
      if (address) {
        // 自动设置为默认
        this.setDefaultAddress(id);
        wx.showToast({ title: '已选择地址', icon: 'success' });
      }
    }
  },

  // 设为默认
  onSetDefault: function(e) {
    const id = parseInt(e.currentTarget.dataset.id);
    this.setDefaultAddress(id);
  },

  setDefaultAddress: function(id) {
    const addresses = this.data.addresses.map(addr => ({
      ...addr,
      isDefault: addr.id === id
    }));
    app.globalData.addresses = addresses;
    this.setData({ addresses });
    wx.showToast({ title: '设置成功', icon: 'success' });
  },

  // 编辑地址
  onEditAddress: function(e) {
    wx.showToast({ title: '编辑功能待开发', icon: 'none' });
  },

  // 删除地址
  onDeleteAddress: function(e) {
    const id = parseInt(e.currentTarget.dataset.id);
    const addresses = this.data.addresses;

    if (addresses.length === 1) {
      wx.showToast({ title: '至少保留一个地址', icon: 'none' });
      return;
    }

    wx.showModal({
      title: '确认删除',
      content: '确定要删除该地址吗？',
      success: (res) => {
        if (res.confirm) {
          const newAddresses = addresses.filter(addr => addr.id !== id);

          // 如果删除的是默认地址，设置第一个为默认
          if (addresses.find(a => a.id === id)?.isDefault && newAddresses.length > 0) {
            newAddresses[0].isDefault = true;
          }

          app.globalData.addresses = newAddresses;
          this.setData({ addresses: newAddresses });
          wx.showToast({ title: '删除成功', icon: 'success' });
        }
      }
    });
  },

  // 添加地址
  onAddAddress: function() {
    wx.showToast({ title: '添加功能待开发', icon: 'none' });
  },

  // 提交订单
  onConfirmOrder: function() {
    const defaultAddress = this.data.addresses.find(addr => addr.isDefault);

    if (!defaultAddress) {
      wx.showToast({ title: '请选择收货地址', icon: 'none' });
      return;
    }

    wx.showLoading({ title: '提交中...' });

    setTimeout(() => {
      wx.hideLoading();

      const products = app.globalData.checkoutProducts || [];
      const total = app.globalData.checkoutTotal || '0.00';

      const order = app.createOrder(defaultAddress, products, total);

      // 清空购物车
      app.globalData.cart = app.globalData.cart.filter(item => !item.selected);
      app.updateCartCount();
      app.saveToStorage();

      // 跳转到订单页
      wx.redirectTo({
        url: `/pages/orders/orders?status=pending`
      });
    }, 1500);
  }
});
