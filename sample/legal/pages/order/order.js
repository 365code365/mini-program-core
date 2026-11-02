const D = require('../../utils/data.js');
const S = require('../../utils/store.js');

Page({
  data: {
    l: null, services: [], service: 'text', price: 0,
    cats: D.categories, catIdx: 0,
    desc: '', images: [], phone: '', agree: true,
    errors: {}, cashier: false, payWay: 'wx', paying: false
  },
  onLoad(q) {
    const raw = D.lawyerById(q.lawyer) || D.lawyers[0];
    const services = D.services.map(s => Object.assign({}, s, { price: raw.price[s.key] }));
    const service = services.some(s => s.id === q.service) ? q.service : 'text';
    const catIdx = Math.max(0, D.categories.findIndex(c => c.id === raw.fields[0]));
    this.setData({ l: D.decorate(raw), services, service, catIdx, price: raw.price[service], serviceName: services.find(s => s.id === service).name });
  },
  onService(e) {
    const s = this.data.services.find(x => x.id === e.detail.value);
    this.setData({ service: s.id, price: s.price, serviceName: s.name });
  },
  onCat(e) { this.setData({ catIdx: Number(e.detail.value) }); },
  onDesc(e) { this.setData({ desc: e.detail.value, 'errors.desc': '' }); },
  onPhone(e) { this.setData({ phone: e.detail.value, 'errors.phone': '' }); },
  onAgree(e) { this.setData({ agree: e.detail.value.length > 0 }); },
  addImage() {
    if (this.data.images.length >= 3) return wx.showToast({ title: '最多 3 张', icon: 'none' });
    wx.chooseImage({
      count: 3 - this.data.images.length,
      sizeType: ['compressed'],
      success: res => this.setData({ images: this.data.images.concat(res.tempFilePaths).slice(0, 3) }),
      fail: () => {
        // 桌面端没有相册：给一张示例证据图，流程照样走得通
        wx.showModal({
          title: '无法打开相册',
          content: '当前环境不能选图，是否使用示例证据图（租赁合同照片）？',
          confirmText: '使用示例',
          success: r => { if (r.confirm) this.setData({ images: this.data.images.concat(['/images/evidence.jpg']) }); }
        });
      }
    });
  },
  delImage(e) {
    const imgs = this.data.images.slice();
    imgs.splice(e.currentTarget.dataset.i, 1);
    this.setData({ images: imgs });
  },
  preview(e) { wx.previewImage({ urls: this.data.images, current: this.data.images[e.currentTarget.dataset.i] }); },
  validate() {
    const errors = {};
    if (this.data.desc.trim().length < 10) errors.desc = '请至少用 10 个字描述你的问题，律师才能准确判断';
    if (!/^1[3-9]\d{9}$/.test(this.data.phone)) errors.phone = '请输入 11 位手机号，律师回复后短信通知你';
    this.setData({ errors });
    if (!this.data.agree) { wx.showToast({ title: '请先同意服务须知', icon: 'none' }); return false; }
    return Object.keys(errors).length === 0;
  },
  submit() {
    if (!this.validate()) return;
    wx.showLoading({ title: '创建订单' });
    setTimeout(() => {
      wx.hideLoading();
      wx.requestPayment({
        timeStamp: String(Math.floor(Date.now() / 1000)), nonceStr: 'la' + Date.now(), package: 'prepay_id=wx_legal_demo', signType: 'RSA', paySign: 'demo',
        success: () => this.paid('微信支付'),
        // 桌面端没有支付宿主：弹页内模拟收银台，用户再确认一次才算付款
        fail: () => this.setData({ cashier: true })
      });
    }, 400);
  },
  pickWay(e) { this.setData({ payWay: e.currentTarget.dataset.w }); },
  closeCashier() { this.setData({ cashier: false }); wx.showToast({ title: '已取消支付', icon: 'none' }); },
  confirmPay() {
    if (this.data.paying) return;
    this.setData({ paying: true });
    setTimeout(() => this.paid(this.data.payWay === 'wx' ? '微信支付（模拟）' : '律安心余额'), 600);
  },
  paid(way) {
    const { l, service, price, cats, catIdx, desc, images, phone } = this.data;
    const svc = this.data.services.find(s => s.id === service);
    const first = [{ from: 'sys', type: 'text', content: `订单已支付 ¥${price}（${way}），${l.name}律师将在 24 小时内为你解答` }, { from: 'me', type: 'text', content: desc.trim() }]
      .concat(images.map(src => ({ from: 'me', type: 'image', content: src })));
    const rec = S.addConsult({
      lawyerId: l.id, lawyerName: l.name, avatar: l.avatar, service: svc.name, cat: cats[catIdx].name,
      question: desc.trim(), amount: price, phone, payWay: way
    });
    first.forEach(m => S.pushMessage(rec.id, m));
    this.setData({ cashier: false, paying: false });
    wx.showToast({ title: '支付成功', icon: 'success' });
    setTimeout(() => wx.redirectTo({ url: '/pages/chat/chat?id=' + rec.id }), 500);
  }
});
