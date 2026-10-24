const data = require('../../utils/data.js');

Page({
  data: { job: null, fav: false, applied: false },
  onLoad(q) {
    const job = data.getJob(q.id || 'j1');
    wx.setNavigationBarTitle({ title: job.co.name });
    const favs = wx.getStorageSync('jobs_fav') || [];
    const applied = wx.getStorageSync('jobs_applied') || [];
    this.setData({ job, fav: favs.indexOf(job.id) >= 0, applied: applied.some(a => a.id === job.id) });
  },
  onFav() {
    const id = this.data.job.id;
    let favs = wx.getStorageSync('jobs_fav') || [];
    const fav = !this.data.fav;
    favs = fav ? favs.concat([id]) : favs.filter(x => x !== id);
    wx.setStorageSync('jobs_fav', favs);
    this.setData({ fav });
    wx.showToast({ title: fav ? '已收藏' : '已取消收藏', icon: 'none' });
  },
  onApply() {
    if (this.data.applied) { wx.showToast({ title: '已经投递过了', icon: 'none' }); return; }
    const resume = wx.getStorageSync('jobs_resume') || {};
    wx.showModal({
      title: '确认投递',
      content: '使用「' + (resume.name || '我') + '」的在线简历投递该职位，HR 一般 3 个工作日内查看',
      confirmText: '投递',
      success: (r) => {
        if (!r.confirm) return;
        const list = wx.getStorageSync('jobs_applied') || [];
        list.unshift({ id: this.data.job.id, time: '刚刚', status: 0 });
        wx.setStorageSync('jobs_applied', list);
        this.setData({ applied: true });
        wx.showToast({ title: '投递成功', icon: 'success' });
      }
    });
  },
  onChat() { wx.switchTab({ url: '/pages/messages/messages' }); },
  onCompany() { wx.navigateTo({ url: '/pages/company/company?id=' + this.data.job.co.id }); }
});
