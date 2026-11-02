// 校园通知：分类筛选、下拉刷新（onPullDownRefresh / wx.startPullDownRefresh），未读加粗 + tabBar 角标
const D = require('../../utils/data.js');
const store = require('../../utils/store.js');
const TYPES = ['全部', '教务', '考试', '图书馆', '后勤', '活动', '资助'];
Page({
  data: { types: TYPES, type: '全部', list: [], refreshedAt: '' },
  onShow() { store.syncTab(this, 2); this.refresh(); },
  refresh() {
    const read = store.readSet();
    const list = D.NOTICES.filter(n => this.data.type === '全部' || n.type === this.data.type)
      .map(n => Object.assign({}, n, { unread: read.indexOf(n.id) < 0 }));
    this.setData({ list });
  },
  pick(e) { this.setData({ type: e.currentTarget.dataset.t }); this.refresh(); },
  open(e) { wx.navigateTo({ url: '/pages/notice/notice?id=' + e.currentTarget.dataset.id }); },
  readAll() {
    D.NOTICES.forEach(n => store.markRead(n.id));
    store.syncTab(this, 2);
    this.refresh();
    wx.showToast({ title: '已全部标为已读', icon: 'none' });
  },
  manualRefresh() { wx.startPullDownRefresh(); },
  onPullDownRefresh() {
    setTimeout(() => {
      this.refresh();
      const d = new Date();
      this.setData({ refreshedAt: d.getHours() + ':' + ('0' + d.getMinutes()).slice(-2) });
      wx.stopPullDownRefresh();
    }, 500);
  }
});
