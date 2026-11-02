// 用药提醒：今日待服（按今天星期几筛出的每一次服药，点「打卡」记为已服）+ 全部提醒（开关、编辑）。
// 打卡记录按日期存（ph_taken: {日期: [提醒id@时刻]}），用 wx.getStorage / wx.setStorage 的 Promise 写法
const S = require('../../utils/store.js');

const WK = ['', '周一', '周二', '周三', '周四', '周五', '周六', '周日'];
function daysText(days) {
  if (days.length === 7) return '每天';
  if (days.join() === '1,2,3,4,5') return '工作日';
  if (days.join() === '6,7') return '周末';
  return days.map(d => WK[d]).join('、');
}

Page({
  data: { today: '', doses: [], done: 0, list: [] },
  onShow() { this.load(); },
  load() {
    const d = new Date();
    const list = S.get('reminders', []).map(r => Object.assign({}, r, { daysText: daysText(r.days), timesText: r.times.join('  ') }));
    const doses = S.todayDoses();
    const now = ('0' + d.getHours()).slice(-2) + ':' + ('0' + d.getMinutes()).slice(-2);
    doses.forEach(x => { x.late = !x.taken && x.time < now; });
    this.setData({ today: (d.getMonth() + 1) + ' 月 ' + d.getDate() + ' 日 ' + WK[S.weekNo(d)], doses, done: doses.filter(x => x.taken).length, list });
  },
  take(e) {
    const k = e.currentTarget.dataset.k, day = S.todayKey();
    wx.getStorage({ key: S.K.taken })
      .then(r => r.data || {}, () => ({}))
      .then(all => {
        const keep = {}; // 只留最近 7 天的打卡记录
        Object.keys(all).slice(-6).forEach(x => { keep[x] = all[x]; });
        const arr = keep[day] || [];
        const i = arr.indexOf(k);
        if (i >= 0) arr.splice(i, 1); else arr.push(k);
        keep[day] = arr;
        return wx.setStorage({ key: S.K.taken, data: keep }).then(() => i < 0);
      })
      .then(on => { this.load(); wx.showToast({ title: on ? '已打卡，记得多喝水' : '已撤销打卡', icon: 'none' }); })
      .catch(err => wx.showToast({ title: '保存失败：' + (err.errMsg || ''), icon: 'none' }));
  },
  onSwitch(e) {
    const id = e.currentTarget.dataset.id;
    const all = S.get('reminders', []);
    all.find(r => r.id === id).on = e.detail.value;
    S.set('reminders', all);
    this.load();
  },
  add() { wx.navigateTo({ url: '/pages/remind-edit/remind-edit' }); },
  edit(e) { wx.navigateTo({ url: '/pages/remind-edit/remind-edit?id=' + e.currentTarget.dataset.id }); }
});
