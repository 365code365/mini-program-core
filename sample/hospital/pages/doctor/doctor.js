const data = require('../../utils/data.js');

Page({
  data: { doc: {}, deptName: '', days: [], dayIdx: 0, halves: [], slot: '', followed: false },
  onLoad(q) {
    const doc = data.findDoctor(q.id || 'd1') || data.doctors[0];
    const days = data.nextDays(7).map(d => {
      const s = data.slotsFor(doc, d.idx);
      return Object.assign(d, { total: s.reduce((a, x) => a + Math.max(0, x.left), 0) });
    });
    const firstOpen = Math.max(0, days.findIndex(d => d.total > 0));
    const follows = wx.getStorageSync('hs_follow') || [];
    this.doc = doc;
    this.setData({ doc, deptName: data.findDept(doc.dept).name, days, followed: follows.indexOf(doc.id) >= 0 });
    this.loadDay(firstOpen);
    wx.setNavigationBarTitle({ title: doc.name + ' ' + doc.title });
  },
  loadDay(i) {
    const s = data.slotsFor(this.doc, i);
    this.setData({
      dayIdx: i,
      slot: '',
      halves: [{ name: '上午', slots: s.filter(x => x.half === '上午') }, { name: '下午', slots: s.filter(x => x.half === '下午') }]
    });
  },
  pickDay(e) { this.loadDay(e.currentTarget.dataset.index); },
  pickSlot(e) {
    const { time, left } = e.currentTarget.dataset;
    if (left <= 0) {
      wx.showToast({ title: left === 0 ? '该时段已约满' : '该时段停诊', icon: 'none' });
      return;
    }
    this.setData({ slot: time });
  },
  toggleFollow() {
    let f = wx.getStorageSync('hs_follow') || [];
    const on = !this.data.followed;
    f = on ? f.concat(this.doc.id) : f.filter(x => x !== this.doc.id);
    wx.setStorageSync('hs_follow', f);
    this.setData({ followed: on });
    wx.showToast({ title: on ? '已关注，放号提醒您' : '已取消关注', icon: 'none' });
  },
  book() {
    if (!this.data.slot) {
      wx.showToast({ title: '请先选择就诊时段', icon: 'none' });
      return;
    }
    const day = this.data.days[this.data.dayIdx];
    wx.navigateTo({ url: '/pages/confirm/confirm?id=' + this.doc.id + '&date=' + day.key + '&time=' + this.data.slot });
  }
});
