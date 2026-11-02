// 投递记录：按状态分段；HR 约面试前可以撤回；能直接去聊、去看面试
const S = require('../../utils/store.js');
const TABS = ['全部', '已投递', '被查看', '邀面试', '不合适'];
const CLS = ['st-blue', 'st-orange', 'st-green', 'st-gray'];
Page({
  data: { tabs: TABS, tab: 0, all: [], shown: [], counts: [] },
  onLoad(q) {
    const t = Number(q.tab);
    if (t >= 0 && t < TABS.length) this.setData({ tab: t });
  },
  onShow() { this.refresh(); },
  refresh() {
    const all = S.getApps().map(a => Object.assign({}, a, { cls: CLS[a.status] }));
    const counts = TABS.map((t, i) => (i === 0 ? all.length : all.filter(a => a.status === i - 1).length));
    const t = this.data.tab;
    this.setData({ all, counts, shown: t === 0 ? all : all.filter(a => a.status === t - 1) });
  },
  onTab(e) { this.setData({ tab: Number(e.currentTarget.dataset.i) }); this.refresh(); },
  onJob(e) { wx.navigateTo({ url: '/pages/job/job?id=' + e.currentTarget.dataset.id }); },
  onChat(e) { wx.navigateTo({ url: '/pages/chat/chat?sid=' + S.sessionForJob(e.currentTarget.dataset.id) }); },
  onInterview(e) {
    const v = S.getInterviews().find(x => x.job === e.currentTarget.dataset.id);
    wx.navigateTo({ url: '/pages/interviews/interviews' + (v ? '?id=' + v.id : '') });
  },
  onWithdraw(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '撤回投递', content: '撤回后 HR 将看不到这份投递，之后还可以重新投递。', confirmText: '撤回', confirmColor: '#F5455C',
      success: r => {
        if (!r.confirm) return;
        if (S.withdraw(id)) wx.showToast({ title: '已撤回', icon: 'none' });
        else wx.showToast({ title: 'HR 已约面试，不能撤回了', icon: 'none' });
        this.refresh();
      }
    });
  },
  onGo() { wx.switchTab({ url: '/pages/index/index' }); }
});
