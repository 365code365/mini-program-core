const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
// 记一次训练：选类型、填时长，消耗按类型估算（自己改过就不再覆盖），保存后自动打卡
Page({
  data: { types: data.workoutTypes, typeIdx: 0, name: '', min: '', kcal: '', kcalTouched: false, date: '', time: '', today: '' },
  onLoad() {
    const t = store.today();
    this.setData({ date: t, today: t, time: store.hm(Date.now()) });
  },
  pickType(e) {
    this.setData({ typeIdx: Number(e.currentTarget.dataset.i) });
    this.estimate();
  },
  onName(e) { this.setData({ name: e.detail.value }); },
  onMin(e) {
    this.setData({ min: e.detail.value });
    this.estimate();
  },
  // 估算值写进 kcal 输入框：用户没碰过它，所以 setData 改 value 能显示出来
  estimate() {
    if (this.data.kcalTouched) return;
    const m = Number(this.data.min);
    this.setData({ kcal: m > 0 ? String(Math.round(m * data.workoutTypes[this.data.typeIdx].perMin)) : '' });
  },
  onKcal(e) { this.setData({ kcal: e.detail.value, kcalTouched: true }); },
  onDate(e) { this.setData({ date: e.detail.value }); },
  onTime(e) { this.setData({ time: e.detail.value }); },
  save() {
    const d = this.data;
    const min = Number(d.min);
    const kcal = Number(d.kcal);
    if (!(min >= 1 && min <= 300) || min % 1) return wx.showToast({ title: '时长请填 1~300 分钟的整数', icon: 'none' });
    if (!(kcal >= 0 && kcal <= 3000)) return wx.showToast({ title: '消耗请填 0~3000 千卡', icon: 'none' });
    if (d.date === d.today && d.time > store.hm(Date.now())) return wx.showToast({ title: '开始时间不能晚于现在', icon: 'none' });
    const t = data.workoutTypes[d.typeIdx];
    store.addWorkout({ date: d.date, time: d.time, name: d.name.trim() || t.name + '训练', type: t.name, min, kcal: Math.round(kcal) });
    wx.showToast({ title: '已记录，' + (d.date === d.today ? '今天' : d.date.slice(5)) + '已打卡', icon: 'none' });
    setTimeout(() => wx.navigateBack(), 500);
  }
});
