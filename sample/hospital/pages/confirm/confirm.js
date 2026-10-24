const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];

Page({
  data: {
    doc: {}, deptName: '', date: '', week: '', time: '',
    patients: [], patientNames: [], pIdx: 0, patient: {},
    payTypes: ['自费', '上海医保', '异地医保', '商业保险'], payIdx: 1,
    desc: '', agree: false
  },
  onLoad(q) {
    const doc = data.findDoctor(q.id || 'd1') || data.doctors[0];
    const date = q.date || data.nextDays(2)[1].key;
    const p = date.split('-');
    const week = WEEK[new Date(+p[0], +p[1] - 1, +p[2]).getDay()];
    this.setData({ doc, deptName: data.findDept(doc.dept).name, date, week, time: q.time || '09:00-10:00' });
  },
  onShow() {
    const patients = store.getPatients().map(x => Object.assign({}, x, { sx: x.name.charAt(0) }));
    let pIdx = patients.findIndex(x => x.isDefault);
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
    if (!this.data.agree) {
      wx.showToast({ title: '请先阅读并同意挂号须知', icon: 'none' });
      return;
    }
    const d = this.data;
    wx.showModal({
      title: '确认挂号',
      content: d.patient.name + ' · ' + d.date + ' ' + d.time + '\n' + d.deptName + ' ' + d.doc.name + '，支付 ¥' + d.doc.fee,
      confirmText: '确认支付',
      success: (res) => {
        if (!res.confirm) return;
        const orders = store.getOrders();
        const no = 10 + (orders.length * 7) % 30;
        orders.unshift({
          id: 'R' + d.date.replace(/-/g, '') + (100 + orders.length),
          doctor: d.doc.name, title: d.doc.title, dept: d.deptName, date: d.date, time: d.time,
          patient: d.patient.name, fee: d.doc.fee, status: 'wait', no, room: '门诊 3 楼 B 区 307'
        });
        store.saveOrders(orders);
        wx.showToast({ title: '挂号成功', icon: 'success' });
        setTimeout(() => wx.switchTab({ url: '/pages/orders/orders' }), 800);
      }
    });
  }
});
