const data = require('../../utils/data.js');

Page({
  data: { co: null, jobs: [], followed: false },
  onLoad(q) {
    const co = data.companies[q.id] || data.companies.c1;
    wx.setNavigationBarTitle({ title: co.name });
    const follows = wx.getStorageSync('jobs_follow') || [];
    this.setData({ co, jobs: co.jobs.map(data.getJob), followed: follows.indexOf(co.id) >= 0 });
  },
  onFollow() {
    let follows = wx.getStorageSync('jobs_follow') || [];
    const followed = !this.data.followed;
    follows = followed ? follows.concat([this.data.co.id]) : follows.filter(x => x !== this.data.co.id);
    wx.setStorageSync('jobs_follow', follows);
    this.setData({ followed });
    wx.showToast({ title: followed ? '关注成功' : '已取消关注', icon: 'none' });
  },
  onJob(e) { wx.navigateTo({ url: '/pages/job/job?id=' + e.currentTarget.dataset.id }); }
});
