const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

Page({
  data: {
    greet: '你好', streak: 0, goal: 600, pct: 0, rightDeg: 0, leftDeg: 0,
    today: { kcal: 386, min: 42, steps: 8216 },
    month: 1, weekHead: ['日', '一', '二', '三', '四', '五', '六'], cells: [], checkCount: 0, todayChecked: false,
    recs: []
  },
  onLoad() {
    const h = new Date().getHours();
    this.setData({
      greet: h < 11 ? '早上好' : (h < 14 ? '中午好' : (h < 18 ? '下午好' : '晚上好')),
      recs: [data.courses[0], data.courses[3], data.courses[2]]
    });
  },
  onShow() {
    this.setGoalValue(wx.getStorageSync('fit_goal') || 600);
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
    let checks = store.getChecks();
    if (!checks) {
      // 首次打开：给本月今天之前的日子铺一批历史打卡（演示数据）
      checks = [];
      for (let d = 1; d < today; d++) if ((d * 7) % 5 !== 0) checks.push(d);
      store.setChecks(checks);
    }
    const first = new Date(y, m, 1).getDay();
    const total = new Date(y, m + 1, 0).getDate();
    const cells = [];
    for (let i = 0; i < first; i++) cells.push({ k: 'e' + i });
    for (let d = 1; d <= total; d++) cells.push({ k: 'd' + d, day: d, checked: checks.indexOf(d) >= 0, today: d === today });
    let streak = 0;
    for (let d = today - 1; d >= 1 && checks.indexOf(d) >= 0; d--) streak++;
    if (checks.indexOf(today) >= 0) streak++;
    this.setData({ month: m + 1, cells, checkCount: checks.length, todayChecked: checks.indexOf(today) >= 0, streak: streak + 36 });
  },
  checkIn() {
    if (this.data.todayChecked) {
      wx.showToast({ title: '今天已经打过卡啦', icon: 'none' });
      return;
    }
    const checks = store.getChecks() || [];
    checks.push(new Date().getDate());
    store.setChecks(checks);
    this.buildCalendar();
    wx.showToast({ title: '打卡成功', icon: 'success' });
  },
  setGoal() {
    const goals = [400, 500, 600, 800];
    wx.showActionSheet({
      itemList: goals.map(g => '每日消耗 ' + g + ' 千卡'),
      success: (r) => {
        const g = goals[r.tapIndex];
        wx.setStorageSync('fit_goal', g);
        this.setGoalValue(g);
      }
    });
  },
  pickStore() {
    wx.showActionSheet({
      itemList: ['燃动 · 科技园店（0.8km）', '燃动 · 海岸城店（2.3km）', '燃动 · 后海汇店（3.6km）'],
      success: () => wx.showToast({ title: '已切换门店', icon: 'none' })
    });
  },
  goClasses() { wx.switchTab({ url: '/pages/classes/classes' }); },
  goCourse(e) { wx.navigateTo({ url: '/pages/course/course?id=' + e.currentTarget.dataset.id + '&day=0' }); }
});
