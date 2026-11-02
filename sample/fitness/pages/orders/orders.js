const store = require('../../utils/store.js');
// 我的订单：会员卡（7 天内、最近一笔可退）与单次团课（取消预约时自动退）
Page({
  data: { list: [] },
  onShow() { this.refresh(); },
  refresh() {
    this.setData({
      list: store.getOrders().map(o => Object.assign({}, o, {
        time: store.dateTime(o.createdAt),
        statusText: o.status === 'refund' ? '已退款' : '已支付',
        sub: o.kind === 'card' ? (o.status === 'refund' ? '有效期已退回 ' + o.oldExpire : '有效期顺延至 ' + o.newExpire) : (o.status === 'refund' ? '取消预约，原路退回' : '预约成功，按次扣费'),
        canRefund: store.canRefund(o)
      }))
    });
  },
  onRefund(e) {
    const o = store.getOrders().find(x => x.id === e.currentTarget.dataset.id);
    wx.showModal({
      title: '申请退款',
      content: '「' + o.title + '」¥' + o.price + ' 将原路退回，会员有效期恢复到 ' + o.oldExpire + '，确定吗？',
      confirmText: '确定退款',
      cancelText: '再想想',
      confirmColor: '#ff3d6e',
      success: r => {
        if (!r.confirm) return;
        wx.showToast({ title: store.refundCard(o.id) ? '退款成功，1~3 个工作日到账' : '已超过 7 天，无法退款', icon: 'none' });
        this.refresh();
      }
    });
  },
  goCard() { wx.navigateTo({ url: '/pages/card/card' }); }
});
