const store = require('../../utils/store.js');
Page({
  data: {
    beans: 0,
    yuan: 0,
    signed: false,
    logs: []
  },
  onShow() {
    this.load();
  },
  load() {
    const beans = store.getBeans();
    this.setData({
      beans,
      yuan: Math.floor(beans / 100),
      signed: store.signed(),
      logs: store.beansLog().map((l, k) => Object.assign({ k }, l, { sign: l.delta > 0 ? '+' : '' }))
    });
  },
  onSign() {
    const r = store.sign();
    if (r === null) {
      wx.showToast({ title: '今天已经签过啦', icon: 'none' });
      return;
    }
    this.load();
    wx.showToast({ title: '签到成功 +20 鲜豆', icon: 'none' });
  },
  onRule() {
    wx.showModal({
      title: '鲜豆规则',
      content: '每日签到 +20，订单签收后每消费 1 元得 1 鲜豆，评价订单另得鲜豆。结算时 100 鲜豆抵 1 元，单笔最多抵 10 元且不超过应付金额的 20%。',
      showCancel: false,
      confirmText: '知道了'
    });
  },
  onUse() {
    wx.switchTab({ url: '/pages/cart/cart' });
  }
});
