const store = require('../../utils/store.js');
const WEEK = ['日', '一', '二', '三', '四', '五', '六'];
Page({
  data: { week: WEEK, days: [], streak: 0, total: 0, monthCount: 0, todayDone: false, title: '', y: 0, m: 0, isCurMonth: true,
    tasks: [
      { key: 'learn', name: '学习 1 节课程', sub: '任意课程学完一节', pts: 10, done: true },
      { key: 'exam', name: '完成一次章节练习', sub: '5 道单选题', pts: 20, done: false },
      { key: 'share', name: '分享学习成果', sub: '邀请好友一起学', pts: 5, done: false }
    ]
  },
  onLoad() { const t = new Date(); this.setData({ y: t.getFullYear(), m: t.getMonth() }); },
  onShow() {
    const tasks = this.data.tasks.slice();
    tasks[1] = Object.assign({}, tasks[1], { done: !!wx.getStorageSync('zx_exam_today') });
    this.setData({ tasks });
    this.refresh();
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
    wx.showModal({ title: '打卡成功', content: '已连续学习 ' + this.data.streak + ' 天，获得 10 积分。连续 7 天可领取「学霸勋章」！', showCancel: false, confirmText: '太棒了' });
  },
  onTask(e) {
    const k = e.currentTarget.dataset.k;
    if (k === 'exam') wx.navigateTo({ url: '/pages/exam/exam' });
    else if (k === 'learn') wx.switchTab({ url: '/pages/learn/learn' });
    else wx.showToast({ title: '已生成分享海报', icon: 'none' });
  }
});
