// 参观预约：日期（未来 7 个开放日）→ 时段（余票）→ 人数（1~5）→ 逐人实名（姓名 + 身份证校验位）→ 同意须知 → 提交。
// 同一证件同一天只能有一张待使用的预约；提交成功 redirectTo 凭证页（返回不会回到填写页）
const { SLOTS } = require('../../utils/data.js');
const U = require('../../utils/util.js');

const blank = () => ({ name: '', id: '', err: '' });

Page({
  data: { days: [], date: '', slots: [], slotId: '', count: 1, visitors: [blank()], saved: [], agree: false, submitting: false, formErr: '' },
  onLoad(q) {
    const days = U.bookDays();
    const date = days.some(d => d.date === q.date) ? q.date : days[0].date;
    this.setData({ days, saved: U.load('visitors', []) });
    this.pickDate(date);
  },
  pickDate(date) {
    const slots = U.slotsFor(date);
    const keep = slots.find(s => s.id === this.data.slotId && !s.full);
    this.setData({ date, slots, slotId: keep ? keep.id : '' });
  },
  onDate(e) { this.pickDate(e.currentTarget.dataset.date); },
  onSlot(e) {
    const s = this.data.slots.find(x => x.id === e.currentTarget.dataset.id);
    if (s.full) { wx.showToast({ title: '该时段已约满，请换个时段', icon: 'none' }); return; }
    if (s.left < this.data.count) { wx.showToast({ title: '该时段仅剩 ' + s.left + ' 个名额', icon: 'none' }); return; }
    this.setData({ slotId: s.id, formErr: '' });
  },
  step(e) {
    const n = Math.min(5, Math.max(1, this.data.count + +e.currentTarget.dataset.d));
    if (n === this.data.count) { wx.showToast({ title: n === 5 ? '每单最多 5 人' : '至少 1 人', icon: 'none' }); return; }
    const v = this.data.visitors.slice(0, n);
    while (v.length < n) v.push(blank());
    this.setData({ count: n, visitors: v });
  },
  onInput(e) {
    const { i, f } = e.currentTarget.dataset;
    this.setData({ ['visitors[' + i + '].' + f]: e.detail.value.trim(), ['visitors[' + i + '].err']: '' });
  },
  useSaved(e) {
    const p = this.data.saved[e.currentTarget.dataset.i];
    const v = this.data.visitors;
    if (v.some(x => x.id === p.id)) { wx.showToast({ title: p.name + ' 已在名单里', icon: 'none' }); return; }
    let i = v.findIndex(x => !x.name && !x.id);
    if (i < 0) {
      if (v.length >= 5) { wx.showToast({ title: '每单最多 5 人', icon: 'none' }); return; }
      v.push(blank()); i = v.length - 1;
    }
    v[i] = { name: p.name, id: p.id, err: '' };
    this.setData({ visitors: v, count: v.length });
  },
  onAgree(e) { this.setData({ agree: e.detail.value.length > 0 }); },
  validate() {
    const { date, slotId, visitors, agree, slots } = this.data;
    if (!slotId) return '请选择入馆时段';
    const s = slots.find(x => x.id === slotId);
    if (s.left < visitors.length) return '该时段余票不足 ' + visitors.length + ' 张';
    let bad = false;
    const seen = {};
    const exist = U.load('bookings', []).filter(b => b.date === date && b.status === '待使用');
    const v = visitors.map(p => {
      let err = '';
      if (!/^[\u4e00-\u9fa5·]{2,15}$/.test(p.name)) err = '请填写与证件一致的中文姓名';
      else err = U.checkIdCard(p.id);
      if (!err && seen[p.id.toUpperCase()]) err = '同一证件不能重复添加';
      if (!err && exist.some(b => b.ids.indexOf(p.id.toUpperCase()) >= 0)) err = '该证件当天已有预约';
      seen[p.id.toUpperCase()] = 1;
      if (err) bad = true;
      return Object.assign({}, p, { err });
    });
    this.setData({ visitors: v });
    if (bad) return '请检查参观人信息';
    if (!agree) return '请阅读并同意参观须知';
    return '';
  },
  submit() {
    if (this.data.submitting) return;
    const err = this.validate();
    this.setData({ formErr: err });
    if (err) { wx.showToast({ title: err, icon: 'none' }); return; }
    const { date, slotId, visitors, slots } = this.data;
    const slot = slots.find(s => s.id === slotId);
    wx.showModal({
      title: '确认预约', content: date + ' ' + slot.label + '，共 ' + visitors.length + ' 人：' + visitors.map(v => v.name).join('、'),
      confirmText: '确认预约', confirmColor: '#a45c34',
      success: r => { if (r.confirm) this.commit(slot); }
    });
  },
  commit(slot) {
    this.setData({ submitting: true });
    wx.showLoading({ title: '提交中' });
    const { date, visitors } = this.data;
    const id = 'B' + Date.now();
    const code = 'YC' + date.replace(/-/g, '').slice(2) + String(Math.floor(100000 + Math.random() * 900000));
    const rec = {
      id, code, date, slotId: slot.id, slot: slot.label, status: '待使用', createdAt: Date.now(),
      ids: visitors.map(v => v.id.toUpperCase()),
      visitors: visitors.map(v => ({ name: U.maskName(v.name), id: U.maskId(v.id.toUpperCase()) }))
    };
    U.save('bookings', [rec].concat(U.load('bookings', [])));
    const saved = U.load('visitors', []).filter(s => !visitors.some(v => v.id.toUpperCase() === s.id));
    U.save('visitors', visitors.map(v => ({ name: v.name, id: v.id.toUpperCase() })).concat(saved).slice(0, 8));
    setTimeout(() => {
      wx.hideLoading();
      this.setData({ submitting: false });
      wx.redirectTo({ url: '/pages/ticket/ticket?id=' + id + '&new=1' });
    }, 400);
  }
});
