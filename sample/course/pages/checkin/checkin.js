const store = require('../../utils/store.js');
const WEEK = ['日', '一', '二', '三', '四', '五', '六'];
Page({
  data: { week: WEEK, days: [], streak: 0, total: 0, monthCount: 0, todayDone: false, title: '', y: 0, m: 0, isCurMonth: true,
    points: 0,
    tasks: [
      { key: 'learn', name: '学习 1 节课程', sub: '任意课程学完一节', pts: 10, done: false, claimed: false },
      { key: 'exam', name: '完成一次章节练习', sub: '5 道单选题', pts: 20, done: false, claimed: false },
      { key: 'note', name: '记一条学习笔记', sub: '在课时页写下要点', pts: 5, done: false, claimed: false }
    ]
  },
  onLoad() { const t = new Date(); this.setData({ y: t.getFullYear(), m: t.getMonth() }); },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    this.syncTasks();
    this.refresh();
  },
  // 任务完成与否按今天的记录判断；完成后点「领取」才加积分（zx_task_claim: { date, keys }）
  syncTasks() {
    const t = store.today();
    const done = {
      learn: wx.getStorageSync('zx_learn_day') === t,
      exam: wx.getStorageSync('zx_exam_today') === t,
      note: store.getNotes().some(n => n.day === t)
    };
    const cl = wx.getStorageSync('zx_task_claim');
    const keys = cl && cl.date === t ? cl.keys : [];
    this.setData({ tasks: this.data.tasks.map(x => Object.assign({}, x, { done: done[x.key], claimed: keys.indexOf(x.key) >= 0 })), points: store.getPoints() });
  },
  refresh() {
    const list = store.getCheckins();
    const set = {};
    list.forEach(k => { set[k] = true; });
    const { y, m } = this.data;
    const today = store.dayKey(new Date());
    const first = new Date(y, m, 1).getDay();
    const n = new Date(y, m + 1, 0).getDate();
    const days = [];
    for (let i = 0; i < first; i++) days.push({ k: 'e' + i, d: '', empty: true });
    let monthCount = 0;
    for (let d = 1; d <= n; d++) {
      const key = store.dayKey(new Date(y, m, d));
      const on = !!set[key];
      if (on) monthCount++;
      days.push({ k: key, d, on, today: key === today, future: key > today });
    }
    const t = new Date();
    this.setData({ days, monthCount, streak: store.streak(list), total: list.length, todayDone: !!set[today],
      title: y + ' 年 ' + (m + 1) + ' 月', isCurMonth: y === t.getFullYear() && m === t.getMonth() });
  },
  prev() { const d = new Date(this.data.y, this.data.m - 1, 1); this.setData({ y: d.getFullYear(), m: d.getMonth() }); this.refresh(); },
  next() {
    if (this.data.isCurMonth) return;
    const d = new Date(this.data.y, this.data.m + 1, 1); this.setData({ y: d.getFullYear(), m: d.getMonth() }); this.refresh();
  },
  checkin() {
    if (this.data.todayDone) { wx.showToast({ title: '今天已经打过卡了', icon: 'none' }); return; }
    const list = store.getCheckins();
    list.push(store.dayKey(new Date()));
    store.saveCheckins(list);
    const t = new Date();
    this.setData({ y: t.getFullYear(), m: t.getMonth() });
    this.refresh();
    store.addPoints(10);
    this.setData({ points: store.getPoints() });
    wx.showModal({ title: '打卡成功', content: '已连续学习 ' + this.data.streak + ' 天，获得 10 积分（现有 ' + this.data.points + ' 积分）。积分可在「优惠券」里兑换无门槛券。', showCancel: false, confirmText: '太棒了' });
  },
  onTask(e) {
    const k = String(e.currentTarget.dataset.k);
    const task = this.data.tasks.find(x => x.key === k);
    if (task.done && !task.claimed) {
      const t = store.today();
      const cl = wx.getStorageSync('zx_task_claim');
      const keys = cl && cl.date === t ? cl.keys : [];
      wx.setStorageSync('zx_task_claim', { date: t, keys: keys.concat([k]) });
      store.addPoints(task.pts);
      this.syncTasks();
      wx.showToast({ title: '积分 +' + task.pts, icon: 'none' });
      return;
    }
    if (task.claimed) { wx.showToast({ title: '今天的这个任务已经领过了', icon: 'none' }); return; }
    if (k === 'exam') wx.navigateTo({ url: '/pages/exam/exam' });
    else if (k === 'learn') wx.switchTab({ url: '/pages/learn/learn' });
    else {
      const cur = store.myCourses().find(x => !x.finished);
      if (cur) wx.navigateTo({ url: '/pages/lesson/lesson?cid=' + cur.id + '&lid=' + cur.last });
      else wx.switchTab({ url: '/pages/learn/learn' });
    }
  },
  goPoints() { wx.navigateTo({ url: '/pages/coupons/coupons' }); }
});
