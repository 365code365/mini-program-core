const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: {
    beans: 0,
    tab: 'mall',
    goods: [],
    logs: []
  },
  onLoad(q) {
    if (q && q.tab === 'log') this.setData({ tab: 'log' });
  },
  onShow() {
    this.load();
  },
  load() {
    const beans = store.getBeans();
    this.setData({
      beans,
      goods: data.beanGoods.map(g => Object.assign({}, g, { cond: g.min ? `满 ${g.min} 可用` : '无门槛', short: Math.max(0, g.beans - beans) })),
      logs: store.beansLog().map((l, k) => Object.assign({ k }, l, { sign: l.delta > 0 ? '+' : '' }))
    });
  },
  onTab(e) {
    this.setData({ tab: e.currentTarget.dataset.tab });
  },
  onExchange(e) {
    const g = this.data.goods.find(x => x.id === String(e.currentTarget.dataset.id));
    if (!g) return;
    if (g.short > 0) {
      wx.showToast({ title: `还差 ${g.short} 个吃货豆`, icon: 'none' });
      return;
    }
    wx.showModal({
      title: '确认兑换',
      content: `用 ${g.beans} 个吃货豆兑换 ¥${g.amount} ${g.title}（${g.cond}，30 天有效）`,
      confirmText: '兑换',
      success: r => {
        if (!r.confirm) return;
        // 弹窗开着的时候豆子可能变了（另一页评价 / 退款），以当前存量为准
        if (store.getBeans() < g.beans) {
          wx.showToast({ title: '吃货豆不足', icon: 'none' });
          this.load();
          return;
        }
        store.changeBeans(-g.beans, `兑换 ¥${g.amount} ${g.title}`);
        store.grantCoupon({ title: '吃货豆兑换红包', amount: g.amount, min: g.min }, 'b');
        this.load();
        wx.showToast({ title: `¥${g.amount} 红包已放入卡包`, icon: 'none' });
      }
    });
  },
  onRule() {
    wx.showModal({
      title: '吃货豆规则',
      content: '每实付 1 元得 1 个吃货豆，评价订单再得 20 个；订单取消或退款，这一单得到的吃货豆会扣回。吃货豆可兑换平台红包，兑换后 30 天内有效。',
      showCancel: false,
      confirmText: '知道了'
    });
  },
  onCoupons() {
    wx.navigateTo({ url: '/pages/coupons/coupons?tab=red' });
  }
});
