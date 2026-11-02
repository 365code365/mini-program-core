const S = require('../../utils/store.js');
Page({
  data: { filters: ['全部', '未读', '面试'], fIdx: 0, chats: [], shown: [], viewed: 0, pendingIv: 0 },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    this.load();
  },
  load() {
    const apps = S.getApps();
    this.setData({
      chats: S.getSessions(),
      viewed: apps.filter(a => a.status >= 1).length,
      pendingIv: S.pendingInterviews()
    });
    this.apply();
  },
  apply() {
    const f = this.data.fIdx;
    const shown = this.data.chats.filter(c => f === 0 || (f === 1 && c.unread > 0) || (f === 2 && c.hasInvite));
    this.setData({ shown });
  },
  onFilter(e) { this.setData({ fIdx: Number(e.currentTarget.dataset.idx) }); this.apply(); },
  onReadAll() {
    const ids = this.data.chats.filter(c => c.unread > 0).map(c => c.id);
    if (!ids.length) return wx.showToast({ title: '没有未读消息', icon: 'none' });
    S.markRead(ids);
    this.load();
    wx.showToast({ title: '已全部标为已读', icon: 'none' });
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/chat/chat?sid=' + e.currentTarget.dataset.id });
  },
  onAssistant() {
    wx.navigateTo({ url: '/pages/applications/applications' });
  },
  onInterviews() {
    wx.navigateTo({ url: '/pages/interviews/interviews' });
  }
});
