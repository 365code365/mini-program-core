// 发起审批：fields 来自 utils/forms.js，wxml 里 <block wx:for> + wx:if/elif 按 type 渲染。
// 审批人经 navigateTo 的 events 回传（pick 页 getOpenerEventChannel().emit('picked')）。
const F = require('../../utils/forms.js');
const S = require('../../utils/store.js');
const O = require('../../utils/org.js');

Page({
  data: { type: 'leave', typeInfo: null, fields: [], values: {}, errors: {}, approver: null, submitting: false },
  onLoad(q) {
    const type = F.fields[q.type] ? q.type : 'leave';
    const typeInfo = F.types.find(t => t.id === type);
    wx.setNavigationBarTitle({ title: '发起' + typeInfo.name });
    const today = S.ymd(new Date());
    const values = {};
    const fields = F.fields[type].map(f => {
      let v = f.value !== undefined ? f.value : '';
      if (f.type === 'date') v = today;
      if (f.type === 'month') v = today.slice(0, 7);
      values[f.key] = v;
      return Object.assign({}, f, { idx: f.type === 'selector' ? -1 : 0 });
    });
    const preset = q.approver ? O.emp(q.approver) : O.emp(O.FLOW[type][0]);
    values.approver = preset.id;
    this.setData({ type, typeInfo, fields, values, today, approver: { id: preset.id, name: preset.name, title: preset.title, deptName: preset.deptName } });
    this.recalc();
  },
  set(key, v) {
    this.setData({ ['values.' + key]: v, ['errors.' + key]: '' });
    this.recalc();
  },
  recalc() {
    if (this.data.type !== 'leave') return;
    this.setData({ 'values.days': F.leaveDays(this.data.values) });
  },
  onInput(e) { this.set(e.currentTarget.dataset.key, e.detail.value); },
  onPicker(e) {
    const key = e.currentTarget.dataset.key;
    const f = this.data.fields.find(x => x.key === key);
    if (f.type === 'selector') {
      const i = Number(e.detail.value);
      const fi = this.data.fields.indexOf(f);
      this.setData({ ['fields[' + fi + '].idx']: i });
      this.set(key, f.options[i]);
    } else this.set(key, e.detail.value);
  },
  onRadio(e) { this.set(e.currentTarget.dataset.key, e.detail.value); },
  onSwitch(e) { this.set(e.currentTarget.dataset.key, e.detail.value); },
  pickApprover() {
    wx.navigateTo({
      url: '/pages/pick/pick',
      events: {
        picked: emp => {
          this.setData({ approver: emp });
          this.set('approver', emp.id);
        }
      }
    });
  },
  validate() {
    const v = this.data.values, errors = {};
    this.data.fields.forEach(f => {
      const x = v[f.key];
      if (f.required && (x === '' || x === undefined || x === null)) errors[f.key] = '请填写' + f.label;
    });
    if (this.data.type === 'leave') {
      if (!errors.end && v.end < v.start) errors.end = '结束日期不能早于开始日期';
      else if (!v.days) errors.days = '结束时间要晚于开始时间';
      if (!errors.reason && v.reason.trim().length < 4) errors.reason = '请简要说明事由（至少 4 个字）';
    }
    if (this.data.type === 'expense') {
      const a = Number(v.amount);
      if (!errors.amount && !(a > 0)) errors.amount = '金额要大于 0';
      else if (a > 50000) errors.amount = '单笔超过 5 万元请走付款申请';
      if (!errors.invoices && !(Number(v.invoices) >= 1)) errors.invoices = '至少 1 张发票';
    }
    if (this.data.type === 'seal' && !errors.copies && !(Number(v.copies) >= 1)) errors.copies = '至少 1 份';
    this.setData({ errors });
    const keys = Object.keys(errors);
    if (keys.length) wx.showToast({ title: errors[keys[0]], icon: 'none' });
    return keys.length === 0;
  },
  submit() {
    if (this.data.submitting || !this.validate()) return;
    this.setData({ submitting: true });
    wx.showLoading({ title: '提交中' });
    setTimeout(() => {
      wx.hideLoading();
      const values = Object.assign({}, this.data.values);
      values.reason = (values.reason || '').trim();
      const id = S.addApproval(this.data.type, values);
      wx.showToast({ title: '已提交', icon: 'success' });
      setTimeout(() => wx.redirectTo({ url: '/pages/approval/approval?id=' + id + '&fresh=1' }), 400);
    }, 500);
  }
});
