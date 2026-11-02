const S = require('../../utils/store.js');

const TABS = [{ id: 'all', name: '全部' }, { id: 'active', name: '咨询中' }, { id: 'done', name: '待评价' }, { id: 'rated', name: '已完成' }];

Page({
  data: { tabs: TABS, tab: 'all', list: [], loading: true },
  onLoad(q) { this.setData({ tab: q.tab || 'all' }); },
  onShow() { this.load(); },
  onPullDownRefresh() { this.load(() => wx.stopPullDownRefresh()); },
  load(done) {
    this.setData({ loading: true });
    setTimeout(() => {
      const all = S.consults().map(c => Object.assign({}, c, { statusText: S.STATUS[c.status], last: (c.messages[c.messages.length - 1] || {}).content || '' }));
      const list = this.data.tab === 'all' ? all : all.filter(c => c.status === this.data.tab);
      this.setData({ list: list.map(c => Object.assign(c, { last: c.last.indexOf('/images/') === 0 ? '[图片]' : c.last })), loading: false });
      done && done();
    }, 250);
  },
  pick(e) { this.setData({ tab: e.currentTarget.dataset.id }); this.load(); },
  open(e) { wx.navigateTo({ url: '/pages/chat/chat?id=' + e.currentTarget.dataset.id }); },
  findLawyer() { wx.switchTab({ url: '/pages/lawyers/lawyers' }); }
});
