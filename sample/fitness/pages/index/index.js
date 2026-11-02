const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: {
    greet: '你好', name: '', storeName: '', streak: 0, goal: 600, pct: 0, rightDeg: 0, leftDeg: 0,
    today: { kcal: 0, min: 0, count: 0 }, weekDays: 0,
    month: 1, weekHead: ['日', '一', '二', '三', '四', '五', '六'], cells: [], checkCount: 0, todayChecked: false,
    plan: null, next: null, recs: []
  },
  onLoad() {
    const h = new Date().getHours();
    this.setData({
      greet: h < 11 ? '早上好' : (h < 14 ? '中午好' : (h < 18 ? '下午好' : '晚上好')),
      recs: [data.courses[0], data.courses[3], data.courses[2]]
    });
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    store.tick();
    const goal = wx.getStorageSync('fit_goal');
    const pv = store.planView();
    const nb = store.upcoming()[0] || null;
    this.setData({
      name: store.getProfile().name,
      storeName: store.currentStore().name,
      today: store.statsOf(store.today()),
      weekDays: store.weekStats().trained,
      plan: pv ? { name: pv.plan.name, day: pv.day, days: pv.plan.days, pct: pv.pct, finished: pv.finished, todayText: pv.finished ? '计划已完成，换一个新计划吧' : (pv.today.rest ? '今天是休息日' : '今日：' + pv.today.name + (pv.today.done ? '（已完成）' : '')) } : null,
      next: nb ? { bid: nb.bid, courseId: nb.courseId, text: (nb.date === store.today() ? '今天' : nb.date.slice(5)) + ' ' + nb.time + ' ' + nb.name, wait: nb.status === 'wait' } : null
    });
    this.setGoalValue(typeof goal === 'number' ? goal : 600);
    this.buildCalendar();
  },
  setGoalValue(goal) {
    const p = Math.min(1, this.data.today.kcal / goal);
    this.setData({
      goal, pct: Math.round(p * 100),
      rightDeg: Math.round(Math.min(p, 0.5) * 360),
      leftDeg: Math.round(Math.max(p - 0.5, 0) * 360)
    });
  },
  buildCalendar() {
    const now = new Date();
    const y = now.getFullYear(), m = now.getMonth(), today = now.getDate();
    const prefix = y + '-' + store.pad(m + 1) + '-';
    const days = store.getCheckins().filter(d => d.indexOf(prefix) === 0).map(d => Number(d.slice(8)));
    const first = new Date(y, m, 1).getDay();
    const total = new Date(y, m + 1, 0).getDate();
    const cells = [];
    for (let i = 0; i < first; i++) cells.push({ k: 'e' + i });
    for (let d = 1; d <= total; d++) cells.push({ k: 'd' + d, day: d, checked: days.indexOf(d) >= 0, today: d === today });
    this.setData({ month: m + 1, cells, checkCount: days.length, todayChecked: days.indexOf(today) >= 0, streak: store.streak() });
  },
  // 打卡：今天还没记训练的话，顺手问一句要不要记
  checkIn() {
    if (this.data.todayChecked) {
      wx.showToast({ title: '今天已经打过卡啦', icon: 'none' });
      return;
    }
    store.checkIn();
    this.buildCalendar();
    if (this.data.today.count) {
      wx.showToast({ title: '打卡成功，已连续 ' + this.data.streak + ' 天', icon: 'none' });
      return;
    }
    wx.showModal({
      title: '打卡成功',
      content: '已连续打卡 ' + this.data.streak + ' 天。要记录今天练了什么吗？',
      confirmText: '去记录',
      cancelText: '不用了',
      success: r => { if (r.confirm) this.goWorkout(); }
    });
  },
  setGoal() {
    const goals = [400, 500, 600, 800];
    wx.showActionSheet({
      itemList: goals.map(g => '每日消耗 ' + g + ' 千卡'),
      success: (r) => {
        const g = goals[r.tapIndex];
        wx.setStorageSync('fit_goal', g);
        this.setGoalValue(g);
        wx.showToast({ title: '每日目标改为 ' + g + ' 千卡', icon: 'none' });
      }
    });
  },
  pickStore() {
    wx.showActionSheet({
      itemList: data.stores.map(s => s.name + '（' + s.dist + '）'),
      success: r => {
        store.setStore(r.tapIndex);
        this.setData({ storeName: data.stores[r.tapIndex].name });
        wx.showToast({ title: '已切换到' + data.stores[r.tapIndex].short, icon: 'none' });
      }
    });
  },
  goClasses() { wx.switchTab({ url: '/pages/classes/classes' }); },
  goCourse(e) { wx.navigateTo({ url: '/pages/course/course?id=' + e.currentTarget.dataset.id + '&day=0' }); },
  goPlan() { wx.navigateTo({ url: '/pages/plan/plan' }); },
  goWorkout() { wx.navigateTo({ url: '/pages/workout-edit/workout-edit' }); },
  goRecords() { wx.switchTab({ url: '/pages/records/records' }); },
  goBookings() { wx.navigateTo({ url: '/pages/bookings/bookings' }); }
});
