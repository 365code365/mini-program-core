const D = require('../../utils/data.js');
const T = require('../../utils/teams.js');
const FILTERS = [{ id: '', name: '全部' }, { id: 'mine', name: '我参与的' }].concat(D.SPORTS.filter((s) => s.id !== 'swim' && s.id !== 'yoga'));
Page({
  data: { filters: FILTERS, filter: '', list: [], loading: true, total: 0 },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 1 });
    // 「我的」页点「我的组局」：switchTab 不带 query，经 globalData 传
    const app = getApp();
    if (app.globalData.teamFilter) { this.setData({ filter: app.globalData.teamFilter }); app.globalData.teamFilter = ''; }
    this.load();
  },
  onPullDownRefresh() { this.load(); wx.stopPullDownRefresh(); wx.showToast({ title: '已刷新', icon: 'none' }); },
  load() {
    const f = this.data.filter;
    const all = T.all().map(T.view).filter((t) => !t.ended);
    const list = all.filter((t) => !f || (f === 'mine' ? t.inIt : t.sport === f))
      .sort((a, b) => (a.full - b.full) || (a.date < b.date ? -1 : a.date > b.date ? 1 : a.start - b.start));
    this.setData({ list, loading: false, total: all.length });
  },
  pick(e) { this.setData({ filter: e.currentTarget.dataset.id }); this.load(); },
  join(e) {
    const t = this.data.list.find((x) => x.id === e.currentTarget.dataset.id);
    if (t.full) { wx.showToast({ title: '已满员，看看其他局', icon: 'none' }); return; }
    wx.showModal({
      title: '报名 ' + t.sportName + '局',
      content: t.dateText + ' ' + t.timeText + ' @' + t.venueName + '\nAA 费 ¥' + t.fee + '/人，到场交给发起人' + t.host.name + '。',
      confirmText: '报名',
      success: (r) => {
        if (!r.confirm) return;
        T.join(t.id);
        this.load();
        wx.showToast({ title: '报名成功', icon: 'success' });
      }
    });
  },
  quit(e) {
    const t = this.data.list.find((x) => x.id === e.currentTarget.dataset.id);
    const host = !!t.mine;
    wx.showModal({
      title: host ? '解散这个局？' : '退出报名？', content: host ? '已报名的 ' + (t.count - 1) + ' 位球友会收到通知。' : '退出后名额会让给其他人。',
      confirmColor: '#e5484d',
      success: (r) => {
        if (!r.confirm) return;
        if (host) T.remove(t.id); else T.quit(t.id);
        this.load();
      }
    });
  },
  venue(e) { wx.navigateTo({ url: '/pages/venue/venue?id=' + e.currentTarget.dataset.v }); },
  create() { wx.navigateTo({ url: '/pages/team-create/team-create' }); },
  onShareAppMessage(res) {
    const id = res && res.target && res.target.dataset && res.target.dataset.id;
    const t = id && this.data.list.find((x) => x.id === id);
    if (t) return { title: t.sportName + '局还差 ' + t.left + ' 人：' + t.dateText + ' ' + t.timeText, path: '/pages/teams/teams' };
    return { title: '约场 · 约球广场，凑人打球更快', path: '/pages/teams/teams' };
  }
});
