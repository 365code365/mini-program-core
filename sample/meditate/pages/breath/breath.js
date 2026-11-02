const store = require('../../utils/store.js');
const theme = require('../../utils/theme.js');

// 4-7-8 呼吸：吸 4 秒（圆放大）→ 屏 7 秒（保持）→ 呼 8 秒（缩回）。圆的缩放用 this.animate 按关键帧插值
const PHASES = [
  { key: 'in', text: '吸气', sec: 4 },
  { key: 'hold', text: '屏息', sec: 7 },
  { key: 'out', text: '呼气', sec: 8 }
];
const BIG = 1.75;

Page({
  data: { theme: 'light', running: false, phase: '', phaseText: '准备', countText: '点击下方开始', rounds: 4, roundOpts: [3, 4, 6, 8], totalText: '', vibrate: true },

  onLoad() { this.updateTotal(4); },
  onShow() { theme.apply(this, -1); },
  applyTheme() { theme.apply(this, -1); },

  updateTotal(n) {
    const s = n * 19;
    this.setData({ totalText: Math.floor(s / 60) + ' 分 ' + (s % 60) + ' 秒' });
  },

  setRounds(e) {
    if (this.data.running) return wx.showToast({ title: '练习中不能修改轮数', icon: 'none' });
    const n = e.currentTarget.dataset.n;
    this.setData({ rounds: n });
    this.updateTotal(n);
  },

  onVibrate(e) { this.setData({ vibrate: e.detail.value }); },

  toggle() {
    if (this.data.running) this.stop(true);
    else this.start();
  },

  start() {
    this.round = 0;
    this.startAt = Date.now();
    this.setData({ running: true });
    wx.setKeepScreenOn({ keepScreenOn: true });
    this.runPhase(0);
  },

  runPhase(i) {
    if (!this.data.running) return;
    const p = PHASES[i];
    this.phaseIdx = i;
    this.left = p.sec;
    this.setData({ phase: p.key, phaseText: p.text, countText: p.sec + ' 秒 · 第 ' + (this.round + 1) + '/' + this.data.rounds + ' 轮' });
    if (this.data.vibrate) wx.vibrateShort({ type: 'light', fail() {} });
    if (p.key === 'in') this.animate('#ball', [{ scale: [1, 1] }, { scale: [BIG, BIG], ease: 'ease-out' }], p.sec * 1000);
    else if (p.key === 'out') this.animate('#ball', [{ scale: [BIG, BIG] }, { scale: [1, 1], ease: 'ease-in-out' }], p.sec * 1000);
    clearInterval(this.tick);
    this.tick = setInterval(() => {
      this.left--;
      if (this.left > 0) {
        this.setData({ countText: this.left + ' 秒 · 第 ' + (this.round + 1) + '/' + this.data.rounds + ' 轮' });
        return;
      }
      clearInterval(this.tick);
      if (i < 2) return this.runPhase(i + 1);
      this.round++;
      if (this.round >= this.data.rounds) return this.complete();
      this.runPhase(0);
    }, 1000);
  },

  complete() {
    const sec = (Date.now() - this.startAt) / 1000;
    this.stop(false);
    store.addSession(sec, 'breath', '4-7-8 呼吸 · ' + this.data.rounds + ' 轮');
    wx.redirectTo({ url: '/pages/done/done?type=breath&rounds=' + this.data.rounds + '&sec=' + Math.round(sec) });
  },

  // 中途结束：练了一轮以上也记一次
  stop(byUser) {
    clearInterval(this.tick);
    this.clearAnimation('#ball');
    wx.setKeepScreenOn({ keepScreenOn: false });
    this.setData({ running: false, phase: '', phaseText: '准备', countText: '点击下方开始' });
    if (byUser && this.round >= 1) {
      const sec = (Date.now() - this.startAt) / 1000;
      store.addSession(sec, 'breath', '4-7-8 呼吸 · ' + this.round + ' 轮');
      wx.showToast({ title: '已记录 ' + this.round + ' 轮练习', icon: 'none' });
    } else if (byUser) {
      wx.showToast({ title: '完成一整轮才会记录哦', icon: 'none' });
    }
  },

  onUnload() {
    clearInterval(this.tick);
    wx.setKeepScreenOn({ keepScreenOn: false });
  }
});
