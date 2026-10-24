const T = require('../../utils/store.js');
const { CITIES, TYPES } = require('../../utils/data.js');

const MAX_DAYS = 30;

// 联想：中文名包含、全拼前缀、首字母前缀都算命中；中文命中的那段高亮
function suggest(kw) {
  const k = kw.trim().toLowerCase();
  if (!k) return [];
  const out = [];
  CITIES.forEach(c => {
    const i = c.name.indexOf(kw.trim());
    if (i >= 0) out.push({ name: c.name, prov: c.prov, pre: c.name.slice(0, i), hit: kw.trim(), post: c.name.slice(i + kw.trim().length), rank: i });
    else if (c.py.indexOf(k) === 0 || c.ab.indexOf(k) === 0) out.push({ name: c.name, prov: c.prov, pre: '', hit: '', post: c.name, rank: 9 });
  });
  return out.sort((a, b) => a.rank - b.rank).slice(0, 6);
}

Page({
  data: {
    city: '', cityOk: false, sugs: [], showSug: false, focus: false,
    start: '', end: '', minDate: '', maxDate: '', startText: '', endText: '', days: 0,
    dateErr: '', people: 2, types: TYPES, type: TYPES[0], title: '', titleTouched: false,
    canSubmit: false, cityErr: ''
  },
  onLoad(q) {
    const t0 = T.toStr(T.today());
    const start = T.addDays(t0, 7);
    const end = T.addDays(t0, 10);
    this.setData({ minDate: t0, maxDate: T.addDays(t0, 365), start, end });
    if (q && q.city) this.pickCity(decodeURIComponent(q.city));
    this.check();
  },
  // —— 目的地：受控输入 + 联想下拉 ——
  onCityInput(e) {
    const v = (e.detail.value || '').replace(/\s+/g, '').slice(0, 10);
    const sugs = suggest(v);
    const exact = CITIES.some(c => c.name === v);
    this.setData({ city: v, sugs, showSug: sugs.length > 0 && !exact, cityOk: exact, cityErr: '' });
    this.check();
    return v;
  },
  onCityFocus() { this.setData({ showSug: this.data.sugs.length > 0 && !this.data.cityOk }); },
  onCityBlur() {
    // 失焦稍等一下再收起，否则点下拉项时下拉先没了
    setTimeout(() => {
      const err = this.data.city && !this.data.cityOk ? '暂不支持「' + this.data.city + '」，请从下拉列表中选择' : '';
      this.setData({ showSug: false, cityErr: err });
    }, 200);
  },
  onSug(e) { this.pickCity(e.currentTarget.dataset.name); },
  pickCity(name) {
    this.setData({ city: name, cityOk: true, showSug: false, sugs: [], cityErr: '' });
    this.check();
  },
  clearCity() { this.setData({ city: '', cityOk: false, sugs: [], showSug: false, cityErr: '' }); this.check(); },
  // —— 日期 ——
  onStart(e) { this.setData({ start: e.detail.value }); this.check(); },
  onEnd(e) { this.setData({ end: e.detail.value }); this.check(); },
  // —— 人数 ——
  minus() { if (this.data.people > 1) { this.setData({ people: this.data.people - 1 }); this.check(); } },
  plus() {
    if (this.data.people >= 20) { wx.showToast({ title: '最多 20 人', icon: 'none' }); return; }
    this.setData({ people: this.data.people + 1 }); this.check();
  },
  onType(e) { this.setData({ type: e.currentTarget.dataset.t }); },
  onTitle(e) { this.setData({ title: e.detail.value.slice(0, 20), titleTouched: true }); },
  check() {
    const d = this.data;
    const days = T.diff(d.start, d.end) + 1;
    let dateErr = '';
    if (days < 1) dateErr = '返回日期不能早于出发日期';
    else if (days > MAX_DAYS) dateErr = '单个行程最长 ' + MAX_DAYS + ' 天';
    const patch = {
      startText: T.short(d.start) + ' ' + T.week(d.start), endText: T.short(d.end) + ' ' + T.week(d.end),
      days: dateErr ? 0 : days, dateErr, canSubmit: d.cityOk && !dateErr
    };
    if (!d.titleTouched) patch.title = d.cityOk && !dateErr ? d.city + ' ' + days + ' 日游' : '';
    this.setData(patch);
  },
  submit() {
    if (!this.data.canSubmit) {
      if (!this.data.cityOk) this.setData({ cityErr: '请先选择目的地' });
      return;
    }
    const d = this.data;
    const trip = T.put(T.newTrip({ city: d.city, start: d.start, end: d.end, people: d.people, type: d.type, title: d.title || d.city + ' ' + d.days + ' 日游' }));
    const ch = typeof this.getOpenerEventChannel === 'function' ? this.getOpenerEventChannel() : null;
    if (ch && ch.emit) ch.emit('created', { id: trip.id, title: trip.title });
    wx.showToast({ title: '行程已创建', icon: 'success' });
    // 从「灵感」页进来时上一页不是行程列表：直接切到行程 tab
    const pages = getCurrentPages();
    const prev = pages.length > 1 ? pages[pages.length - 2].route : '';
    if (prev === 'pages/trips/trips') wx.navigateBack();
    else wx.switchTab({ url: '/pages/trips/trips' });
  }
});
