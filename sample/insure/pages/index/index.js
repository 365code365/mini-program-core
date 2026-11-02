const { CATS, PRODUCTS } = require('../../utils/products.js');
const premium = require('../../utils/premium.js');
const store = require('../../utils/store.js');
const util = require('../../utils/util.js');

const BANNERS = [
  { id: 'p01', img: '/images/banner/b1.jpg', title: '百万医疗险 2026 版', sub: '600 万保额 · 保证续保 20 年', badge: '168 元起' },
  { id: 'p02', img: '/images/banner/b2.jpg', title: '守护多倍重疾险', sub: '120 种重疾 · 最多赔 3 次', badge: '新品' },
  { id: 'p06', img: '/images/banner/b3.jpg', title: '境外旅行险', sub: '申根签证认可 · 全球救援', badge: '45 元起' }
];

function card(p) {
  return {
    id: p.id, name: p.name, cover: p.cover, tagline: p.tagline, tags: p.tags.slice(0, 3),
    price: util.money(premium.priceFrom(p)), sales: (p.sales / 10000).toFixed(1) + '万'
  };
}

Page({
  data: {
    banners: BANNERS, cats: CATS, hot: [], reminder: null, current: 0,
    services: [
      { key: 'calc', name: '保费测算', icon: '/images/cat/critical.png' },
      { key: 'claim', name: '理赔申请', icon: '/images/cat/medical.png' },
      { key: 'policy', name: '保单查询', icon: '/images/cat/life.png' },
      { key: 'call', name: '顾问热线', icon: '/images/cat/travel.png' }
    ]
  },
  onLoad() {
    const hot = PRODUCTS.slice().sort((a, b) => b.sales - a.sales).slice(0, 4).map(card);
    this.setData({ hot });
  },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 0 });
    this.refreshReminder();
  },
  refreshReminder() {
    const list = store.policies();
    const pending = list.filter(p => store.statusOf(p).key === 'pending');
    const active = list.filter(p => ['active', 'waiting'].indexOf(store.statusOf(p).key) >= 0);
    let reminder;
    if (pending.length) reminder = { kind: 'pending', title: '您有 ' + pending.length + ' 份保单待支付', sub: pending[0].productName + ' · ¥' + util.money(pending[0].premium), act: '去支付', id: pending[0].id };
    else if (active.length) reminder = { kind: 'active', title: '安心保已为您守护 ' + active.length + ' 份保障', sub: '最近生效：' + active[0].productName, act: '查看', id: active[0].id };
    else reminder = { kind: 'tip', title: '30 岁前配置重疾险更划算', sub: '同样 50 万保额，30 岁比 40 岁每年少交约 30%', act: '去测算', id: 'p02' };
    this.setData({ reminder });
  },
  onSwiper(e) { this.setData({ current: e.detail.current }); },
  openProduct(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  openCat(e) {
    getApp().globalData.productCat = e.currentTarget.dataset.key;
    wx.switchTab({ url: '/pages/products/products' });
  },
  openSearch() {
    getApp().globalData.productCat = '';
    wx.switchTab({ url: '/pages/products/products' });
  },
  onReminder() {
    const r = this.data.reminder;
    if (r.kind === 'tip') wx.navigateTo({ url: '/pages/calc/calc?id=' + r.id });
    else wx.navigateTo({ url: '/pages/policy/policy?id=' + r.id });
  },
  onService(e) {
    const k = e.currentTarget.dataset.key;
    if (k === 'calc') wx.navigateTo({ url: '/pages/calc/calc?id=p01' });
    else if (k === 'policy') wx.switchTab({ url: '/pages/policies/policies' });
    else if (k === 'claim') {
      const ok = store.policies().filter(p => ['active', 'expired'].indexOf(store.statusOf(p).key) >= 0);
      if (!ok.length) {
        wx.showModal({ title: '暂无可理赔保单', content: '理赔需要一份已生效的保单。先去挑选一份适合您的保障吧。', confirmText: '去看看', success: r => { if (r.confirm) this.openSearch(); } });
      } else wx.navigateTo({ url: '/pages/claim/claim?id=' + ok[0].id });
    } else if (k === 'call') callAdvisor();
  }
});

function callAdvisor() {
  wx.makePhoneCall({
    phoneNumber: '4008109955',
    fail() {
      wx.showModal({
        title: '保险顾问热线', content: '400-810-9955（工作日 9:00-21:00）\n当前设备无法直接拨号，可复制号码后拨打。', confirmText: '复制号码',
        success: r => { if (r.confirm) wx.setClipboardData({ data: '4008109955' }); }
      });
    }
  });
}
