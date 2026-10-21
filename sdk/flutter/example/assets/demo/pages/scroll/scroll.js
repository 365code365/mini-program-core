function makeRows(from, n) {
  const out = [];
  for (let i = from; i < from + n; i++) out.push({ id: i });
  return out;
}

Page({
  data: {
    tabs: ['推荐', '数码', '服饰', '美妆', '食品', '家居', '运动', '图书', '母婴', '宠物', '汽车', '旅行'],
    tabIndex: 0,
    tabInto: '',
    rows: makeRows(1, 30),
    ctxRows: makeRows(0, 30),
    top: 0,
    into: '',
    scrollInfo: '拖动上面的列表；拖到底自动加载 10 条',
    news: [{ id: 1, text: '下拉这个列表试试' }, { id: 2, text: '过了阈值松手触发 refresherrefresh' }, { id: 3, text: '1.2 秒后 setData 把 refresher-triggered 设回 false' }],
    refreshing: false,
    refreshInfo: '还没刷新',
    seq: 3
  },
  pickTab(e) {
    const i = e.currentTarget.dataset.index;
    // 滑到前一个，让选中项不贴着左边
    this.setData({ tabIndex: i, tabInto: 'tab' + Math.max(0, i - 1) });
  },
  onScroll(e) {
    const d = e.detail;
    this.setData({ scrollInfo: 'bindscroll scrollTop=' + Math.round(d.scrollTop) + ' scrollHeight=' + Math.round(d.scrollHeight) + ' deltaY=' + Math.round(d.deltaY) });
  },
  onLower() {
    const rows = this.data.rows;
    if (rows.length >= 80) {
      this.setData({ scrollInfo: 'scrolltolower：已经 80 条，不再加载' });
      return;
    }
    this.setData({ rows: rows.concat(makeRows(rows.length + 1, 10)), scrollInfo: 'scrolltolower：加载到 ' + (rows.length + 10) + ' 条' });
  },
  onUpper() {
    this.setData({ scrollInfo: 'scrolltoupper：回到顶部' });
  },
  toTop() {
    // scroll-top 值不变时不会动：先给一个不同的值
    this.setData({ into: '', top: this.data.top === 0 ? 0.5 : 0 });
  },
  toTen() { this.setData({ into: 'r10' }); },
  toTwenty() { this.setData({ into: 'r20' }); },

  onPulling(e) {
    this.setData({ refreshInfo: 'refresherpulling dy=' + Math.round(e.detail.dy) });
  },
  onRefresh() {
    this.setData({ refreshing: true, refreshInfo: 'refresherrefresh：刷新中…' });
    setTimeout(() => {
      const seq = this.data.seq + 1;
      const d = new Date();
      this.setData({
        seq,
        refreshing: false,
        news: [{ id: seq, text: '新消息 #' + seq + '（' + d.getHours() + ':' + d.getMinutes() + ':' + d.getSeconds() + '）' }].concat(this.data.news)
      });
    }, 1200);
  },
  onRestore() {
    this.setData({ refreshInfo: 'refresherrestore：复位完成，共 ' + this.data.news.length + ' 条' });
  },
  onAbort() {
    this.setData({ refreshInfo: 'refresherabort：没拉过阈值' });
  },

  withCtx(fn) {
    wx.createSelectorQuery().select('#sv3').node((res) => res && res.node && fn(res.node)).exec();
  },
  ctxTo() { this.withCtx((sv) => sv.scrollTo({ top: 300, animated: true, duration: 400 })); },
  ctxInto() { this.withCtx((sv) => sv.scrollIntoView('#s25', { animated: true })); },
  ctxTop() { this.withCtx((sv) => sv.scrollTo({ top: 0, animated: true })); }
});
