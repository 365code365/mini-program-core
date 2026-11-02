// 发起组局：项目 → 场馆 → 日期 / 开始时间 / 时长 → 人数（slider）→ 水平 → AA 费（默认按场地价自动算）→ 说明。
// 从订单进来（?order=）时场馆、日期、时段锁定成已订好的那块场地。
const D = require('../../utils/data.js');
const F = require('../../utils/fmt.js');
const S = require('../../utils/slots.js');
const store = require('../../utils/store.js');
const T = require('../../utils/teams.js');
const SPORTS = D.SPORTS.filter((s) => s.id !== 'swim' && s.id !== 'yoga');
const MAXP = { badminton: 8, basketball: 12, tennis: 4, football: 14, pingpong: 4, pickle: 4 };
const DURS = [1, 2, 3];
Page({
  data: {
    sports: SPORTS, si: 0, venues: [], vi: 0, days: [], di: 0, hours: [], hi: 0, durs: DURS.map((h) => h + ' 小时'), du: 1,
    need: 4, maxNeed: 8, levels: T.LEVELS, level: 0, fee: '', feeAuto: true, note: '', locked: false, lockText: '', err: ''
  },
  onLoad(q) {
    const days = [];
    for (let i = 0; i < 7; i++) {
      const k = T.day(i);
      days.push({ key: k, label: (i === 0 ? '今天 ' : i === 1 ? '明天 ' : '') + F.dateText(k) });
    }
    this.setData({ days });
    const o = q.order && store.getOrder(q.order);
    if (o) {
      const seg = o.segments[0];
      const si = Math.max(0, SPORTS.findIndex((s) => s.id === o.sport));
      this.setData({ si, locked: true, lockText: seg.courtLabel + ' ' + seg.text + '（已订好场地）' });
      this.order = { venueId: o.venueId, date: o.date, start: seg.start, end: seg.end, amount: seg.amount };
    }
    this.onSportChanged();
  },
  onSportChanged() {
    const sp = SPORTS[this.data.si];
    const venues = D.VENUES.filter((v) => v.sport === sp.id);
    const maxNeed = MAXP[sp.id] || 8;
    this.setData({ venues, vi: 0, maxNeed, need: Math.min(Math.max(this.data.need, 2), maxNeed) });
    this.onSlotChanged();
  },
  // 可选开始时间：场馆营业时段，今天的要晚于现在
  onSlotChanged() {
    const v = this.data.venues[this.data.vi];
    const k = this.data.days[this.data.di].key;
    const now = new Date();
    const hrs = S.hours(v).filter((h) => k !== F.dateKey(now) || h > now.getHours());
    this.setData({ hours: hrs.map((h) => ({ h, label: F.hh(h) })), hi: Math.min(this.data.hi, Math.max(0, hrs.length - 1)) });
    this.autoFee();
  },
  autoFee() {
    if (!this.data.feeAuto) return;
    const fee = this.suggest();
    this.setData({ fee: fee ? String(fee) : '' });
  },
  // 建议 AA 费 = 场地费 / 人数，向上取整
  suggest() {
    if (this.order) return Math.ceil(this.order.amount / this.data.need);
    const v = this.data.venues[this.data.vi], k = this.data.days[this.data.di].key, h0 = this.data.hours[this.data.hi];
    if (!h0) return 0;
    let sum = 0;
    for (let h = h0.h; h < h0.h + DURS[this.data.du]; h++) sum += S.priceAt(v, k, h);
    return Math.ceil(sum / this.data.need);
  },
  onSport(e) { this.setData({ si: Number(e.detail.value) }); this.onSportChanged(); },
  onVenue(e) { this.setData({ vi: Number(e.detail.value) }); this.onSlotChanged(); },
  onDay(e) { this.setData({ di: Number(e.detail.value) }); this.onSlotChanged(); },
  onHour(e) { this.setData({ hi: Number(e.detail.value), err: '' }); this.autoFee(); },
  onDur(e) { this.setData({ du: Number(e.detail.value), err: '' }); this.autoFee(); },
  onNeed(e) { this.setData({ need: e.detail.value }); this.autoFee(); },
  onLevel(e) { this.setData({ level: Number(e.detail.value) }); },
  onFee(e) { this.setData({ fee: e.detail.value, feeAuto: false, err: '' }); },
  resetFee() { this.setData({ feeAuto: true }); this.autoFee(); },
  onNote(e) { this.setData({ note: e.detail.value }); },
  submit() {
    let t;
    if (this.order) {
      t = { sport: SPORTS[this.data.si].id, venueId: this.order.venueId, date: this.order.date, start: this.order.start, end: this.order.end, booked: true };
    } else {
      const v = this.data.venues[this.data.vi], h0 = this.data.hours[this.data.hi];
      if (!h0) { this.setData({ err: '今天已经没有可选的开始时间，换一天吧' }); return; }
      const end = h0.h + DURS[this.data.du];
      const close = Number(v.open.split('-')[1].split(':')[0]);
      if (end > close) { this.setData({ err: '场馆 ' + F.hh(close) + ' 关门，请缩短时长或提前开始' }); return; }
      t = { sport: v.sport, venueId: v.id, date: this.data.days[this.data.di].key, start: h0.h, end, booked: false };
    }
    const fee = Number(this.data.fee);
    if (!/^\d{1,3}$/.test(this.data.fee) || fee > 500) { this.setData({ err: 'AA 费请填 0~500 的整数' }); return; }
    const note = this.data.note.trim() || (t.booked ? '场地已订好，人齐就开打' : '场地到时一起订，人齐就开打');
    T.create(Object.assign(t, { need: this.data.need, fee, level: this.data.level, note }));
    wx.showToast({ title: '组局已发布', icon: 'success' });
    setTimeout(() => wx.switchTab({ url: '/pages/teams/teams' }), 600);
  }
});
