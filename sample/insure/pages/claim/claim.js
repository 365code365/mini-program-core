// 理赔：?id=<保单> 填写申请；?claim=<案件> 看进度（store.claimView 按提交后经过的时间推进，每 5 秒刷新）
const store = require('../../utils/store.js');
const util = require('../../utils/util.js');
const TYPES = ['疾病住院', '意外伤害', '门急诊', '重疾确诊'];
const SAMPLES = ['/images/doc/record.jpg', '/images/doc/invoice.jpg', '/images/doc/idcard.jpg'];
Page({
  data: {
    mode: 'form', p: null, c: null, types: TYPES, type: 0, date: '', today: '', amount: '', desc: '', account: '',
    images: [], errors: {}, submitting: false
  },
  onLoad(q) {
    if (q.claim) { this.claimId = q.claim; this.setData({ mode: 'view' }); this.refreshView(); this.timer = setInterval(() => this.refreshView(), 5000); return; }
    const p = store.getPolicy(q.id);
    const today = util.ymd(new Date());
    this.setData({ p, today, date: today, account: p ? '招商银行储蓄卡（8848）' : '' });
  },
  onUnload() { clearInterval(this.timer); },
  refreshView() {
    const raw = store.claims().find(x => x.id === this.claimId);
    if (!raw) return;
    const c = store.claimView(raw);
    this.setData({ c, amountText: util.money(c.amount), approvedText: util.money(c.approved) });
    if (c.finished) clearInterval(this.timer);
  },
  onType(e) { this.setData({ type: Number(e.detail.value) }); },
  onDate(e) { this.setData({ date: e.detail.value }); },
  onAmount(e) { this.setData({ amount: e.detail.value, 'errors.amount': '' }); },
  onDesc(e) { this.setData({ desc: e.detail.value, 'errors.desc': '' }); },
  onAccount(e) { this.setData({ account: e.detail.value }); },
  addImage() {
    const left = 6 - this.data.images.length;
    if (left <= 0) return;
    const done = paths => this.setData({ images: this.data.images.concat(paths).slice(0, 6), 'errors.images': '' });
    wx.chooseMedia({
      count: left, mediaType: ['image'], sourceType: ['album', 'camera'],
      success: r => done(r.tempFiles.map(f => f.tempFilePath)),
      // 没有相册能力（桌面引擎）：用示例材料，流程照常走
      fail: () => { wx.showToast({ title: '无法打开相册，已附上示例材料', icon: 'none' }); done(SAMPLES.slice(0, left)); }
    });
  },
  preview(e) { wx.previewImage({ current: e.currentTarget.dataset.src, urls: this.data.images }); },
  removeImage(e) {
    const i = Number(e.currentTarget.dataset.i);
    this.setData({ images: this.data.images.filter((_, k) => k !== i) });
  },
  submit() {
    const d = this.data, errors = {};
    const amount = parseFloat(d.amount);
    if (!(amount > 0)) errors.amount = '请填写申请理赔金额';
    else if (amount > (d.p ? d.p.amount : 0)) errors.amount = '超过保障额度 ' + d.p.amountText;
    if ((d.desc || '').trim().length < 10) errors.desc = '请用至少 10 个字说明出险经过';
    if (!d.images.length) errors.images = '请至少上传一张病历、发票或事故证明';
    this.setData({ errors });
    if (Object.keys(errors).length) { wx.showToast({ title: errors[Object.keys(errors)[0]], icon: 'none' }); return; }
    this.setData({ submitting: true });
    wx.showLoading({ title: '提交中', mask: true });
    const c = store.addClaim({
      id: util.serial('LP'), policyId: d.p.id, productName: d.p.productName, type: TYPES[d.type], date: d.date,
      amount, approved: Math.round(amount * 0.92 * 100) / 100, desc: d.desc.trim(), images: d.images, account: d.account, createdAt: Date.now()
    });
    setTimeout(() => { wx.hideLoading(); wx.redirectTo({ url: '/pages/claim/claim?claim=' + c.id }); }, 500);
  },
  revoke() {
    wx.showModal({
      title: '撤销理赔申请', content: '撤销后案件将终止处理，可以重新提交。', confirmColor: '#f0524a', confirmText: '撤销',
      success: r => { if (r.confirm) { store.cancelClaim(this.claimId); this.refreshView(); } }
    });
  },
  call() {
    const phone = '400-820-6688';
    wx.makePhoneCall({ phoneNumber: phone, fail: () => wx.showModal({ title: '理赔专线', content: phone + '（7×24 小时）', confirmText: '复制号码', success: r => { if (r.confirm) wx.setClipboardData({ data: phone }); } }) });
  }
});
