const data = require('../../utils/data.js');
const S = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: { job: null, fav: false, applied: false, app: null, followed: false },
  onLoad(q) {
    const job = data.getJob(q.id || 'j1');
    this.id = job.id;
    wx.setNavigationBarTitle({ title: job.co.name });
    this.setData({ job });
  },
  onShow() {
    const app = S.getApp(this.id);
    this.setData({ fav: S.isFav(this.id), applied: !!app, app, followed: S.isFollow(this.data.job.co.id) });
  },
  onFav() {
    const fav = S.toggleFav(this.id);
    this.setData({ fav });
    wx.showToast({ title: fav ? '已收藏' : '已取消收藏', icon: 'none' });
  },
  onApply() {
    if (this.data.applied) {
      wx.navigateTo({ url: '/pages/applications/applications' });
      return;
    }
    if (!S.resumeReady()) {
      wx.showModal({
        title: '先完善简历', content: '投递需要填写姓名和手机号，HR 才能联系到你。', confirmText: '去完善', confirmColor: '#2E6BFF',
        success: r => r.confirm && wx.navigateTo({ url: '/pages/resume/resume' })
      });
      return;
    }
    const resume = S.getResume();
    wx.showModal({
      title: '确认投递',
      content: '使用「' + resume.name + '」的在线简历投递该职位，HR 一般 3 个工作日内查看',
      confirmText: '投递',
      confirmColor: '#2E6BFF',
      success: (r) => {
        if (!r.confirm) return;
        S.apply(this.id);
        // 投递同时在会话里留一条「简历已发送」，HR 的回复在那边看
        const sid = S.sessionForJob(this.id);
        S.addMsg(sid, { from: 'me', text: '[简历] ' + resume.name + '的在线简历', resume: true });
        this.onShow();
        wx.showToast({ title: '投递成功', icon: 'success' });
      }
    });
  },
  onChat() {
    wx.navigateTo({ url: '/pages/chat/chat?sid=' + S.sessionForJob(this.id) });
  },
  onCompany() { wx.navigateTo({ url: '/pages/company/company?id=' + this.data.job.co.id }); },
  onMap() {
    const c = this.data.job.co;
    host.openMap({ lat: c.lat, lng: c.lng, name: c.name, addr: c.addr });
  },
  onShare() {
    const j = this.data.job;
    wx.setClipboardData({
      data: j.co.name + ' 招聘 ' + j.title + '（' + j.salary + '）https://jobs.example.com/j/' + j.id,
      success: () => wx.showToast({ title: '职位链接已复制', icon: 'none' })
    });
  }
});
