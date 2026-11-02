const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
Page({
  data: {
    doc: {}, deptName: '', room: '', date: '', week: '', time: '',
    patients: [], patientNames: [], pIdx: 0, patient: {},
    payTypes: ['自费', '上海医保', '异地医保', '商业保险'], payIdx: 1,
    desc: '', agree: false
  },
  onLoad(q) {
    const doc = data.findDoctor(q.id || 'd1') || data.doctors[0];
    const date = q.date || data.nextDays(2)[1].key;
    const p = date.split('-');
    const week = WEEK[new Date(+p[0], +p[1] - 1, +p[2]).getDay()];
    const dept = data.findDept(doc.dept);
    this.setData({ doc, deptName: dept.name, room: dept.room, date, week, time: q.time || '09:00-10:00' });
  },
  // 从「管理就诊人」回来：列表可能变了，尽量留住刚才选的人
  onShow() {
    const keep = this.data.patient && this.data.patient.id;
    const patients = store.getPatients().map(x => Object.assign({}, x, { sx: x.name.charAt(0) }));
    let pIdx = patients.findIndex(x => x.id === keep);
    if (pIdx < 0) pIdx = patients.findIndex(x => x.isDefault);
    if (pIdx < 0) pIdx = 0;
    this.setData({ patients, patientNames: patients.map(x => x.name + '（' + x.relation + '）'), pIdx, patient: patients[pIdx] });
  },
  onPatient(e) {
    const i = Number(e.detail.value);
    this.setData({ pIdx: i, patient: this.data.patients[i] });
  },
  onPay(e) { this.setData({ payIdx: Number(e.detail.value) }); },
  onDesc(e) { this.setData({ desc: e.detail.value }); },
  onAgree(e) { this.setData({ agree: e.detail.value.length > 0 }); },
  goPatients() { wx.navigateTo({ url: '/pages/patients/patients' }); },
  submit() {
    if (this.paying) return;
    if (!this.data.agree) {
      wx.showToast({ title: '请先阅读并同意挂号须知', icon: 'none' });
      return;
    }
    const d = this.data;
    const err = store.canBook({ doctorId: d.doc.id, deptId: d.doc.dept, date: d.date, time: d.time, patientId: d.patient.id, patientName: d.patient.name });
    if (err) {
      wx.showToast({ title: err, icon: 'none' });
      return;
    }
    this.paying = true;
    host.pay(d.doc.fee, d.patient.name + ' · ' + d.date + ' ' + d.time + '\n' + d.deptName + ' ' + d.doc.name + ' 挂号费').then(ok => {
      this.paying = false;
      if (!ok) {
        wx.showToast({ title: '已取消支付，号源未锁定', icon: 'none' });
        return;
      }
      store.createOrder({ doc: d.doc, date: d.date, time: d.time, patient: d.patient, payType: d.payTypes[d.payIdx], desc: d.desc.trim() });
      wx.showToast({ title: '挂号成功', icon: 'success' });
      setTimeout(() => wx.switchTab({ url: '/pages/orders/orders' }), 800);
    });
  }
});
