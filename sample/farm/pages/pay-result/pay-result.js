const store = require('../../utils/store.js');
const ticker = require('../../utils/ticker.js');

Page({
  data: { order: null, grp: null, slots: [], lack: 0, cd: null, title: '支付成功' },
  onLoad(q) { this.id = q.id; },
  onShow() { this.refresh(); ticker.attach(this, (now) => this.tick(now)); },
  onHide() { ticker.detach(this); },
  onUnload() { ticker.detach(this); },
  refresh() {
    const order = store.orderById(this.id);
    if (!order) return;
    const grp = order.groupId ? store.settleGroup(order.groupId) : null;
    const o2 = store.orderById(this.id);
    const slots = grp ? Array.from({ length: grp.size }, (_, i) => grp.members[i] || null) : [];
    const title = !grp ? '支付成功' : grp.status === 'success' ? '拼团成功' : grp.status === 'fail' ? '拼团未成功，已退款' : '开团成功';
    this.setData({ order: o2, grp, slots, lack: grp ? grp.size - grp.members.length : 0, cd: grp ? ticker.left(grp.endAt) : null, title });
  },
  tick(now) {
    const g = this.data.grp;
    if (!g || g.status !== 'ongoing') return;
    // 每 15 秒拼团大厅可能匹配进一位拼友，顺手刷新
    if (now - g.lastJoin >= 15000) this.refresh(); else this.setData({ cd: ticker.left(g.endAt, now) });
  },
  goGroup() { wx.navigateTo({ url: '/pages/group/group?id=' + this.data.grp.id }); },
  goOrder() { wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + this.id }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); },
  onShareAppMessage() {
    const it = this.data.order.items[0];
    return { title: `我在拼「${it.name}」，就差你了！`, path: '/pages/group/group?id=' + this.data.grp.id, imageUrl: it.cover };
  }
});
