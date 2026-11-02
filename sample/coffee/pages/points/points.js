const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: {
    points: 0,
    tab: 'mall',
    mall: data.mall,
    logs: [],
    records: []
  },
  onLoad(q) {
    if (q && q.tab) this.setData({ tab: q.tab });
  },
  onShow() {
    this.load();
  },
  load() {
    this.setData({
      points: store.getPoints(),
      logs: store.pointsLog().map((l, k) => Object.assign({ k }, l, { sign: l.delta > 0 ? '+' : '' })),
      records: store.exchanges()
    });
  },
  onTab(e) {
    this.setData({ tab: e.currentTarget.dataset.tab });
  },
  onExchange(e) {
    const item = data.mall.find(m => m.id === e.currentTarget.dataset.id);
    if (this.data.points < item.points) {
      wx.showToast({ title: `还差 ${item.points - this.data.points} 积分`, icon: 'none' });
      return;
    }
    wx.showModal({
      title: '确认兑换',
      content: `用 ${item.points} 积分兑换「${item.name}」${item.type === 'coupon' ? '，兑换后放入我的优惠券' : '，凭兑换码到店领取'}`,
      confirmText: '兑换',
      success: r => {
        if (!r.confirm) return;
        store.changePoints(-item.points, `兑换 ${item.name}`);
        if (item.type === 'coupon') {
          store.grantCoupon(item.coupon, 'xm');
          wx.showToast({ title: '已放入我的优惠券', icon: 'success' });
        } else {
          const code = String(100000 + Math.floor(Math.random() * 900000));
          const s = data.findStore(getApp().globalData.storeId);
          store.addExchange({ id: 'e' + Date.now().toString(36), name: item.name, points: item.points, code, store: s.name, time: store.dateTime(Date.now()) });
          wx.showModal({ title: '兑换成功', content: `兑换码 ${code}\n请到「${s.name}」出示兑换码领取`, showCancel: false, confirmText: '好的' });
        }
        this.load();
      }
    });
  },
  onRule() {
    wx.showModal({
      title: '积分规则',
      content: '每消费 1 元得 1 积分，每日签到 +10，评价订单 +20。下单时 100 积分抵 1 元（单笔最多 500 积分）；积分商城的券与周边用积分直接兑换。',
      showCancel: false,
      confirmText: '知道了'
    });
  },
  onCoupons() {
    wx.navigateTo({ url: '/pages/coupons/coupons' });
  }
});
