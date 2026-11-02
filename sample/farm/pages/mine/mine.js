const { goodsById } = require('../../utils/data.js');
const store = require('../../utils/store.js');

const ORD = [
  { k: 'unpaid', t: '待付款', ic: '💳' }, { k: 'grouping', t: '待成团', ic: '👥' }, { k: 'paid', t: '待发货', ic: '📦' },
  { k: 'shipped', t: '待收货', ic: '🚚' }, { k: 'after', t: '售后', ic: '🛠' }
];
function today() { const d = new Date(); return `${d.getFullYear()}-${d.getMonth() + 1}-${d.getDate()}`; }

Page({
  data: { nick: '田园会员', levelText: '', signed: false, points: 0, favCount: 0, groupCount: 0, ordTabs: ORD, favs: [], addrCount: 0 },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 3, badge: store.cartCount() });
    const orders = store.orders();
    const count = (k) => orders.filter((o) => (k === 'after' ? o.status === 'refunding' : o.status === k)).length;
    const done = orders.filter((o) => o.status === 'done').length;
    const favs = store.favs().map(goodsById).filter(Boolean).map((g) => ({ id: g.id, cover: g.cover, short: g.name.split(' ')[0] }));
    this.setData({
      ordTabs: ORD.map((x) => Object.assign({}, x, { n: count(x.k) })), favs, favCount: favs.length,
      groupCount: store.myGroups().length, addrCount: store.addresses().length,
      points: wx.getStorageSync('farm_points') || 0, signed: wx.getStorageSync('farm_sign') === today(),
      levelText: done >= 3 ? '金穗会员 · 运费券每月 2 张' : `新苗会员 · 再完成 ${3 - done} 单升级金穗`
    });
  },
  sign() {
    if (this.data.signed) { wx.showToast({ title: '今天已经签过了', icon: 'none' }); return; }
    const p = (wx.getStorageSync('farm_points') || 0) + 5;
    wx.setStorageSync('farm_points', p);
    wx.setStorageSync('farm_sign', today());
    this.setData({ points: p, signed: true });
    wx.showToast({ title: '签到成功 +5', icon: 'success' });
  },
  showPoints() { wx.showModal({ title: '我的积分', content: `当前 ${this.data.points} 积分，100 积分可抵 1 元，下单时自动抵扣（单笔最多抵 10%）`, showCancel: false }); },
  goFavs() {
    if (!this.data.favs.length) { wx.showToast({ title: '还没有收藏商品', icon: 'none' }); return; }
    wx.pageScrollTo({ selector: '#favs', duration: 300 });
  },
  goGroups() { wx.navigateTo({ url: '/pages/group/group' }); },
  goOrders(e) { wx.navigateTo({ url: '/pages/orders/orders?tab=' + e.currentTarget.dataset.tab }); },
  goAddress() { wx.navigateTo({ url: '/pages/address/address' }); },
  openGoods(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  callService() {
    wx.makePhoneCall({ phoneNumber: '4008826517', fail: () => wx.showModal({ title: '田园直供客服', content: '客服热线 4008-826-517\n每天 8:00-22:00，生鲜售后优先处理', showCancel: false }) });
  },
  about() { wx.showModal({ title: '关于田园直供', content: '与全国 40 多个县的合作社直连，产地分拣、冷链直发，不经过批发市场。', showCancel: false }); }
});
