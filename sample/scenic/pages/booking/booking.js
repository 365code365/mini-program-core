// 门票预订：入园日期（picker date，限今天起 30 天内）+ 票种步进器 + 实名游客（一票一证）+ 联系电话 → 支付 → 结果页
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const D = require('../../utils/date.js');
const idc = require('../../utils/idcard.js');
const MAX = 5;

Page({
  data: {
    s: null, start: '', end: '', date: '', dateText: '', quick: [], tickets: [], count: 0, total: 0,
    tourists: [], picked: [], phone: '', agree: false, errors: {}, submitting: false
  },
  onLoad(q) {
    const s = data.find(q && q.id);
    const t = D.today(), end = D.addDays(t, 29);
    const tickets = s.tickets.map((x) => Object.assign({}, x, { n: x.id === (q && q.t) ? 1 : 0 }));
    if (!tickets.some((x) => x.n)) tickets[0].n = 1;
    const quick = [0, 1, 2].map((i) => { const d = D.addDays(t, i); return { d, label: ['今天', '明天', '后天'][i], sub: D.short(d) }; });
    this.setData({ s, start: t, end, quick, tickets });
    this.setDate(t);
    this.calc();
  },
  onShow() {
    // 从「常用游客」页新增回来时刷新；已勾选的保留
    const picked = this.data.picked;
    const list = store.tourists().map((x) => Object.assign({}, x, { idMask: idc.mask(x.idcard), age: idc.check(x.idcard).age }));
    const keep = picked.filter((id) => list.some((x) => x.id === id));
    if (!picked.length && list.length) keep.push(list[0].id);
    this.setData({ tourists: list.map((x) => Object.assign(x, { on: keep.indexOf(x.id) >= 0 })), picked: keep });
    if (!this.data.phone && list.length) this.setData({ phone: list[0].phone });
  },
  setDate(d) {
    const closed = this.data.s.id === 'gugong' && D.parse(d).getDay() === 1;
    this.setData({ date: d, dateText: d + ' ' + D.weekday(d), 'errors.date': closed ? '故宫周一闭馆（法定节假日除外），请换一天' : '' });
  },
  onQuick(e) { this.setDate(e.currentTarget.dataset.d); },
  onDate(e) { this.setDate(e.detail.value); },
  onStep(e) {
    const { key, value } = e.detail;
    const tickets = this.data.tickets.map((x) => (x.id === key ? Object.assign({}, x, { n: value }) : x));
    const sum = tickets.reduce((a, x) => a + x.n, 0);
    if (sum > MAX) { wx.showToast({ title: '一笔订单最多 ' + MAX + ' 张', icon: 'none' }); return; }
    this.setData({ tickets });
    this.calc();
  },
  calc() {
    const count = this.data.tickets.reduce((a, x) => a + x.n, 0);
    const total = this.data.tickets.reduce((a, x) => a + x.n * x.price, 0);
    this.setData({ count, total });
  },
  onPick(e) {
    const picked = e.detail.value;
    this.setData({ picked, tourists: this.data.tourists.map((x) => Object.assign({}, x, { on: picked.indexOf(x.id) >= 0 })), 'errors.people': '' });
  },
  onPhone(e) { this.setData({ phone: e.detail.value, 'errors.phone': '' }); },
  onAgree(e) { this.setData({ agree: e.detail.value.length > 0 }); },
  goTourists() { wx.navigateTo({ url: '/pages/tourists/tourists?pick=1' }); },
  validate() {
    const errors = {};
    if (this.data.errors.date) errors.date = this.data.errors.date;
    if (!this.data.count) errors.tickets = '请至少选择 1 张门票';
    const ppl = this.data.tourists.filter((x) => x.on);
    if (ppl.length !== this.data.count) errors.people = '实名制入园：需选择 ' + this.data.count + ' 位游客（已选 ' + ppl.length + ' 位）';
    const child = this.data.tickets.filter((x) => x.id === 'child').reduce((a, x) => a + x.n, 0);
    const senior = this.data.tickets.filter((x) => x.id === 'senior').reduce((a, x) => a + x.n, 0);
    if (!errors.people && child > ppl.filter((x) => x.age < 18).length) errors.people = '儿童票需对应 18 周岁以下的游客';
    if (!errors.people && senior > ppl.filter((x) => x.age >= 60).length) errors.people = '老人票需对应 60 周岁以上的游客';
    if (!/^1[3-9]\d{9}$/.test(this.data.phone)) errors.phone = '请填写正确的 11 位手机号，用于接收入园短信';
    this.setData({ errors });
    return Object.keys(errors).length === 0;
  },
  onSubmit() {
    if (this.data.submitting) return;
    if (!this.validate()) { wx.showToast({ title: '请按提示修改', icon: 'none' }); return; }
    if (!this.data.agree) {
      wx.showModal({ title: '预订须知', content: this.data.s.notice.join('\n'), confirmText: '同意并支付', success: (r) => {
        if (r.confirm) { this.setData({ agree: true }); this.pay(); }
      } });
      return;
    }
    this.pay();
  },
  pay() {
    this.setData({ submitting: true });
    const total = this.data.total;
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)), nonceStr: Math.random().toString(36).slice(2, 18),
      package: 'prepay_id=wx' + Date.now(), signType: 'RSA', paySign: 'demo',
      success: () => this.finish('wxpay'),
      fail: (err) => {
        // 桌面 / 模拟器拉不起微信支付（fail 里分不清取消与不支持）：一律给模拟收银台，用户再确认一次才算付款
        console.log('requestPayment fail:', err && err.errMsg);
        wx.showModal({
          title: '模拟收银台', content: '当前环境无法调起微信支付。确认用模拟支付完成 ¥' + total + ' 吗？', confirmText: '确认支付',
          success: (r) => { if (r.confirm) this.finish('mock'); else { this.setData({ submitting: false }); wx.showToast({ title: '已取消支付', icon: 'none' }); } }
        });
      }
    });
  },
  finish(payWay) {
    const s = this.data.s;
    const people = this.data.tourists.filter((x) => x.on).map((x) => ({ name: x.name, idMask: x.idMask, idTail: x.idcard.slice(-4) }));
    const o = store.createOrder({
      scenic: s.id, scenicName: s.name, cover: s.cover, date: this.data.date, payWay,
      items: this.data.tickets.filter((x) => x.n).map((x) => ({ id: x.id, name: x.name, price: x.price, n: x.n })),
      people, phone: this.data.phone, total: this.data.total, count: this.data.count
    });
    this.setData({ submitting: false });
    wx.redirectTo({ url: '/pages/pay-result/pay-result?no=' + o.no });
  }
});
