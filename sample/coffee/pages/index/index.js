const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

// 等级：按累计杯数
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
    lv: cur.lv,
    name: cur.name,
    next: next.name,
    need: Math.max(0, next.cups - cups),
    percent: Math.min(100, Math.round(((cups - cur.cups) / span) * 100))
  };
}

Page({
  data: {
    greeting: '你好',
    mode: 'pickup',
    store: data.stores[0],
    level: levelOf(46),
    points: 2380,
    couponCount: 4,
    banners: [
      { id: 'k1', kicker: '春季限定 · 第 2 杯半价', title: '橙C美式', desc: '鲜橙撞上日晒豆，一口清醒', c1: '#f59b42', c2: '#d4602a', img: '/images/drinks/orange.png' },
      { id: 'k2', kicker: '新品上市 · 尝鲜价 ¥22', title: '桃桃乌龙拿铁', desc: '白桃 × 冻顶乌龙 × 鲜奶', c1: '#f08c94', c2: '#c9566a', img: '/images/drinks/peach.png' },
      { id: 'k4', kicker: '人气 No.1 · 月售 5 万+', title: '生椰拿铁', desc: '厚椰乳 + 双份浓缩', c1: '#8fae7a', c2: '#4f7a4a', img: '/images/drinks/coconut.png' }
    ],
    recs: []
  },

  onLoad() {
    const h = new Date().getHours();
    const greeting = h < 11 ? '早上好' : h < 14 ? '中午好' : h < 18 ? '下午好' : '晚上好';
    const recs = ['k4', 'k8', 'k1', 'k5', 'k10'].map(id => data.findDrink(id));
    this.setData({ greeting, recs });
  },

  onShow() {
    const storeId = getApp().globalData.storeId;
    const cups = 46 + store.getOrders().reduce((n, o) => n + o.count, 0);
    const used = wx.getStorageSync('cf_used_coupons') || [];
    const extra = wx.getStorageSync('cf_extra_coupons') || [];
    const valid = data.coupons.filter(c => c.status === 'valid' && used.indexOf(c.id) < 0).length + extra.length;
    this.setData({
      mode: store.getMode(),
      store: data.stores.find(s => s.id === storeId) || data.stores[0],
      level: levelOf(cups),
      points: wx.getStorageSync('cf_points') || 2380,
      couponCount: valid
    });
  },

  onMode(e) {
    const mode = e.currentTarget.dataset.mode;
    store.setMode(mode);
    this.setData({ mode });
    wx.switchTab({ url: '/pages/menu/menu' });
  },

  onStore() {
    wx.showActionSheet({
      itemList: data.stores.map(s => `${s.name}  ${s.distance}`),
      success: (res) => {
        const s = data.stores[res.tapIndex];
        getApp().globalData.storeId = s.id;
        wx.setStorageSync('cf_store', s.id);
        this.setData({ store: s });
      }
    });
  },

  onMemberCode() {
    wx.showModal({ title: '会员码', content: '到店出示会员码：8866 2380 1314\n结账自动累计积分与杯数', showCancel: false, confirmText: '好的' });
  },

  onPoints() {
    wx.showToast({ title: '积分商城即将开放', icon: 'none' });
  },

  onCoupons() {
    wx.navigateTo({ url: '/pages/coupons/coupons' });
  },

  onBanner(e) {
    wx.setStorageSync('cf_open_drink', e.currentTarget.dataset.id);
    wx.switchTab({ url: '/pages/menu/menu' });
  },

  onRec(e) {
    wx.setStorageSync('cf_open_drink', e.currentTarget.dataset.id);
    wx.switchTab({ url: '/pages/menu/menu' });
  },

  onMenu() {
    wx.switchTab({ url: '/pages/menu/menu' });
  }
});
