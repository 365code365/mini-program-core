const store = require('../../utils/store');
const { getItem } = require('../../utils/items');

Page({
  data: { order: null, promise: '', time: '' },
  onLoad(q) {
    const order = store.getOrder(q.id) || store.getOrders()[0];
    const item = getItem(order.itemId);
    this.setData({
      order, time: store.fmtTime(order.createdAt),
      promise: item.promise ? `${item.promise} 个工作日内办结` : '即时办结', dept: item.dept
    });
  },
  copyNo() { wx.setClipboardData({ data: this.data.order.id }); },
  goProgress() { wx.navigateTo({ url: '/pages/progress/progress?id=' + this.data.order.id }); },
  // reLaunch 回首页：结果页之前的栈已经清空，用 reLaunch 而不是 switchTab 保证首页重新 onLoad（草稿提示随之消失）
  goHome() { wx.reLaunch({ url: '/pages/index/index' }); }
});
