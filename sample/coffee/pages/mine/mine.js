const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

const LEVELS = [
  { lv: 1, name: '银豆会员', cups: 0 },
  { lv: 2, name: '铜豆会员', cups: 10 },
  { lv: 3, name: '金豆会员', cups: 30 },
  { lv: 4, name: '黑金会员', cups: 80 }
];

function levelOf(cups) {
  let i = 0;
  LEVELS.forEach((l, k) => { if (cups >= l.cups) i = k; });
  const cur = LEVELS[i];
  const next = LEVELS[Math.min(i + 1, LEVELS.length - 1)];
  const span = Math.max(1, next.cups - cur.cups);
  return {
    lv: cur.lv, name: cur.name, next: next.name,
    need: Math.max(0, next.cups - cups),
    percent: Math.min(100, Math.round(((cups - cur.cups) / span) * 100))
  };
}

function today() {
  const d = new Date();
  return `${d.getFullYear()}-${d.getMonth() + 1}-${d.getDate()}`;
}

Page({
  data: {
    cups: 46,
    level: levelOf(46),
    points: 2380,
    couponCount: 4,
    signed: false,
    notify: true,
    perks: [
      { glyph: '折', name: '周三 8 折' },
      { glyph: '升', name: '免费升杯' },
      { glyph: '生', name: '生日礼' },
      { glyph: '优', name: '优先制作' }
    ],
    menus: [
      { name: '我的优惠券', glyph: '券', color: '#b5562a', extra: '' },
      { name: '历史订单', glyph: '单', color: '#8a5a3b', extra: '' },
      { name: '积分商城', glyph: '兑', color: '#c98b4e', extra: '杯子 / 豆子' },
      { name: '收货地址', glyph: '址', color: '#6b8fb5', extra: '' },
      { name: '帮助与客服', glyph: '助', color: '#9b9087', extra: '' }
    ]
  },

  onShow() {
    const orders = store.getOrders();
    const cups = 46 + orders.reduce((n, o) => n + o.count, 0);
    const used = wx.getStorageSync('cf_used_coupons') || [];
    const extra = wx.getStorageSync('cf_extra_coupons') || [];
    const couponCount = data.coupons.filter(c => c.status === 'valid' && used.indexOf(c.id) < 0).length + extra.length;
    this.setData({
      cups,
      level: levelOf(cups),
      points: wx.getStorageSync('cf_points') || 2380,
      couponCount,
      signed: wx.getStorageSync('cf_sign') === today(),
      notify: wx.getStorageSync('cf_notify') !== false,
      'menus[0].extra': couponCount + ' 张可用',
      'menus[1].extra': orders.length ? orders.length + ' 笔' : ''
    });
  },

  onSign() {
    if (this.data.signed) {
      wx.showToast({ title: '今天已经签过啦', icon: 'none' });
      return;
    }
    const points = this.data.points + 10;
    wx.setStorageSync('cf_points', points);
    wx.setStorageSync('cf_sign', today());
    this.setData({ signed: true, points });
    wx.showToast({ title: '签到成功 +10 积分', icon: 'none' });
  },

  onCode() {
    wx.showModal({ title: '会员码', content: '到店出示会员码：8866 2380 1314', showCancel: false });
  },

  onRights() {
    wx.showModal({ title: '金豆会员权益', content: '每周三全场 8 折、每月 1 次免费升杯、生日当月赠饮一杯、高峰期优先制作', showCancel: false, confirmText: '知道了' });
  },

  onPoints() {
    wx.showToast({ title: '100 积分可抵 1 元', icon: 'none' });
  },

  onWallet() {
    wx.showActionSheet({ itemList: ['充 100 送 20', '充 200 送 50', '充 500 送 150'] });
  },

  onCoupons() {
    wx.navigateTo({ url: '/pages/coupons/coupons' });
  },

  onMenu(e) {
    const name = e.currentTarget.dataset.name;
    if (name === '我的优惠券') return this.onCoupons();
    if (name === '历史订单') return wx.switchTab({ url: '/pages/pickup/pickup' });
    if (name === '帮助与客服') {
      wx.showModal({ title: '帮助与客服', content: '客服热线 400-626-1314（08:00-22:00）', showCancel: false });
      return;
    }
    wx.showToast({ title: `${name}即将开放`, icon: 'none' });
  },

  onNotify(e) {
    wx.setStorageSync('cf_notify', e.detail.value);
    this.setData({ notify: e.detail.value });
  }
});
