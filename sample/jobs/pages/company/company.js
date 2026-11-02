const data = require('../../utils/data.js');
const S = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: { co: null, jobs: [], followed: false },
  onLoad(q) {
    const co = data.companies[q.id] || data.companies.c1;
    wx.setNavigationBarTitle({ title: co.name });
    this.setData({ co });
  },
  onShow() {
    const applied = S.getApps().map(a => a.id);
    this.setData({
      jobs: this.data.co.jobs.map(data.getJob).map(j => Object.assign({}, j, { applied: applied.indexOf(j.id) >= 0 })),
      followed: S.isFollow(this.data.co.id)
    });
  },
  onFollow() {
    const followed = S.toggleFollow(this.data.co.id);
    this.setData({ followed });
    wx.showToast({ title: followed ? '关注成功，有新职位会通知你' : '已取消关注', icon: 'none' });
  },
  onMap() {
    const c = this.data.co;
    host.openMap({ lat: c.lat, lng: c.lng, name: c.name, addr: c.addr });
  },
  onJob(e) { wx.navigateTo({ url: '/pages/job/job?id=' + e.currentTarget.dataset.id }); }
});
