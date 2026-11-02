const store = require('../../utils/store.js');
const T = require('../../utils/time.js');
const WHO = require('../../utils/who.js');

// 合理范围（0~3 岁）：超出多半是单位填错（斤 / 米）
const RANGE = {
  weight: { min: 1.5, max: 25, label: '体重', unit: 'kg' },
  height: { min: 40, max: 110, label: '身长', unit: 'cm' },
  head: { min: 30, max: 55, label: '头围', unit: 'cm' }
};

Page({
  data: { date: '', start: '', end: '', weight: '', height: '', head: '', errs: {}, hint: '' },

  onLoad() {
    const baby = store.profile();
    this._baby = baby;
    const today = T.ymd(Date.now());
    const last = store.list(store.K.growth).slice().sort((a, b) => (a.date < b.date ? 1 : -1))[0];
    this.setData({ date: today, start: baby.birthday, end: today, hint: last ? '上次 ' + last.date + '：' + last.weight + 'kg / ' + last.height + 'cm' : '' });
  },

  onDate(e) { this.setData({ date: e.detail.value }); },
  onInput(e) {
    const k = e.currentTarget.dataset.k;
    const errs = Object.assign({}, this.data.errs);
    delete errs[k];
    this.setData({ [k]: e.detail.value, errs });
  },

  validate() {
    const errs = {};
    ['weight', 'height', 'head'].forEach((k) => {
      const raw = String(this.data[k]).trim();
      const r = RANGE[k];
      if (!raw) { if (k !== 'head') errs[k] = '请填写' + r.label; return; }
      const v = Number(raw);
      if (isNaN(v)) errs[k] = r.label + '只能填数字';
      else if (v < r.min || v > r.max) errs[k] = r.label + '应在 ' + r.min + '~' + r.max + r.unit + ' 之间，请检查单位';
    });
    if (store.list(store.K.growth).some((g) => g.date === this.data.date)) errs.date = '这一天已经有记录了，可以先在列表里删除旧的';
    this.setData({ errs });
    return !Object.keys(errs).length;
  },

  save() {
    if (!this.validate()) return;
    const d = this.data;
    const rec = { date: d.date, weight: Number(d.weight).toFixed(2), height: Number(d.height).toFixed(1), head: d.head ? Number(d.head).toFixed(1) : '' };
    store.add(store.K.growth, rec);
    const m = T.age(this._baby.birthday, T.parse(d.date)).monthFloat;
    const ev = WHO.evaluate(this._baby.sex, 'weight', m, rec.weight);
    wx.vibrateShort({ type: 'light' });
    wx.showModal({
      title: '已保存',
      content: '体重' + ev.text + '。可以在生长曲线里查看趋势。',
      showCancel: false,
      confirmText: '看曲线',
      success: () => wx.navigateBack()
    });
  }
});
