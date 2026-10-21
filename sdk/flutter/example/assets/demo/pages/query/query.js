function round(r) {
  if (!r) return r;
  const o = {};
  Object.keys(r).forEach((k) => { o[k] = typeof r[k] === 'number' ? Math.round(r[k]) : r[k]; });
  return o;
}

Page({
  data: {
    queryResult: '点按钮查询（结果在下一帧异步回来）',
    tickResult: '',
    ioVisible: false,
    ioRatio: 0,
    scrollTop: 0,
    fillers: [1, 2, 3, 4, 5, 6, 7, 8],
    more: [],
    bottomTip: '滚到底会触发 onReachBottom，每次加载 5 条'
  },
  onReady() {
    this.io = wx.createIntersectionObserver(this, { thresholds: [0, 0.5, 1] });
    this.io.relativeToViewport({ bottom: 0 }).observe('#target', (res) => {
      this.setData({ ioVisible: res.intersectionRatio > 0, ioRatio: Number(res.intersectionRatio).toFixed(2) });
    });
  },
  onUnload() {
    if (this.io) this.io.disconnect();
  },
  onPageScroll(e) {
    // 每帧都会来：只在跨过阈值或变化够大时 setData，别每一帧都改数据
    const top = Math.round(e.scrollTop);
    if (Math.abs(top - this.data.scrollTop) >= 20 || (top > 200) !== (this.data.scrollTop > 200)) {
      this.setData({ scrollTop: top });
    }
  },
  onReachBottom() {
    const n = this.data.more.length;
    if (n >= 30) {
      this.setData({ bottomTip: '已经加载 30 条，没有更多了' });
      return;
    }
    const more = this.data.more.concat([n + 1, n + 2, n + 3, n + 4, n + 5]);
    this.setData({ more, bottomTip: 'onReachBottom：已加载 ' + more.length + ' 条' });
  },

  measure() {
    wx.createSelectorQuery().select('#box').boundingClientRect((r) => {
      this.setData({ queryResult: 'boundingClientRect #box\n' + JSON.stringify(round(r)) });
    }).exec();
  },
  measureAll() {
    wx.createSelectorQuery().selectAll('.chip').boundingClientRect((list) => {
      this.setData({ queryResult: 'selectAll .chip → ' + list.length + ' 个，宽度：' + list.map((r) => Math.round(r.width)).join(' / ') });
    }).exec();
  },
  viewport() {
    wx.createSelectorQuery().selectViewport().scrollOffset((r) => {
      this.setData({ queryResult: 'selectViewport().scrollOffset\nscrollTop=' + Math.round(r.scrollTop) + ' scrollHeight=' + Math.round(r.scrollHeight) });
    }).exec();
  },
  fields() {
    wx.createSelectorQuery().select('#box').fields({ id: true, dataset: true, size: true, rect: true, computedStyle: ['width', 'height'] }, (r) => {
      this.setData({ queryResult: 'fields #box\n' + JSON.stringify(round(r)) });
    }).exec();
  },
  tick() {
    const order = [];
    order.push('1 同步代码');
    wx.nextTick(() => {
      order.push('3 nextTick');
      this.setData({ tickResult: order.join(' → ') });
    });
    setTimeout(() => {
      order.push('4 setTimeout 0');
      this.setData({ tickResult: order.join(' → ') });
    }, 0);
    order.push('2 同步代码结束');
  },
  toTarget() {
    wx.pageScrollTo({ selector: '#target', offsetTop: -80, duration: 400 });
  },
  toBottom() {
    wx.pageScrollTo({ scrollTop: 99999, duration: 300 });
  },
  toTop() {
    wx.pageScrollTo({ scrollTop: 0, duration: 300 });
  }
});
