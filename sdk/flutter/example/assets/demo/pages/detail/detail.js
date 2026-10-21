Page({
  data: { from: '', query: '', depth: 0 },
  onLoad(q) {
    this.setData({ from: q.from || '（无）', query: JSON.stringify(q), depth: getCurrentPages().length });
  },
  rename() { wx.setNavigationBarTitle({ title: '改过的标题 ' + new Date().getSeconds() }); },
  goBack() {
    if (getCurrentPages().length > 1) wx.navigateBack({});
    else wx.reLaunch({ url: '/pages/index/index' });
  }
});
