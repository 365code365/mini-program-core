const data = require('../../utils/data.js');
const bgaudio = require('../../utils/bgaudio.js');
const store = require('../../utils/store.js');
const theme = require('../../utils/theme.js');

const W = 311, H = 180;
const WEEK = ['日', '一', '二', '三', '四', '五', '六'];

function toMin(s) { const a = s.split(':'); return +a[0] * 60 + +a[1]; }

Page({
  data: {
    theme: 'light', list: [], bed: '23:30', wake: '07:00', quality: 3, qualityOpts: [1, 2, 3, 4, 5], err: '', previewHours: 7.5,
    hasToday: false, formDayText: '昨晚', avgText: '-', avgBed: '-', goodDays: 0, settings: {}, remindDue: false,
    radio: data.RADIO, radioIdx: -1, radioPlaying: false, radioTime: ''
  },

  onLoad() {
    // 背景音频是全局单例：监听只挂一次（tab 页只 onLoad 一次）
    const bam = bgaudio.get();
    bam.onPlay(() => this.setData({ radioPlaying: true }));
    bam.onPause(() => this.setData({ radioPlaying: false }));
    bam.onStop(() => this.setData({ radioPlaying: false }));
    bam.onEnded(() => this.nextRadio());
    bam.onTimeUpdate(() => {
      const t = Math.floor(bam.currentTime || 0);
      const txt = store.pad(Math.floor(t / 60)) + ':' + store.pad(t % 60);
      if (txt !== this.data.radioTime) this.setData({ radioTime: txt });
    });
    bam.onError(() => { this.setData({ radioPlaying: false }); wx.showToast({ title: '电台加载失败', icon: 'none' }); });
    this.bam = bam;
  },

  onShow() {
    theme.apply(this, 2);
    const s = store.settings();
    const now = new Date();
    const nowMin = now.getHours() * 60 + now.getMinutes();
    this.setData({ settings: s, remindDue: s.remind && nowMin >= toMin(s.remindTime), radioPlaying: !!this.bam.src && !this.bam.paused });
    this.refresh();
  },

  applyTheme() { theme.apply(this, 2); this.drawChart(); },

  onReady() { this.drawChart(); },

  refresh() {
    const all = store.sleepList().slice().sort((a, b) => (a.date < b.date ? 1 : -1));
    const list = all.map(r => Object.assign({}, r, { hours: store.sleepHours(r.bed, r.wake) }));
    const today = store.dateStr();
    const week = list.filter(r => r.date >= store.dateStr(store.addDays(new Date(), -6)));
    let avgText = '-', avgBed = '-';
    if (week.length) {
      avgText = (week.reduce((a, r) => a + r.hours, 0) / week.length).toFixed(1) + 'h';
      // 入睡时间跨午夜：18 点前算第二天，先加 24h 再平均
      const bm = week.map(r => { const m = toMin(r.bed); return m < 18 * 60 ? m + 1440 : m; });
      const avg = Math.round(bm.reduce((a, b) => a + b, 0) / bm.length) % 1440;
      avgBed = store.pad(Math.floor(avg / 60)) + ':' + store.pad(avg % 60);
    }
    this.week = week;
    this.setData({
      list, avgText, avgBed, goodDays: week.filter(r => r.hours >= 7).length,
      hasToday: !!list.find(r => r.date === today)
    });
    this.drawChart();
  },

  drawChart() {
    if (!this.week) return;
    const dark = this.data.theme === 'dark';
    const ctx = wx.createCanvasContext('sleepChart', this);
    const left = 28, bottom = H - 24, top = 12, maxH = 10;
    const y = h => bottom - (bottom - top) * Math.min(h, maxH) / maxH;
    ctx.clearRect(0, 0, W, H);
    // 先铺卡片底色：画布默认底不跟随深色主题
    ctx.setFillStyle(dark ? '#23243f' : '#ffffff');
    ctx.fillRect(0, 0, W, H);
    ctx.setFontSize(10);
    ctx.setFillStyle(dark ? '#8f90b3' : '#a5a4ba');
    ctx.setStrokeStyle(dark ? '#2e2f4d' : '#efeef5');
    ctx.setLineWidth(1);
    [0, 4, 8].forEach(h => {
      ctx.beginPath(); ctx.moveTo(left, y(h)); ctx.lineTo(W, y(h)); ctx.stroke();
      ctx.fillText(h + 'h', 2, y(h) + 3);
    });
    // 目标线 8h
    ctx.setStrokeStyle('#f5b54a');
    ctx.beginPath(); ctx.moveTo(left, y(8)); ctx.lineTo(W, y(8)); ctx.stroke();
    const slot = (W - left) / 7, bw = 18;
    const map = {};
    this.week.forEach(r => { map[r.date] = r; });
    for (let i = 0; i < 7; i++) {
      const d = store.addDays(new Date(), i - 6);
      const r = map[store.dateStr(d)];
      const cx = left + slot * i + slot / 2;
      if (r) {
        const top2 = y(r.hours);
        ctx.setFillStyle(r.hours >= 7 ? '#6c63d9' : '#e0904f');
        ctx.fillRect(cx - bw / 2, top2, bw, bottom - top2);
        ctx.setFillStyle(dark ? '#e7e6f5' : '#5d5b78');
        ctx.setTextAlign('center');
        ctx.fillText(String(r.hours), cx, top2 - 4);
      } else {
        ctx.setFillStyle(dark ? '#2a2b47' : '#ecebf3');
        ctx.fillRect(cx - bw / 2, bottom - 4, bw, 4);
      }
      ctx.setFillStyle(dark ? '#8f90b3' : '#a5a4ba');
      ctx.setTextAlign('center');
      ctx.fillText(i === 6 ? '今天' : '周' + WEEK[d.getDay()], cx, H - 6);
      ctx.setTextAlign('left');
    }
    ctx.draw();
  },

  updatePreview(bed, wake) {
    this.setData({ previewHours: store.sleepHours(bed, wake), err: '' });
  },

  onBed(e) { this.setData({ bed: e.detail.value }); this.updatePreview(e.detail.value, this.data.wake); },
  onWake(e) { this.setData({ wake: e.detail.value }); this.updatePreview(this.data.bed, e.detail.value); },
  onQuality(e) { this.setData({ quality: e.currentTarget.dataset.q }); },

  // 记录归到「今天」（昨晚入睡、今早起床）；同一天再存就覆盖
  save() {
    const h = store.sleepHours(this.data.bed, this.data.wake);
    if (h < 1) return this.setData({ err: '睡眠不足 1 小时，检查一下时间' });
    if (h > 16) return this.setData({ err: '超过 16 小时，检查一下时间' });
    const today = store.dateStr();
    const list = store.sleepList().filter(r => r.date !== today);
    list.push({ date: today, bed: this.data.bed, wake: this.data.wake, quality: this.data.quality });
    store.saveSleep(list);
    this.refresh();
    wx.showToast({ title: h >= 7 ? '睡得不错，继续保持' : '已保存，今晚早点睡', icon: 'none' });
  },

  remove(e) {
    const date = e.currentTarget.dataset.date;
    wx.showModal({
      title: '删除 ' + date + ' 的记录？',
      success: res => {
        if (!res.confirm) return;
        store.saveSleep(store.sleepList().filter(r => r.date !== date));
        this.refresh();
      }
    });
  },

  // ───── 睡前电台（BackgroundAudioManager）─────
  onRadio(e) {
    const i = e.currentTarget.dataset.i;
    if (i === this.data.radioIdx && this.bam.src) {
      if (this.bam.paused) this.bam.play(); else this.bam.pause();
      return;
    }
    this.playRadio(i);
  },

  playRadio(i) {
    const r = data.RADIO[i];
    this.bam.title = r.title;
    this.bam.singer = r.singer;
    this.bam.epname = '睡前电台';
    this.bam.coverImgUrl = r.cover;
    this.bam.src = r.src; // 设 src 即开播
    this.setData({ radioIdx: i, radioTime: '00:00' });
  },

  nextRadio() { this.playRadio((this.data.radioIdx + 1) % data.RADIO.length); },

  onRemind(e) {
    const s = store.saveSettings({ remind: e.detail.value });
    this.setData({ settings: s });
    if (s.remind) wx.showToast({ title: '每晚 ' + s.remindTime + ' 提醒你入睡', icon: 'none' });
  },

  onRemindTime(e) {
    const s = store.saveSettings({ remindTime: e.detail.value });
    const now = new Date();
    this.setData({ settings: s, remindDue: now.getHours() * 60 + now.getMinutes() >= toMin(s.remindTime) });
  }
});
