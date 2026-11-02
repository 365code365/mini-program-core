// 收藏的职位 / 关注的公司
const data = require('../../utils/data.js');
const S = require('../../utils/store.js');
Page({
  data: { tab: 0, jobs: [], cos: [] },
  onLoad(q) {
    if (Number(q.tab) === 1) this.setData({ tab: 1 });
  },
  onShow() { this.refresh(); },
  refresh() {
    const applied = S.getApps().map(a => a.id);
    this.setData({
      jobs: S.getFavs().map(id => data.jobs.find(j => j.id === id)).filter(Boolean).map(j => Object.assign(data.decorate(j), { applied: applied.indexOf(j.id) >= 0 })),
      cos: S.getFollows().map(id => data.companies[id]).filter(Boolean).map(c => Object.assign({}, c, { count: c.jobs.length }))
    });
  },
  onTab(e) { this.setData({ tab: Number(e.currentTarget.dataset.i) }); },
  onJob(e) { wx.navigateTo({ url: '/pages/job/job?id=' + e.currentTarget.dataset.id }); },
  onCompany(e) { wx.navigateTo({ url: '/pages/company/company?id=' + e.currentTarget.dataset.id }); },
  onUnfav(e) {
    S.toggleFav(e.currentTarget.dataset.id);
    wx.showToast({ title: '已取消收藏', icon: 'none' });
    this.refresh();
  },
  onUnfollow(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '取消关注', content: '不再接收「' + data.companies[id].name + '」的新职位动态？', confirmText: '取消关注', cancelText: '保留', confirmColor: '#F5455C',
      success: r => {
        if (!r.confirm) return;
        S.toggleFollow(id);
        this.refresh();
      }
    });
  },
  onGo() { wx.switchTab({ url: '/pages/index/index' }); }
});
