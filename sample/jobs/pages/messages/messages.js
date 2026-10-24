const data = require('../../utils/data.js');

Page({
  data: { filters: ['全部', '未读', '面试'], fIdx: 0, chats: [], shown: [] },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    const read = wx.getStorageSync('jobs_read') || [];
    const chats = data.chats.map((c, idx) => Object.assign({}, c, {
      idx,
      hrInfo: data.hrs[c.hr],
      co: data.companies[c.company],
      jobTitle: data.getJob(c.job).title,
      unread: read.indexOf(c.id) >= 0 ? 0 : c.unread
    }));
    this.setData({ chats });
    this.apply();
  },
  apply() {
    const f = this.data.fIdx;
    const shown = this.data.chats.filter(c => f === 0 || (f === 1 && c.unread > 0) || (f === 2 && c.last.indexOf('面试') >= 0));
    this.setData({ shown });
  },
  onFilter(e) { this.setData({ fIdx: e.currentTarget.dataset.idx }); this.apply(); },
  markRead(ids) {
    const read = wx.getStorageSync('jobs_read') || [];
    wx.setStorageSync('jobs_read', read.concat(ids));
    const chats = this.data.chats.map(c => ids.indexOf(c.id) >= 0 ? Object.assign({}, c, { unread: 0 }) : c);
    this.setData({ chats });
    this.apply();
  },
  onReadAll() { this.markRead(this.data.chats.map(c => c.id)); wx.showToast({ title: '已全部标为已读', icon: 'none' }); },
  onOpen(e) {
    const c = this.data.chats[e.currentTarget.dataset.idx];
    this.markRead([c.id]);
    wx.navigateTo({ url: '/pages/job/job?id=' + c.job });
  }
});
