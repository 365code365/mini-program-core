const db = require('../../utils/db.js');
const { STAFF } = require('../../utils/data.js');

Page({
  data: { order: null, staff: null, localPhotos: 0 },
  onLoad(q) {
    const order = db.getOrder(q.id);
    if (!order) return;
    this.setData({
      order, staff: STAFF.find((s) => s.id === order.staffId),
      localPhotos: order.photos.filter((p) => p.local).length
    });
  },
  goDetail() { wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + this.data.order.id }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); }
});
