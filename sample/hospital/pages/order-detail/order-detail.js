const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const flows = require('../../utils/flows.js');
const { qrDots } = require('../../utils/qr.js');
// 「康源大药房」（sample/pharmacy）：处方里的药去那边买
const PHARMACY = { appId: 'wx372a5c26935a3559', name: '康源大药房' };
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
  // 处方里的药：去「康源大药房」（sample/pharmacy）按药名搜。药名去掉规格（「布洛芬混悬液 100ml」→「布洛芬混悬液」），
  // 放 extraData 而不是 path 的 query（理由同上：中文进 URL 要编解码）
  onBuyDrug(e) {
    const name = String(e.currentTarget.dataset.name || '');
    const o = this.data.order;
    wx.navigateToMiniProgram({
      appId: PHARMACY.appId,
      path: 'pages/search/search',
      extraData: { from: 'hospital', fromName: '健康挂号', kw: name.split(' ')[0], drug: name, dept: o.dept, doctor: o.doctor },
      fail(err) {
        const msg = (err && err.errMsg) || 'navigateToMiniProgram:fail';
        // 用户在确认框里点了取消：不再弹一次
        if (msg.indexOf('cancel') < 0) wx.showModal({ title: '没能打开「' + PHARMACY.name + '」', content: msg, showCancel: false });
      }
    });
  },
  onBills() { wx.navigateTo({ url: '/pages/bills/bills' }); },
  onReports() { wx.navigateTo({ url: '/pages/reports/reports' }); },
  onAgain() {
    const o = this.data.order;
    const doc = data.findDoctor(o.doctorId) || data.doctors.find(d => d.name === o.doctor);
    wx.navigateTo({ url: '/pages/doctor/doctor?id=' + (doc ? doc.id : 'd1') });
  }
});
