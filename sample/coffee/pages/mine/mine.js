const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const SERVICE_PHONE = '400-626-1314';
Page({
  data: {
    cups: 46,
    level: store.levelOf(46),
    points: 2380,
    couponCount: 4,
    balance: 128,
    signed: false,
    signDays: 0,
    signDots: [],
    notify: true,
    perks: [
      { glyph: '折', name: '周三 8 折' },
      { glyph: '活', name: '新品第 2 杯半价' },
      { glyph: '积', name: '消费 1 元 1 积分' },
      { glyph: '退', name: '制作前可取消' }
    ],
    menus: [
      { key: 'coupons', name: '我的优惠券', glyph: '券', color: '#b5562a', extra: '' },
      { key: 'orders', name: '我的订单', glyph: '单', color: '#8a5a3b', extra: '' },
      { key: 'points', name: '积分商城', glyph: '兑', color: '#c98b4e', extra: '券 / 周边' },
      { key: 'address', name: '收货地址', glyph: '址', color: '#6b8fb5', extra: '' },
      { key: 'help', name: '帮助与客服', glyph: '助', color: '#9b9087', extra: '' }
    ]
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 3 });
    const cups = store.cups();
    const orders = store.getOrders();
    const couponCount = store.validCoupons().length;
    const sign = store.signState();
    this.setData({
      cups,
      level: store.levelOf(cups),
      points: store.getPoints(),
      couponCount,
      balance: store.getWallet().balance,
      signed: sign.signed,
      signDays: sign.days,
      signDots: [1, 2, 3, 4, 5, 6, 7].map(n => ({ n, on: n <= sign.days })),
      notify: wx.getStorageSync('cf_notify') !== false,
      'menus[0].extra': couponCount + ' 张可用',
      'menus[1].extra': orders.length ? orders.length + ' 笔' : '',
      'menus[3].extra': store.getAddrs().length + ' 个'
    });
  },
  onSign() {
    const r = store.sign();
    if (!r) {
      wx.showToast({ title: '今天已经签过啦', icon: 'none' });
      return;
    }
    this.onShow();
    if (r.reward) {
      wx.showModal({ title: '连续签到 7 天', content: '+10 积分，并送你一张 5 元无门槛券，已放入我的优惠券', showCancel: false, confirmText: '开心收下' });
    } else {
      wx.showToast({ title: `签到成功 +10 积分，已连续 ${r.days} 天`, icon: 'none' });
    }
  },
  onCode() {
    wx.showModal({ title: '会员码', content: '到店出示会员码：8866 2380 1314', showCancel: false });
  },
  onRights() {
    const l = this.data.level;
    wx.showModal({
      title: `${l.name}权益`,
      content: '金豆及以上会员每周三饮品 8 折；全体会员当季新品第 2 杯半价、消费 1 元得 1 积分，订单在咖啡师制作前可随时取消。',
      showCancel: false,
      confirmText: '知道了'
    });
  },
  onPoints() {
    wx.navigateTo({ url: '/pages/points/points' });
  },
  onWallet() {
    wx.navigateTo({ url: '/pages/wallet/wallet' });
  },
  onCoupons() {
    wx.navigateTo({ url: '/pages/coupons/coupons' });
  },
  onMenu(e) {
    const key = e.currentTarget.dataset.key;
    if (key === 'coupons') return this.onCoupons();
    if (key === 'orders') return wx.navigateTo({ url: '/pages/orders/orders' });
    if (key === 'points') return this.onPoints();
    if (key === 'address') return wx.navigateTo({ url: '/pages/address/address' });
    if (key === 'help') this.onHelp();
  },
  onHelp() {
    const items = data.faqs.map(f => f.q).concat([`拨打客服热线 ${SERVICE_PHONE}`]);
    wx.showActionSheet({
      itemList: items,
      success: res => {
        if (res.tapIndex === data.faqs.length) {
          host.call(SERVICE_PHONE);
          return;
        }
        const f = data.faqs[res.tapIndex];
        wx.showModal({ title: f.q, content: f.a, showCancel: false, confirmText: '知道了' });
      }
    });
  },
  onNotify(e) {
    wx.setStorageSync('cf_notify', e.detail.value);
    this.setData({ notify: e.detail.value });
    wx.showToast({ title: e.detail.value ? '新品上市会第一时间提醒你' : '已关闭新品提醒', icon: 'none' });
  }
});
