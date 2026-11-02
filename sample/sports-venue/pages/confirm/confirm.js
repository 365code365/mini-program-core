// 确认订单：场地时段回显、联系人校验、支付方式（微信 / 储值 / 次卡），提交前再核一次场地是否被别人抢走
const D = require('../../utils/data.js');
const F = require('../../utils/fmt.js');
const S = require('../../utils/slots.js');
const store = require('../../utils/store.js');
const ORD = require('../../utils/order.js');
const pay = require('../../utils/pay.js');
Page({
  data: { d: null, v: null, dateText: '', hours: 0, methods: [], method: 'wechat', name: '', phone: '', errs: {}, agree: true, submitting: false },
  onLoad() {
    const d = store.draft();
    if (!d) return;
    const v = D.getVenue(d.venueId);
    const c = store.contact();
    this.setData({ d, v, dateText: F.dateText(d.date), hours: d.segs.reduce((a, s) => a + s.hours, 0), name: c.name, phone: c.phone });
    this.buildMethods();
  },
  buildMethods() {
    const c = store.card(), d = this.data.d, v = this.data.v;
    const t = c.times.find((x) => x.sport === v.sport && x.left > 0);
    const methods = [
      { id: 'wechat', name: '微信支付', desc: '支付 ¥' + d.total, ok: true },
      { id: 'balance', name: '储值余额', desc: '余额 ¥' + c.balance + (c.balance >= d.total ? '' : '，不足'), ok: c.balance >= d.total },
      { id: 'times', name: t ? t.name : D.sportName(v.sport) + '次卡', desc: t ? '剩 ' + t.left + ' 次，本单抵 ' + this.data.hours + ' 次' : '暂无可用次卡', ok: !!t && t.left >= this.data.hours }
    ];
    // 默认挑最省事的：次卡够用 > 余额够用 > 微信
    const method = methods[2].ok ? 'times' : (methods[1].ok ? 'balance' : 'wechat');
    this.setData({ methods, method });
  },
  onName(e) { this.setData({ name: e.detail.value, 'errs.name': '' }); },
  onPhone(e) { this.setData({ phone: e.detail.value, 'errs.phone': '' }); },
  pickMethod(e) {
    const m = this.data.methods.find((x) => x.id === e.currentTarget.dataset.id);
    if (!m.ok) { wx.showToast({ title: m.id === 'times' ? '没有足够次数的' + D.sportName(this.data.v.sport) + '次卡' : '余额不足，可去会员卡页充值', icon: 'none' }); return; }
    this.setData({ method: m.id });
  },
  goCard() { wx.switchTab({ url: '/pages/card/card' }); },
  onAgree(e) { this.setData({ agree: e.detail.value.length > 0 }); },
  validate() {
    const errs = {}, name = this.data.name.trim();
    if (!name) errs.name = '请填写联系人';
    else if (name.length < 2) errs.name = '联系人至少 2 个字';
    if (!/^1[3-9]\d{9}$/.test(this.data.phone)) errs.phone = '请填写 11 位手机号，场馆会用它核对预订';
    this.setData({ errs });
    return !errs.name && !errs.phone;
  },
  // 下单前再核一次：别的用户（散列占位）或自己别的订单可能已经占了
  stillFree() {
    const d = this.data.d;
    const g = S.grid(this.data.v, d.date);
    const busy = {};
    g.rows.forEach((r) => r.cells.forEach((c) => { if (c.state !== 'free') busy[c.key] = true; }));
    return d.segs.every((s) => { for (let h = s.start; h < s.end; h++) if (busy[s.court + '-' + h]) return false; return true; });
  },
  submit() {
    if (this.data.submitting) return;
    if (!this.validate()) return;
    if (!this.data.agree) { wx.showToast({ title: '请先阅读并同意订场须知', icon: 'none' }); return; }
    if (!this.stillFree()) {
      wx.showModal({ title: '场地已被预订', content: '你选的时段刚刚被别人订走了，请返回重新选择。', showCancel: false, success: () => wx.navigateBack() });
      return;
    }
    const contact = { name: this.data.name.trim(), phone: this.data.phone };
    store.saveContact(contact);
    const d = this.data.d, m = this.data.method;
    this.setData({ submitting: true });
    wx.showLoading({ title: '锁定场地中', mask: true });
    setTimeout(() => {
      wx.hideLoading();
      const done = (ok) => {
        this.setData({ submitting: false });
        if (!ok) { wx.showToast({ title: '已取消支付，场地保留 15 分钟', icon: 'none' }); return; }
        if (m !== 'wechat') this.deduct(m);
        const o = ORD.create(d, m, contact);
        store.setDraft(null);
        wx.redirectTo({ url: '/pages/result/result?id=' + o.id });
      };
      if (m === 'wechat') pay.wechatPay(d.total, this.data.v.name + ' 订场', done);
      else done(true);
    }, 500);
  },
  deduct(m) {
    const c = store.card(), d = this.data.d, v = this.data.v;
    if (m === 'balance') { c.balance -= d.total; store.addRecord(c, '订场 · ' + v.name, -d.total); }
    if (m === 'times') {
      const t = c.times.find((x) => x.sport === v.sport);
      t.left -= this.data.hours;
      store.addRecord(c, t.name + ' 抵扣 ' + this.data.hours + ' 次 · ' + v.name, 0);
    }
    store.saveCard(c);
  },
  back() { wx.navigateBack(); }
});
