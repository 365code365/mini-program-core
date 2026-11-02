const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const SLOT_TEXT = { '0': '约满', '-1': '停诊', '-2': '已过' };
Page({
  data: { doc: {}, deptName: '', days: [], dayIdx: 0, halves: [], slot: '', followed: false, today: false },
  onLoad(q) {
    const doc = data.findDoctor(q.id || 'd1') || data.doctors[0];
    this.doc = doc;
    this.today = q.today === '1';
    this.setData({ doc, deptName: data.findDept(doc.dept).name, followed: store.getFollows().indexOf(doc.id) >= 0, today: this.today });
    wx.setNavigationBarTitle({ title: doc.name + ' ' + doc.title });
  },
  // 每次回到这页重算余号：挂完号返回、取消预约后再进来，号源都要对得上
  onShow() {
    const orders = store.getOrders();
    const days = data.nextDays(this.today ? 1 : 7).map(d => {
      const s = store.slotsFor(this.doc, d.idx, orders);
      return Object.assign(d, { total: s.reduce((a, x) => a + Math.max(0, x.left), 0) });
    });
    const keep = this.data.days.length ? this.data.dayIdx : Math.max(0, days.findIndex(d => d.total > 0));
    this.setData({ days });
    this.loadDay(keep, this.data.slot);
  },
  loadDay(i, keepSlot) {
    const s = store.slotsFor(this.doc, i).map(x => Object.assign(x, { text: x.left > 0 ? '余 ' + x.left : SLOT_TEXT[String(x.left)] }));
    const still = keepSlot && s.some(x => x.time === keepSlot && x.left > 0);
    this.setData({
      dayIdx: i,
      slot: still ? keepSlot : '',
      halves: [{ name: '上午', slots: s.filter(x => x.half === '上午') }, { name: '下午', slots: s.filter(x => x.half === '下午') }]
    });
  },
  pickDay(e) { this.loadDay(e.currentTarget.dataset.index); },
  pickSlot(e) {
    const { time, left } = e.currentTarget.dataset;
    if (left <= 0) {
      wx.showToast({ title: { '0': '该时段已约满', '-1': '该时段停诊', '-2': '该时段已结束' }[String(left)], icon: 'none' });
      return;
    }
    this.setData({ slot: time });
  },
  toggleFollow() {
    let f = store.getFollows();
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
