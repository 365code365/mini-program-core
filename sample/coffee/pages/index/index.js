const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: {
    greeting: '你好',
    mode: 'pickup',
    store: data.stores[0],
    level: store.levelOf(46),
    points: 2380,
    couponCount: 4,
    balance: 128,
    active: null,
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
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    // 进行中的订单：首页顶部给一条提醒，点进去看取餐码
    const o = store.getOrders().find(x => store.stageOf(x).active);
    const st = o ? store.stageOf(o) : null;
    this.setData({
      mode: store.getMode(),
      store: data.findStore(getApp().globalData.storeId),
      level: store.levelOf(store.cups()),
      points: store.getPoints(),
      couponCount: store.validCoupons().length,
      balance: store.getWallet().balance,
      active: o ? { code: o.code, status: st.status, tip: st.tip } : null
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
    wx.navigateTo({ url: '/pages/points/points' });
  },
  onCoupons() {
    wx.navigateTo({ url: '/pages/coupons/coupons' });
  },
  onWallet() {
    wx.navigateTo({ url: '/pages/wallet/wallet' });
  },
  onActive() {
    wx.switchTab({ url: '/pages/pickup/pickup' });
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
