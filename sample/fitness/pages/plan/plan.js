const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
// 训练计划：没有进行中的计划时选一个开始；有就看今天练什么、完成打卡、看每天进度、放弃
Page({
  data: { plans: [], view: null },
  onShow() { this.load(); },
  load() {
    const view = store.planView();
    this.setData({
      view,
      plans: data.plans.map(p => Object.assign({}, p, { trainDays: Math.round(p.week.filter(x => !x.rest).length * p.days / 7), perDay: Math.max.apply(null, p.week.map(x => x.min || 0)) }))
    });
    wx.setNavigationBarTitle({ title: view ? view.plan.name : '训练计划' });
  },
  start(e) {
    const p = data.findPlan(e.currentTarget.dataset.id);
    const run = () => {
      store.startPlan(p.id);
      wx.showToast({ title: '计划开始，今天是第 1 天', icon: 'none' });
      this.load();
    };
    if (!this.data.view || this.data.view.finished) return run();
    wx.showModal({
      title: '换一个计划',
      content: '当前的「' + this.data.view.plan.name + '」进度会清空，确定换成「' + p.name + '」吗？',
      confirmText: '确定更换',
      success: r => { if (r.confirm) run(); }
    });
  },
  finishToday() {
    const v = this.data.view;
    if (v.today.rest) return wx.showToast({ title: '今天是休息日，好好放松', icon: 'none' });
    if (v.today.done) return wx.showToast({ title: '今天的训练已经完成啦', icon: 'none' });
    wx.showModal({
      title: '完成今日训练',
      content: v.today.name + ' · ' + v.today.min + ' 分钟 · 约 ' + v.today.kcal + ' 千卡\n完成后记入训练记录并打卡',
      confirmText: '已完成',
      success: r => {
        if (!r.confirm) return;
        store.completePlanDay();
        this.load();
        wx.showToast({ title: '第 ' + v.day + ' 天完成，已打卡', icon: 'none' });
      }
    });
  },
  quit() {
    wx.showModal({
      title: '放弃计划',
      content: '放弃后进度清空（已记下的训练记录保留），确定吗？',
      confirmText: '放弃',
      confirmColor: '#ff3d6e',
      success: r => {
        if (!r.confirm) return;
        store.quitPlan();
        this.load();
      }
    });
  },
  showOthers() { this.setData({ 'view.showOthers': !this.data.view.showOthers }); }
});
