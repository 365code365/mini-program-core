Page({
  data: {
    count: 1,
    count2: 4,
    extra: 0,
    showExtra: false,
    foldTip: '点卡片标题栏试试'
  },
  onFold(e) {
    this.setData({ foldTip: '组件 triggerEvent fold：folded=' + e.detail.folded });
  },
  footerTap() {
    wx.showToast({ title: 'slot 里的事件归页面', icon: 'none' });
  },
  onCount(e) { this.setData({ count: e.detail.value }); },
  onCount2(e) { this.setData({ count2: e.detail.value }); },
  onExtra(e) { this.setData({ extra: e.detail.value }); },
  resetByMethod() {
    const c = this.selectComponent('#c1');
    if (c) c.reset();
    else wx.showToast({ title: '没找到 #c1', icon: 'none' });
  },
  setFromPage() {
    this.setData({ count: 3, count2: 3 });
  },
  toggleExtra() {
    this.setData({ showExtra: !this.data.showExtra });
  }
});
