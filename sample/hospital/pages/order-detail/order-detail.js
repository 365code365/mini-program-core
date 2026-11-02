const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const flows = require('../../utils/flows.js');
const { qrDots } = require('../../utils/qr.js');
Page({
  data: { order: null, st: null, steps: [], qr: [], payType: '', created: '', bill: null, reports: 0 },
  onLoad(q) { this.id = q.id; },
  onShow() {
    this.load();
    clearInterval(this.timer);
    this.timer = setInterval(() => this.load(), 15000);
  },
  onHide() { clearInterval(this.timer); },
  onUnload() { clearInterval(this.timer); },
  load() {
    store.tick();
    const o = store.findOrder(this.id);
    if (!o) {
      this.setData({ order: null });
      return;
    }
    const st = store.stageOf(o);
    const bill = store.getBills().find(b => b.orderId === o.id && b.kind === 'clinic') || null;
    this.setData({
      order: o,
      st,
      steps: st.stage >= 0 ? st.names.map((name, i) => ({ name, done: i <= st.stage })) : [],
      qr: this.data.qr.length ? this.data.qr : qrDots(o.id),
      payType: o.payType || '上海医保',
      created: o.createdAt ? store.dateTime(o.createdAt) : '',
      bill,
      reports: bill ? store.getReports().filter(r => r.billId === bill.id).length : 0
    });
  },
  onCheckin() { flows.checkin(this.data.order).then(() => this.load()); },
  onCancel() { flows.cancel(this.data.order).then(() => this.load()); },
  onCopy() { host.copy(this.data.order.id); },
  onCall() { host.call(data.hospital.phone, '医院总机'); },
  // 传科室 id 而不是诊室文字：中文进 URL 要编解码，微信与引擎对 query 解码的时机不一样
  onNavi() { wx.navigateTo({ url: '/pages/navi/navi' + (this.data.order.deptId ? '?dept=' + this.data.order.deptId : '') }); },
  onBills() { wx.navigateTo({ url: '/pages/bills/bills' }); },
  onReports() { wx.navigateTo({ url: '/pages/reports/reports' }); },
  onAgain() {
    const o = this.data.order;
    const doc = data.findDoctor(o.doctorId) || data.doctors.find(d => d.name === o.doctor);
    wx.navigateTo({ url: '/pages/doctor/doctor?id=' + (doc ? doc.id : 'd1') });
  }
});
