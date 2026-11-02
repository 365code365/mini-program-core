const S = require('../../utils/store.js');
const F = require('../../utils/forms.js');
const app = getApp();

const TABS = [{ id: 'mine', name: '我发起的' }, { id: 'todo', name: '待我审批' }, { id: 'done', name: '我已处理' }];

Page({
  data: { tabs: TABS, tab: 'todo', list: [], counts: {}, loading: true, types: F.types },
  onShow() {
    if (app.globalData.apprTab) { this.setData({ tab: app.globalData.apprTab }); app.globalData.apprTab = ''; }
    this.load();
  },
  onPullDownRefresh() { this.load(() => wx.stopPullDownRefresh()); },
  load(done) {
    this.setData({ loading: true });
    setTimeout(() => {
      const all = S.approvals();
      const pick = { mine: a => a.mine, todo: a => a.needMe, done: a => a.handled };
      const counts = { mine: all.filter(pick.mine).length, todo: all.filter(pick.todo).length, done: all.filter(pick.done).length };
      const tb = this.getTabBar && this.getTabBar();
      if (tb) tb.setData({ selected: 1, badge: counts.todo });
      this.setData({ list: all.filter(pick[this.data.tab]), counts, loading: false });
      done && done();
    }, 200);
  },
  pickTab(e) { this.setData({ tab: e.currentTarget.dataset.id }); this.load(); },
  open(e) { wx.navigateTo({ url: '/pages/approval/approval?id=' + e.currentTarget.dataset.id }); },
  create(e) { wx.navigateTo({ url: '/pages/apply/apply?type=' + e.currentTarget.dataset.type }); }
});
