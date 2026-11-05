const app = getApp();
const STATUS = ['下拉刷新', '松手刷新，继续下拉进二楼', '正在刷新', '刷新成功', '刷新失败', '松手进入二楼', '打开二楼', '二楼', '关闭二楼'];
Page({
  data: {
    rows: Array.from({ length: 30 }, (_, i) => i), status: 0, statusText: STATUS[0], triggered: false,
    msgs: Array.from({ length: 8 }, (_, i) => ({ id: `m${i}`, t: `消息 ${i + 1}` })), banners: ['a', 'b', 'c'], cur: '', log: []
  },
  onReady() {
    this.createSelectorQuery().select('#sv').node().exec((res) => { this.ctx = res[0] && res[0].node; });
  },
  onStatus(e) { this.setData({ status: e.detail.status, statusText: STATUS[e.detail.status] || '' }); app.note(this, `refresherstatuschange status=${e.detail.status} dy=${Math.round(e.detail.dy)}`); },
  onRefresh() { setTimeout(() => this.setData({ triggered: false }), 1000); this.setData({ triggered: true }); },
  onScroll(e) { if (e.detail.boundaryVelocity !== undefined) app.note(this, `碰到边界 boundaryVelocity=${Math.round(e.detail.boundaryVelocity)}`); },
  onDragEnd(e) { app.note(this, `dragend velocity.y=${Math.round(e.detail.velocity.y)}`); },
  top() { this.ctx && this.ctx.scrollTo({ top: 0 }); },
  into() { this.ctx && this.ctx.scrollIntoView('#r15', { alignment: 'center' }); },
  lock() { if (this.ctx) { this.ctx.scrollEnabled = !this.ctx.scrollEnabled; app.note(this, `scrollEnabled=${this.ctx.scrollEnabled}`); } },
  twoLevel() { this.ctx && this.ctx.triggerTwoLevel(); },
  closeTwo() { this.ctx && this.ctx.closeTwoLevel(); },
  prepend() {
    // scroll-anchoring：上方插入内容，正在看的那条不跳
    const n = this.data.msgs.length;
    this.setData({ msgs: [{ id: `m${n}`, t: `更早的消息 ${n}` }].concat(this.data.msgs) });
  },
  onSwiper(e) { this.setData({ cur: e.detail.currentItemId }); app.note(this, `swiper currentItemId=${e.detail.currentItemId} source=${e.detail.source}`); }
});
