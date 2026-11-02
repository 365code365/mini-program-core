const data = require('../../utils/data.js');
Page({
  data: { n: null, more: [] },
  onLoad(q) {
    const n = data.findNews(q.id) || data.news[0];
    this.setData({ n, more: data.news.filter(x => x.id !== n.id).slice(0, 3) });
  },
  // 文末按钮：去挂号 / 去问诊 / 去导航
  onAction() {
    const a = this.data.n.action;
    if (a.type === 'dept') wx.navigateTo({ url: '/pages/doctors/doctors?dept=' + a.id });
    else if (a.type === 'consult') wx.navigateTo({ url: '/pages/consult/consult' });
    else wx.navigateTo({ url: '/pages/navi/navi' });
  },
  openMore(e) { wx.redirectTo({ url: '/pages/news/news?id=' + e.currentTarget.dataset.id }); }
});
