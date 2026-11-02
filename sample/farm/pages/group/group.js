const { goods, goodsById } = require('../../utils/data.js');
const store = require('../../utils/store.js');
const ticker = require('../../utils/ticker.js');

const ST = { ongoing: '拼团中', success: '拼团成功', fail: '拼团失败 · 已退款' };

Page({
  data: { grp: null, goods: null, skuText: '', price: '', slots: [], cd: null, statusText: '', tab: 'hall', hall: [], mine: [] },

  onLoad(q) {
    this.q = q;
    if (q.id) wx.setNavigationBarTitle({ title: '拼团详情' });
    else wx.setNavigationBarTitle({ title: '拼团大厅' });
  },
  // 大厅里每个团一个倒计时，都挂在同一个 ticker 上；onHide / onUnload 退订，最后一个退订时定时器清掉
  onShow() { this.refresh(); ticker.attach(this, (now) => this.tick(now)); },
  onHide() { ticker.detach(this); },
  onUnload() { ticker.detach(this); },

  refresh() {
    if (this.q.id) { this.loadGroup(); return; }
    const pool = this.q.goods ? [goodsById(this.q.goods)].concat(goods.filter((g) => g.id !== this.q.goods)) : goods;
    this._hall = pool.map((g) => ({
      id: 'h' + g.id, goodsId: g.id, name: g.name, cover: g.cover, price: g.price, groupPrice: g.groupPrice, size: g.groupSize,
      groups: store.openGroups(g.id).map((og) => Object.assign({}, og, { cd: ticker.left(og.endAt) }))
    })).filter((x) => x.groups.length);
    const mine = store.myGroups().map((x) => {
      const g2 = store.settleGroup(x.id);
      const g = goodsById(g2.goodsId);
      return Object.assign({}, g2, { name: g.name, cover: g.cover, statusText: ST[g2.status] });
    });
    this.setData({ hall: this._hall, mine });
  },
  loadGroup() {
    const grp = store.settleGroup(this.q.id);
    if (!grp) { wx.showToast({ title: '拼团不存在', icon: 'none' }); return; }
    const g = goodsById(grp.goodsId);
    this.setData({
      grp, goods: g, skuText: store.skuText(g, grp.skuKey), price: g.sku.map[grp.skuKey].groupPrice,
      slots: Array.from({ length: grp.size }, (_, i) => grp.members[i] || null), cd: ticker.left(grp.endAt),
      statusText: grp.status === 'ongoing' ? `还差 ${grp.size - grp.members.length} 人成团` : ST[grp.status]
    });
  },
  tick(now) {
    const grp = this.data.grp;
    if (grp) {
      if (grp.status !== 'ongoing') return;
      if (now - grp.lastJoin >= 15000 || now > grp.endAt) this.loadGroup(); else this.setData({ cd: ticker.left(grp.endAt, now) });
      return;
    }
    if (this.data.tab !== 'hall' || !this._hall) return;
    const patch = {};
    this._hall.forEach((h, i) => h.groups.forEach((og, k) => { patch[`hall[${i}].groups[${k}].cd`] = ticker.left(og.endAt, now); }));
    this.setData(patch);
  },

  switchTab(e) { this.setData({ tab: e.currentTarget.dataset.t }); },
  showHall() { this.setData({ tab: 'hall' }); },
  openGoods(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  openGroup(e) { wx.navigateTo({ url: '/pages/group/group?id=' + e.currentTarget.dataset.id }); },
  join(e) {
    const { goods: gid, id } = e.currentTarget.dataset;
    wx.navigateTo({ url: `/pages/detail/detail?id=${gid}&join=${id}` });
  },
  goOrder() { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + this.data.grp.orderId }); },
  onShareAppMessage() {
    if (!this.data.grp) return { title: '田园直供拼团大厅｜产地好物一起拼', path: '/pages/group/group' };
    return { title: `就差你了！${this.data.goods.groupSize}人拼「${this.data.goods.name}」`, path: '/pages/group/group?id=' + this.data.grp.id, imageUrl: this.data.goods.cover };
  }
});
