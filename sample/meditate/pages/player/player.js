const data = require('../../utils/data.js');
const bgaudio = require('../../utils/bgaudio.js');
const store = require('../../utils/store.js');
const theme = require('../../utils/theme.js');

const BG = {
  intro: 'linear-gradient(180deg, #7f7fd5, #3b3a7a)', sleep: 'linear-gradient(180deg, #2b3270, #12142e)',
  relax: 'linear-gradient(180deg, #3fa7c9, #23407a)', focus: 'linear-gradient(180deg, #6b5fd0, #2e2766)',
  emotion: 'linear-gradient(180deg, #d58aa8, #5a4a96)', body: 'linear-gradient(180deg, #43b39b, #24527d)'
};
const SPIN_PERIOD = 24; // 秒，与 wxss 的 .spin 一致
const TIMERS = [15, 30, 60];

function mmss(s) {
  s = Math.max(0, Math.floor(s));
  return store.pad(Math.floor(s / 60)) + ':' + store.pad(s % 60);
}

Page({
  data: {
    theme: 'light', c: null, idx: 0, chapter: {}, bg: '', playing: false, cur: 0, durInt: 25, curText: '00:00', durText: '00:25',
    loop: false, showNoise: false, noiseOnN: 0, timerText: '', spinStyle: '',
    noises: data.NOISES.map(n => ({ key: n.key, name: n.name, color: n.color, on: false, vol: 60 }))
  },

  onLoad(q) {
    const c = data.course(q.id) || data.COURSES[0];
    let idx = Math.min(Math.max(+q.idx || 0, 0), c.chapters.length - 1);
    if (!store.unlocked(c.id, idx)) idx = 0;
    this.c = c;
    this.spinAcc = 0;
    this.spinStart = 0;
    this.dragging = false;
    this.setData({ c, idx, chapter: c.chapters[idx], bg: BG[c.cat] || BG.intro });
    wx.setNavigationBarTitle({ title: c.chapters[idx].title });
    store.setLast(c.id, idx);
    // 背景电台在放就先停，避免两路人声叠在一起
    const bam = bgaudio.get();
    if (bam.src && !bam.paused) bam.pause();
    this.initMain(c.chapters[idx].audio);
    this.initNoises();
    // 进入时的「白噪音」入口：打开混音面板并开雨声
    if (q.noise === '1') { this.setData({ showNoise: true }); this.setNoise(0, true); }
    else this.main.play();
    // 切后台：设置里关了「后台继续播白噪音」就先停混音；回前台按音频时钟重新对一次进度
    this.hideCb = () => {
      if (store.settings().noiseOnHide) return;
      this.data.noises.forEach((n, i) => { if (n.on) this.setNoise(i, false); });
    };
    this.showCb = () => this.syncTime();
    wx.onAppHide(this.hideCb);
    wx.onAppShow(this.showCb);
  },

  onShow() {
    theme.apply(this, -1);
    wx.setNavigationBarColor({ frontColor: '#ffffff', backgroundColor: '#3b3a7a', animation: { duration: 0 } });
  },

  applyTheme() { this.onShow(); },

  // ───── 主音频：一节课 ─────
  initMain(src) {
    const a = wx.createInnerAudioContext();
    a.src = src;
    a.loop = false;
    a.volume = 1;
    a.onCanplay(() => this.syncTime());
    a.onPlay(() => this.setPlaying(true));
    a.onPause(() => this.setPlaying(false));
    a.onStop(() => this.setPlaying(false));
    a.onTimeUpdate(() => { if (!this.dragging) this.syncTime(); });
    a.onEnded(() => this.onMainEnded());
    a.onError(err => {
      this.setPlaying(false);
      wx.showToast({ title: '音频加载失败（' + (err.errCode || '') + '）', icon: 'none' });
    });
    this.main = a;
  },

  syncTime() {
    if (!this.main) return;
    const d = this.main.duration || 25;
    const t = this.main.currentTime || 0;
    this.setData({ cur: Math.floor(t), durInt: Math.max(1, Math.round(d)), curText: mmss(t), durText: mmss(d) });
  },

  setPlaying(on) {
    if (on === this.data.playing) return;
    const now = Date.now();
    if (on) {
      this.spinStart = now;
      // 从上次停下的角度接着转：负的 animation-delay 把动画时间轴拨到已转过的位置
      this.setData({ playing: true, spinStyle: 'animation-play-state: running; animation-delay: -' + (this.spinAcc % SPIN_PERIOD).toFixed(2) + 's;' });
    } else {
      if (this.spinStart) this.spinAcc += (now - this.spinStart) / 1000;
      this.spinStart = 0;
      const deg = (this.spinAcc % SPIN_PERIOD) / SPIN_PERIOD * 360;
      this.setData({ playing: false, spinStyle: 'animation-play-state: paused; transform: rotate(' + deg.toFixed(1) + 'deg);' });
    }
    wx.setKeepScreenOn({ keepScreenOn: on || this.data.noiseOnN > 0 });
  },

  toggle() {
    if (this.data.playing) this.main.pause();
    else this.main.play();
  },

  back15() {
    const t = Math.max(0, (this.main.currentTime || 0) - 15);
    this.main.seek(t);
    this.setData({ cur: Math.floor(t), curText: mmss(t) });
  },

  onSeeking(e) {
    this.dragging = true;
    this.setData({ curText: mmss(e.detail.value) });
  },

  onSeek(e) {
    this.dragging = false;
    const v = e.detail.value;
    this.main.seek(v);
    this.setData({ cur: v, curText: mmss(v) });
  },

  toggleLoop() {
    const loop = !this.data.loop;
    this.setData({ loop });
    wx.showToast({ title: loop ? '单节循环已开启' : '已关闭循环', icon: 'none' });
  },

  onMainEnded() {
    if (this.data.loop) {
      this.main.seek(0);
      this.main.play();
      return;
    }
    if (this.stopAfterChapter) this.clearTimer();
    this.finish(this.main.duration || 25);
  },

  // 一节听完：记练习、解锁下一节、跳完成页（redirectTo：完成页返回直接回课程详情）
  finish(sec) {
    if (this.finished) return;
    this.finished = true;
    const c = this.c, idx = this.data.idx;
    store.addSession(sec, 'course', c.title + ' · ' + c.chapters[idx].title);
    store.markDone(c.id, idx);
    if (idx + 1 < c.chapters.length) store.setLast(c.id, idx + 1);
    wx.redirectTo({ url: '/pages/done/done?type=course&id=' + c.id + '&idx=' + idx + '&sec=' + Math.round(sec) });
  },

  // ───── 白噪音：三路 InnerAudioContext 同时循环播 ─────
  initNoises() {
    this.noiseCtx = data.NOISES.map((n, i) => {
      const a = wx.createInnerAudioContext();
      a.src = n.src;
      a.loop = true;
      a.volume = this.data.noises[i].vol / 100;
      a.onError(() => wx.showToast({ title: n.name + '加载失败', icon: 'none' }));
      return a;
    });
  },

  toggleNoisePanel() { this.setData({ showNoise: !this.data.showNoise }); },

  toggleNoise(e) {
    const i = e.currentTarget.dataset.i;
    this.setNoise(i, !this.data.noises[i].on);
  },

  setNoise(i, on) {
    const a = this.noiseCtx[i];
    if (on) { a.seek(0); a.play(); } else a.pause();
    const key = 'noises[' + i + '].on';
    const n = this.data.noises.filter((x, j) => (j === i ? on : x.on)).length;
    this.setData({ [key]: on, noiseOnN: n });
    wx.setKeepScreenOn({ keepScreenOn: n > 0 || this.data.playing });
  },

  // 拖动中（bindchanging）就实时改音量，松手（bindchange）再定一次
  onVol(e) {
    const i = e.currentTarget.dataset.i;
    const v = e.detail.value;
    this.noiseCtx[i].volume = v / 100;
    this.setData({ ['noises[' + i + '].vol']: v });
  },

  // ───── 定时关闭 ─────
  pickTimer() {
    wx.showActionSheet({
      itemList: ['15 分钟后停止', '30 分钟后停止', '60 分钟后停止', '播完本节停止', '关闭定时'],
      success: res => {
        const i = res.tapIndex;
        if (i < 3) this.startTimer(TIMERS[i] * 60);
        else if (i === 3) { this.clearTimer(); this.stopAfterChapter = true; this.setData({ timerText: '本节' }); }
        else { this.clearTimer(); wx.showToast({ title: '已关闭定时', icon: 'none' }); }
      }
    });
  },

  startTimer(sec) {
    this.clearTimer();
    this.timerEnd = Date.now() + sec * 1000;
    this.setData({ timerText: mmss(sec) });
    this.timer = setTimeout(() => this.onTimerFire(), sec * 1000);
    this.tick = setInterval(() => this.setData({ timerText: mmss((this.timerEnd - Date.now()) / 1000) }), 1000);
  },

  clearTimer() {
    clearTimeout(this.timer);
    clearInterval(this.tick);
    this.timer = this.tick = null;
    this.stopAfterChapter = false;
    if (this.data.timerText) this.setData({ timerText: '' });
  },

  onTimerFire() {
    this.clearTimer();
    this.main.pause();
    this.data.noises.forEach((n, i) => { if (n.on) this.setNoise(i, false); });
    wx.showToast({ title: '定时已到，已为你停止播放', icon: 'none' });
  },

  onUnload() {
    this.clearTimer();
    wx.offAppHide && wx.offAppHide(this.hideCb);
    wx.offAppShow && wx.offAppShow(this.showCb);
    wx.setKeepScreenOn({ keepScreenOn: false });
    if (this.main) this.main.destroy();
    (this.noiseCtx || []).forEach(a => a.destroy());
  }
});
