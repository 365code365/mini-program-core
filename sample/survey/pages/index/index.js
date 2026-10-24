const data = require('../../utils/data.js');

Page({
  data: { todayText: '2.3万', keyword: '', segs: ['全部', '问卷', '投票', '未参与'], sIdx: 0, list: [], mine: [] },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    const mine = data.getMine().map(m => ({ id: m.id, title: m.title, total: m.counts.reduce((a, b) => a + b, 0), ended: !!m.closed || m.deadline < data.today() }));
    this.setData({ mine });
    this.refresh();
  },
  refresh() {
    const ans = data.getAnswers();
    const kw = this.data.keyword.trim();
    const s = this.data.sIdx;
    const list = data.surveys
      .filter(x => !kw || (x.title + x.desc).indexOf(kw) >= 0)
      .filter(x => s === 0 || (s === 1 && x.kind === '问卷') || (s === 2 && x.kind === '投票') || (s === 3 && !ans[x.id]))
      .map(x => ({
        id: x.id, kind: x.kind, title: x.title, desc: x.desc, bg: x.bg, glyph: x.glyph, hot: x.hot,
        done: !!ans[x.id], joinedText: data.fmtNum(x.joined + (ans[x.id] ? 1 : 0)), qCount: x.questions.length,
        deadlineShort: x.deadline.slice(5).replace('-', '/')
      }));
    this.setData({ list });
  },
  onSearch(e) { this.setData({ keyword: e.detail.value }); this.refresh(); },
  onSeg(e) { this.setData({ sIdx: e.currentTarget.dataset.idx }); this.refresh(); },
  onOpen(e) {
    const id = e.currentTarget.dataset.id;
    if (data.getAnswers()[id]) {
      wx.showModal({ title: '你已参与过', content: '要直接查看统计结果吗？', confirmText: '看结果', cancelText: '重新作答', confirmColor: '#6C5CE7',
        success: (r) => wx.navigateTo({ url: r.confirm ? '/pages/result/result?id=' + id : '/pages/answer/answer?id=' + id }) });
      return;
    }
    wx.navigateTo({ url: '/pages/answer/answer?id=' + id });
  },
  onResult(e) { wx.navigateTo({ url: '/pages/result/result?id=' + e.currentTarget.dataset.id }); },
  onCreate() { wx.switchTab({ url: '/pages/create/create' }); },
  onMine() { wx.switchTab({ url: '/pages/mine/mine' }); }
});
