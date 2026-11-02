// 我的（tab）：每日计划量（slider）、提醒时间（picker time）+ 订阅消息、自动发音、各入口、重置
const store = require('../../utils/store.js');
const books = require('../../utils/books.js');
const tab = require('../../utils/tab.js');
// 订阅消息模板 ID（公众平台申请的「学习提醒」模板）
const TMPL_ID = 'Yb3pLq7mN2vXcRk9sTqA1dHfG8wZ0eJuOiP4lK6nMbE';

Page({
  data: { s: {}, bookName: '', learned: 0, mastered: 0, days: 0, streak: 0, wrongs: 0, planTip: '' },
  onShow() {
    tab.sync(this, 3);
    this.load();
  },
  load() {
    const s = store.settings(), p = store.progress();
    const ws = Object.keys(p);
    this.setData({
      s, bookName: books.book(s.book).name, learned: ws.length,
      mastered: ws.filter((w) => !p[w].due).length, days: store.checkins().length,
      streak: store.streak(), wrongs: store.wrongs().length, planTip: this.tip(s.plan)
    });
  },
  tip(n) {
    const left = store.newWords(store.settings().book, 999).length;
    const d = Math.ceil(left / n);
    return left ? '按 ' + n + ' 个 / 天，约 ' + d + ' 天学完本书剩下的 ' + left + ' 词' : '本书新词已全部学完';
  },
  onPlanChanging(e) { this.setData({ planTip: this.tip(e.detail.value) }); },
  onPlan(e) {
    const s = store.saveSettings({ plan: e.detail.value });
    this.setData({ s, planTip: this.tip(s.plan) });
    wx.showToast({ title: '每日 ' + s.plan + ' 个新词', icon: 'none' });
  },
  onTime(e) {
    const s = store.saveSettings({ remind: e.detail.value });
    this.setData({ s });
    if (s.remindOn) wx.showToast({ title: '每天 ' + s.remind + ' 提醒', icon: 'none' });
  },
  onRemind(e) {
    const on = e.detail.value;
    if (!on) { this.setData({ s: store.saveSettings({ remindOn: false }) }); return; }
    // 订阅消息要用户在微信弹窗里同意；拿不到（拒绝 / 宿主没有）时退回小程序内提醒
    wx.requestSubscribeMessage({
      tmplIds: [TMPL_ID],
      success: (res) => {
        const ok = res[TMPL_ID] === 'accept';
        this.setData({ s: store.saveSettings({ remindOn: true, subscribed: ok }) });
        wx.showToast({ title: ok ? '已订阅学习提醒' : '已开启应用内提醒', icon: 'none' });
      },
      fail: () => {
        this.setData({ s: store.saveSettings({ remindOn: true, subscribed: false }) });
        wx.showModal({ title: '已开启应用内提醒', content: '当前环境不能订阅微信消息，过了 ' + this.data.s.remind + ' 还没打卡时，打开词力会在首页提醒你。', showCancel: false });
      }
    });
  },
  onAuto(e) { this.setData({ s: store.saveSettings({ autoPlay: e.detail.value }) }); },
  goBooks() { wx.navigateTo({ url: '/pages/books/books' }); },
  goCalendar() { wx.navigateTo({ url: '/pages/calendar/calendar' }); },
  goStats() { wx.navigateTo({ url: '/pages/stats/stats' }); },
  goWrong() { wx.navigateTo({ url: '/pages/wrong/wrong' }); },
  onReset() {
    wx.showModal({
      title: '重置全部学习记录？', content: '复习进度、打卡、生词本、错题本都会清空，且不能恢复。', confirmText: '重置', confirmColor: '#f0566a',
      success: (r) => {
        if (!r.confirm) return;
        store.resetAll();
        store.saveSettings({});
        this.load();
        tab.sync(this, 3);
        wx.showToast({ title: '已重置', icon: 'success' });
      }
    });
  }
});
