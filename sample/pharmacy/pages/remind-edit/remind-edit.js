// 新增 / 编辑用药提醒：药名（可从买过的药里选）、每次剂量、服药时间（picker mode="time"，最多 4 个）、
// 每周几（checkbox-group，value 是 1~7 的字符串数组）、备注；可同步到手机日历（wx.addPhoneRepeatCalendar，失败只在小程序内提醒）
const S = require('../../utils/store.js');

const WEEK = [{ v: '1', t: '一' }, { v: '2', t: '二' }, { v: '3', t: '三' }, { v: '4', t: '四' }, { v: '5', t: '五' }, { v: '6', t: '六' }, { v: '7', t: '日' }];
const DOSES = ['1 片', '2 片', '1 粒', '1 袋', '5ml', '1 滴'];

Page({
  data: { id: '', drug: '', dose: '1 片', doses: DOSES, times: ['08:00'], week: [], days: [], note: '', cal: false, bought: [], errs: {} },
  onLoad(q) {
    const bought = [];
    S.get('orders', []).forEach(o => o.items.forEach(i => {
      const n = i.brand + ' ' + i.name;
      if (bought.indexOf(n) < 0 && !/器|口罩|血压计|血氧/.test(i.name)) bought.push(n);
    }));
    let r = null;
    if (q.id) r = S.get('reminders', []).find(x => x.id === q.id);
    const base = r || { drug: q.drug ? decodeURIComponent(q.drug) : '', dose: '1 片', times: ['08:00'], days: [1, 2, 3, 4, 5, 6, 7], note: '', cal: false };
    wx.setNavigationBarTitle({ title: r ? '编辑提醒' : '新增提醒' });
    this.setData({ id: r ? r.id : '', drug: base.drug, dose: base.dose, times: base.times.slice(), note: base.note, cal: !!base.cal, bought: bought.slice(0, 6) });
    this.setDays(base.days.map(String));
  },
  setDays(vals) {
    this.setData({ days: vals, week: WEEK.map(w => Object.assign({}, w, { on: vals.indexOf(w.v) >= 0 })), 'errs.days': '' });
  },
  onDrug(e) { this.setData({ drug: e.detail.value, 'errs.drug': '' }); },
  pickDrug(e) { this.setData({ drug: e.currentTarget.dataset.n, 'errs.drug': '' }); },
  pickDose(e) { this.setData({ dose: e.currentTarget.dataset.d }); },
  onDose(e) { this.setData({ dose: e.detail.value }); },
  onTime(e) {
    const i = +e.currentTarget.dataset.i;
    this.setData({ ['times[' + i + ']']: e.detail.value, 'errs.times': '' });
  },
  addTime() {
    if (this.data.times.length >= 4) { wx.showToast({ title: '一天最多 4 次', icon: 'none' }); return; }
    const last = this.data.times[this.data.times.length - 1] || '08:00';
    const h = Math.min(22, +last.slice(0, 2) + 5);
    this.setData({ times: this.data.times.concat(('0' + h).slice(-2) + ':00') });
  },
  delTime(e) {
    if (this.data.times.length <= 1) { wx.showToast({ title: '至少保留 1 个时间', icon: 'none' }); return; }
    const t = this.data.times.slice();
    t.splice(+e.currentTarget.dataset.i, 1);
    this.setData({ times: t });
  },
  onDays(e) { this.setDays(e.detail.value); },
  quick(e) {
    const k = e.currentTarget.dataset.k;
    this.setDays(k === 'all' ? ['1', '2', '3', '4', '5', '6', '7'] : k === 'work' ? ['1', '2', '3', '4', '5'] : ['6', '7']);
  },
  onNote(e) { this.setData({ note: e.detail.value }); },
  onCal(e) { this.setData({ cal: e.detail.value }); },
  save() {
    const d = this.data, errs = {};
    if (d.drug.trim().length < 2) errs.drug = '请填写药品名称';
    if (!d.dose.trim()) errs.dose = '请填写每次剂量';
    if (new Set(d.times).size !== d.times.length) errs.times = '服药时间有重复';
    if (!d.days.length) errs.days = '请至少选择一天';
    this.setData({ errs });
    if (Object.keys(errs).length) { wx.showToast({ title: Object.values(errs)[0], icon: 'none' }); return; }
    const rec = { id: d.id || 'R' + Date.now(), drug: d.drug.trim(), dose: d.dose.trim(), times: d.times.slice().sort(),
      days: d.days.map(Number).sort((a, b) => a - b), note: d.note.trim(), on: true, cal: d.cal };
    const all = S.get('reminders', []);
    const i = all.findIndex(r => r.id === rec.id);
    if (i >= 0) all[i] = rec; else all.unshift(rec);
    S.set('reminders', all);
    if (d.cal) this.syncCalendar(rec); else this.done();
  },
  // 每个服药时间一条重复日程：每天服的按天重复，其余按周重复
  syncCalendar(rec) {
    const [hh, mm] = rec.times[0].split(':').map(Number);
    const start = new Date();
    start.setHours(hh, mm, 0, 0);
    if (start.getTime() < Date.now()) start.setDate(start.getDate() + 1);
    wx.addPhoneRepeatCalendar({
      title: '服药：' + rec.drug + ' ' + rec.dose,
      description: rec.note || '康源大药房用药提醒',
      startTime: Math.floor(start.getTime() / 1000),
      alarm: true, alarmOffset: 0,
      repeatInterval: rec.days.length === 7 ? 'day' : 'week',
      success: () => { wx.showToast({ title: '已同步到手机日历', icon: 'success' }); setTimeout(() => this.done(true), 800); },
      fail: () => wx.showModal({
        title: '未能写入手机日历', content: '提醒已在小程序内保存，打开「用药提醒」即可查看今日待服。可在手机设置中允许访问日历后再试。',
        showCancel: false, confirmText: '知道了', success: () => this.done(true)
      })
    });
  },
  done(quiet) {
    if (!quiet) wx.showToast({ title: '已保存', icon: 'success' });
    setTimeout(() => wx.navigateBack({ fail: () => wx.redirectTo({ url: '/pages/remind/remind' }) }), quiet ? 0 : 600);
  },
  remove() {
    wx.showModal({
      title: '删除提醒', content: '确定删除「' + this.data.drug + '」的用药提醒吗？', confirmText: '删除', confirmColor: '#f04a3e',
      success: r => {
        if (!r.confirm) return;
        S.set('reminders', S.get('reminders', []).filter(x => x.id !== this.data.id));
        wx.navigateBack();
      }
    });
  }
});
