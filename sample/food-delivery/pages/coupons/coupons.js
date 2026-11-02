const store = require('../../utils/store.js');

const TABS = [
  { key: 'red', name: '红包' },
  { key: 'shop', name: '代金券' },
  { key: 'off', name: '已使用/过期' }
];

function view(c, base) {
  const gap = base === null ? 0 : store.round2(c.min - base);
  return Object.assign({}, c, {
    cond: c.min ? `满 ${c.min} 可用` : '无门槛',
    scope: c.kind === 'shop' ? `仅限 ${c.shopName}` : (c.vip ? '超级会员专享 · 全部商家可用' : '全部商家可用'),
    usable: gap <= 0,
    gapText: gap > 0 ? `还差 ¥${gap}` : ''
  });
}

Page({
  data: {
    tabs: TABS,
    tab: 'red',
    select: '',
    cur: '',
    list: [],
    counts: {},
    code: '',
    // 兑换成功清空输入框时 +1，让它重建（用户打过字的 input，setData 改 value 画面不跟）
    codeSeq: 0,
    showRedeem: false
  },

  // 两种用法：「我的」进来看全部（tab=red|shop）；结算页进来挑一张（select=red|shop&base=当前可抵扣的金额）
  onLoad(q) {
    if (q.select) {
      this.select = q.select;
      this.shopId = q.shop || '';
      this.base = Number(q.base) || 0;
      wx.setNavigationBarTitle({ title: q.select === 'red' ? '选择红包' : '选择代金券' });
      this.setData({ select: q.select, cur: q.cur || '' });
    } else if (q.tab && TABS.some(t => t.key === q.tab)) {
      this.setData({ tab: q.tab });
    }
    this.setData({ showRedeem: q.redeem === '1' });
  },

  onShow() {
    this.load();
  },

  load() {
    const all = store.allCoupons();
    if (this.select) {
      const list = all.filter(c => c.status === 'valid' && c.kind === this.select && (this.select !== 'shop' || c.shopId === this.shopId))
        .map(c => view(c, this.base))
        .sort((a, b) => (b.usable - a.usable) || (b.amount - a.amount));
      this.setData({ list });
      return;
    }
    const pick = k => all.filter(c => (k === 'off' ? c.status !== 'valid' : c.status === 'valid' && c.kind === k));
    this.setData({
      counts: { red: pick('red').length, shop: pick('shop').length, off: pick('off').length },
      list: pick(this.data.tab).map(c => view(c, null))
    });
  },

  onTab(e) {
    this.setData({ tab: e.currentTarget.dataset.key });
    this.load();
  },

  // 选择模式：点一张就带回结算页
  onPick(e) {
    if (!this.select) return;
    const c = this.data.list.find(x => x.id === e.currentTarget.dataset.id);
    if (!c.usable) {
      wx.showToast({ title: `${c.gapText}可用`, icon: 'none' });
      return;
    }
    wx.setStorageSync(this.select === 'red' ? store.K.pickRed : store.K.pickShop, c.id);
    wx.navigateBack();
  },

  onNone() {
    wx.setStorageSync(this.select === 'red' ? store.K.pickRed : store.K.pickShop, 'none');
    wx.navigateBack();
  },

  // 浏览模式「去使用」：代金券去对应的店；红包有购物车就去结算，没有就去首页挑店
  onUse(e) {
    const c = this.data.list.find(x => x.id === e.currentTarget.dataset.id);
    if (c.kind === 'shop') {
      wx.navigateTo({ url: `/pages/shop/shop?id=${c.shopId}` });
      return;
    }
    const cart = store.getCart();
    if (cart.lines.length) {
      wx.setStorageSync(store.K.pickRed, c.id);
      wx.navigateTo({ url: `/pages/checkout/checkout?shop=${cart.shopId}` });
    } else {
      wx.switchTab({ url: '/pages/index/index' });
    }
  },

  onToggleRedeem() {
    this.setData({ showRedeem: !this.data.showRedeem });
  },

  onCode(e) {
    this.setData({ code: e.detail.value });
  },

  onRedeem() {
    const code = this.data.code.trim().toUpperCase();
    if (!code) {
      wx.showToast({ title: '请输入兑换码', icon: 'none' });
      return;
    }
    const r = store.redeem(code);
    if (r.err) {
      wx.showToast({ title: r.err, icon: 'none' });
      return;
    }
    this.setData({ code: '', codeSeq: this.data.codeSeq + 1, tab: 'red' });
    wx.showToast({ title: `兑换成功，¥${r.coupon.amount} 红包已到账`, icon: 'none' });
    this.load();
  }
});
