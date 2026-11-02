const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const clinic = require('../../utils/clinic.js');
const host = require('../../utils/host.js');
const TYPES = [
  { key: 'text', name: '图文问诊', price: 19, sub: '30 分钟内回复' },
  { key: 'video', name: '视频问诊', price: 39, sub: '10 分钟内接通' },
  { key: 'renew', name: '复诊续方', price: 9.9, sub: '本院就诊过的慢病' }
];
const STATUS = { wait: '待接诊', reply: '医生已回复', done: '已结束', cancel: '已取消' };
const DEPTS = [];
data.groups.forEach(g => g.depts.forEach(d => DEPTS.push({ id: d.id, name: d.name, group: g.id })));
Page({
  data: {
    types: TYPES, typeIdx: 0,
    patients: [], patientNames: [], pIdx: 0,
    deptNames: DEPTS.map(d => d.name), deptIdx: 0,
    desc: '', renewHint: '', list: [],
    // 打过字的输入框，之后 setData 清空 value 画面不更新：提交后换个 id 让它重建
    formSeq: 0
  },
  onLoad(q) {
    // 从报告详情过来：带上报告，科室默认开单科室
    const r = q.report ? store.getReports().find(x => x.id === q.report) : null;
    if (r) {
      const t = clinic.REPORTS[r.key];
      const di = DEPTS.findIndex(d => d.name === r.dept);
      this.presetPatient = r.patientId;
      this.setData({ desc: '请帮我解读 ' + r.date + ' 的' + t.name + '报告，' + (t.items ? '有几项结果标了异常' : '结论看不太懂') + '，需要怎么处理？', deptIdx: Math.max(0, di) });
    }
  },
  onShow() {
    const patients = store.getPatients();
    let pIdx = patients.findIndex(p => p.id === (this.presetPatient || (this.data.patients[this.data.pIdx] || {}).id));
    if (pIdx < 0) pIdx = Math.max(0, patients.findIndex(p => p.isDefault));
    this.presetPatient = '';
    this.setData({ patients, patientNames: patients.map(p => p.name + '（' + p.relation + '）'), pIdx });
    this.updateRenew();
    this.refresh();
    clearInterval(this.timer);
    this.timer = setInterval(() => this.refresh(), 15000);
  },
  onHide() { clearInterval(this.timer); },
  onUnload() { clearInterval(this.timer); },
  refresh() {
    store.tick();
    this.setData({
      list: store.getConsults().map(c => Object.assign({}, c, {
        statusText: STATUS[c.status], time: store.dateTime(c.createdAt),
        unpaid: c.type === 'renew' && store.getBills().some(b => b.consultId === c.id && b.status === 'unpaid')
      }))
    });
  },
  // 复诊续方：要有本院已完成的就诊，按最近一次就诊的科室续方
  lastVisit() {
    const p = this.data.patients[this.data.pIdx];
    return store.getOrders().find(o => o.patientId === p.id && o.status === 'done') || null;
  },
  updateRenew() {
    if (TYPES[this.data.typeIdx].key !== 'renew') return this.setData({ renewHint: '' });
    const v = this.lastVisit();
    this.setData({ renewHint: v ? '将按 ' + v.date + ' ' + v.dept + ' ' + v.doctor + ' 的处方续方' : '该就诊人近 6 个月没有本院就诊记录，无法续方' });
  },
  pickType(e) {
    this.setData({ typeIdx: Number(e.currentTarget.dataset.i) });
    this.updateRenew();
  },
  onPatient(e) {
    this.setData({ pIdx: Number(e.detail.value) });
    this.updateRenew();
  },
  onDept(e) { this.setData({ deptIdx: Number(e.detail.value) }); },
  onDesc(e) { this.setData({ desc: e.detail.value }); },
  submit() {
    if (this.paying) return;
    const type = TYPES[this.data.typeIdx];
    const p = this.data.patients[this.data.pIdx];
    let dept = DEPTS[this.data.deptIdx];
    const desc = this.data.desc.trim();
    if (type.key === 'renew') {
      const v = this.lastVisit();
      if (!v) {
        wx.showToast({ title: '没有本院就诊记录，无法续方', icon: 'none' });
        return;
      }
      dept = DEPTS.find(d => d.id === v.deptId) || dept;
    } else if (desc.length < 10) {
      wx.showToast({ title: '请至少用 10 个字描述病情', icon: 'none' });
      return;
    }
    if (store.getConsults().some(c => c.status === 'wait' && c.patientId === p.id)) {
      wx.showToast({ title: p.name + ' 还有待接诊的问诊', icon: 'none' });
      return;
    }
    this.paying = true;
    host.pay(type.price, p.name + ' · ' + type.name + ' · ' + dept.name).then(ok => {
      this.paying = false;
      if (!ok) {
        wx.showToast({ title: '已取消支付', icon: 'none' });
        return;
      }
      store.addConsult({ type: type.key, typeName: type.name, price: type.price, deptId: dept.id, dept: dept.name, groupId: dept.group, patientId: p.id, patient: p.name, desc: desc || '按上次处方续方' });
      this.setData({ desc: '', formSeq: this.data.formSeq + 1 });
      wx.showToast({ title: '已提交，医生接诊后会回复您', icon: 'none' });
      this.refresh();
    });
  },
  onCancel(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '取消问诊', content: '医生还未接诊，取消后问诊费原路退回', confirmText: '取消问诊', cancelText: '再等等',
      success: r => {
        if (!r.confirm) return;
        wx.showToast({ title: store.cancelConsult(id) ? '已取消，问诊费原路退回' : '医生已接诊，无法取消', icon: 'none' });
        this.refresh();
      }
    });
  },
  onFinish(e) {
    store.updateConsult(e.currentTarget.dataset.id, { status: 'done', doneAt: Date.now() });
    wx.showToast({ title: '问诊已结束', icon: 'none' });
    this.refresh();
  },
  goBills() { wx.navigateTo({ url: '/pages/bills/bills' }); }
});
