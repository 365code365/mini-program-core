const { getProduct } = require('../../utils/products.js');
const premium = require('../../utils/premium.js');
const util = require('../../utils/util.js');
const store = require('../../utils/store.js');
const ad = require('../../utils/apply-data.js');

const OCC_DEFAULT = [1, 1];

Page({
  data: {
    p: null, amount: 0, amountText: '', term: '', period: '',
    birthday: '', today: '', gender: 'M', premiumText: '--', age: null,
    occCols: ad.occColumns(OCC_DEFAULT[0]), occIdx: OCC_DEFAULT, occText: '',
    health: [], errors: {}, referral: '', refDialog: false, refInput: '', submitting: false, autoRenew: true, shortTerm: true
  },
  onLoad(q) {
    const p = getProduct(q.id || 'p01');
    this.p = p;
    const now = new Date();
    this.setData({
      p: { id: p.id, name: p.name, insurer: p.insurer, ages: p.ages[0] + '~' + p.ages[1] + ' 周岁', cover: p.cover },
      amount: Number(q.amount) || p.defAmount, amountText: util.amountText(Number(q.amount) || p.defAmount),
      term: q.term ? decodeURIComponent(q.term) : p.defTerm, period: premium.periodText(p),
      gender: q.gender === 'F' ? 'F' : 'M', today: util.ymd(now),
      health: ad.healthFor(p.model), referral: store.profile().referral || '',
      shortTerm: ['medical', 'child', 'outpatient', 'cancer'].indexOf(p.model) >= 0
    });
    this.recalc();
  },
  recalc() {
    const d = this.data;
    if (!d.birthday) { this.setData({ premiumText: '--', age: null }); return; }
    const age = util.ageOf(d.birthday);
    const v = premium.calc(this.p, { age, gender: d.gender, amount: d.amount, term: d.term });
    this.setData({ age, premiumText: v ? util.money(v) : '--' });
  },
  setErr(k, msg) { this.setData({ ['errors.' + k]: msg }); },
  // 身份证：失焦时校验校验位，通过就带出生日与性别
  onIdBlur(e) {
    const v = (e.detail.value || '').trim();
    if (!v) return;
    const r = util.checkIdcard(v);
    if (!r.ok) { this.setErr('idcard', r.msg); return; }
    this.setErr('idcard', '');
    this.setData({ birthday: r.birthday, gender: r.gender, 'errors.birthday': '' });
    this.recalc();
  },
  onBirthday(e) { this.setData({ birthday: e.detail.value, 'errors.birthday': '' }); this.recalc(); },
  onGender(e) { this.setData({ gender: e.detail.value }); this.recalc(); },
  // 职业两级联动：第一列（行业）变了，第二列换成该行业的职业，并回到第一项
  onOccColumn(e) {
    if (e.detail.column !== 0) return;
    const i = e.detail.value;
    this.setData({ occCols: ad.occColumns(i), occIdx: [i, 0] });
  },
  onOcc(e) {
    const idx = e.detail.value.map(Number);
    this.setData({ occIdx: idx, occText: ad.occOf(idx).text, 'errors.occupation': '' });
  },
  onAutoRenew(e) { this.setData({ autoRenew: e.detail.value }); },

  // 推荐码：wx.showModal 的 editable 输入框；拿不到 content（宿主不支持输入框）时退回页内输入弹层
  editReferral() {
    wx.showModal({
      title: '填写推荐码', editable: true, placeholderText: '6 位字母或数字，选填', content: this.data.referral,
      success: res => {
        if (!res.confirm) return;
        if (typeof res.content === 'string') this.applyReferral(res.content);
        else this.setData({ refDialog: true, refInput: this.data.referral });
      }
    });
  },
  onRefInput(e) { this.setData({ refInput: e.detail.value }); },
  refCancel() { this.setData({ refDialog: false }); },
  refOk() { if (this.applyReferral(this.data.refInput)) this.setData({ refDialog: false }); },
  applyReferral(raw) {
    const code = String(raw || '').trim().toUpperCase();
    if (code && !/^[A-Z0-9]{6}$/.test(code)) { wx.showToast({ title: '推荐码为 6 位字母或数字', icon: 'none' }); return false; }
    this.setData({ referral: code });
    store.saveProfile({ referral: code });
    wx.showToast({ title: code ? '推荐码已绑定' : '已清除推荐码', icon: 'none' });
    return true;
  },

  validate(v) {
    const errors = {};
    const p = this.p;
    if (!/^[\u4e00-\u9fa5·]{2,15}$/.test((v.name || '').trim())) errors.name = '请填写 2~15 个汉字的真实姓名';
    const id = util.checkIdcard(v.idcard);
    if (!id.ok) errors.idcard = id.msg;
    if (!v.birthday) errors.birthday = '请选择出生日期';
    else if (id.ok && id.birthday !== v.birthday) errors.birthday = '出生日期与身份证号不一致';
    else {
      const age = util.ageOf(v.birthday);
      if (age < p.ages[0] || age > p.ages[1]) errors.birthday = '本产品投保年龄为 ' + p.ages[0] + '~' + p.ages[1] + ' 周岁，当前 ' + age + ' 岁';
    }
    if (id.ok && id.gender !== v.gender) errors.gender = '性别与身份证号不一致';
    if (!this.data.occText) errors.occupation = '请选择职业';
    else {
      const lv = ad.occOf(v.occupation.map(Number)).level;
      if (lv >= 6 || (p.model === 'accident' && lv >= 5)) errors.occupation = '该职业（' + lv + '类）超出本产品承保范围';
    }
    if (!/^1[3-9]\d{9}$/.test(v.phone || '')) errors.phone = '请填写 11 位手机号';
    const income = parseFloat(v.income);
    if (!(income > 0)) errors.income = '请填写年收入（万元）';
    else if ((p.model === 'life' || p.model === 'critical') && this.data.amount > income * 10000 * 10) {
      errors.income = '保额超过年收入的 10 倍，请降低保额或补充财务证明';
    }
    if (this.data.health.length) {
      const h = v.health || [];
      if (!h.length) errors.health = '请完成健康告知';
      else if (h.indexOf('none') >= 0 && h.length > 1) errors.health = '选了「以上情况均没有」就不能再勾选其它项';
    }
    if (!(v.agree || []).length) errors.agree = '请阅读并同意投保须知与条款';
    return errors;
  },
  onSubmit(e) {
    if (this.data.submitting) return;
    const v = e.detail.value;
    const errors = this.validate(v);
    this.setData({ errors });
    const keys = Object.keys(errors);
    if (keys.length) { wx.showToast({ title: errors[keys[0]], icon: 'none' }); return; }
    const h = v.health || [];
    if (this.data.health.length && h.indexOf('none') < 0) {
      wx.showModal({
        title: '健康告知未通过', content: '您勾选的情况不符合本产品的标准承保条件。可上传近 2 年的病历或体检报告转人工核保，3 个工作日内给出结论。',
        confirmText: '我知道了', showCancel: false
      });
      return;
    }
    const d = this.data, p = this.p;
    const age = util.ageOf(v.birthday);
    const prem = premium.calc(p, { age, gender: v.gender, amount: d.amount, term: d.term });
    const policy = store.addPolicy({
      id: util.serial('TB'), productId: p.id, productName: p.name, insurer: p.insurer, cat: p.cat, model: p.model,
      amount: d.amount, amountText: d.amountText, term: d.term, period: d.period, premium: prem,
      holder: { name: v.name.trim(), idcard: v.idcard.trim().toUpperCase(), birthday: v.birthday, gender: v.gender, phone: v.phone, income: parseFloat(v.income), occupation: ad.occOf(v.occupation.map(Number)).text, age },
      autoRenew: d.shortTerm ? !!v.autoRenew : false, referral: d.referral, status: 'pending', createdAt: Date.now()
    });
    this.setData({ submitting: true });
    this.pay(policy);
  },
  // 先调起微信支付；拿不到支付能力（桌面 / 模拟器直接 fail）就去模拟收银台，用户确认后才算支付成功
  pay(policy) {
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)), nonceStr: Math.random().toString(36).slice(2, 18),
      package: 'prepay_id=wx' + policy.id, signType: 'RSA', paySign: 'mock-sign-' + policy.id,
      success: () => {
        store.markPaid(policy.id, '微信支付');
        wx.redirectTo({ url: '/pages/result/result?id=' + policy.id });
      },
      fail: err => {
        this.setData({ submitting: false });
        if (/cancel/.test(err.errMsg || '')) {
          wx.showToast({ title: '已取消支付，可在保单页继续支付', icon: 'none' });
          wx.redirectTo({ url: '/pages/policy/policy?id=' + policy.id });
        } else {
          wx.redirectTo({ url: '/pages/cashier/cashier?id=' + policy.id });
        }
      }
    });
  },
  onReset() {
    this.setData({
      birthday: '', occIdx: OCC_DEFAULT, occCols: ad.occColumns(OCC_DEFAULT[0]), occText: '', errors: {}, autoRenew: true
    });
    this.recalc();
    wx.showToast({ title: '已清空填写内容', icon: 'none' });
  },
  openTerms() { wx.navigateTo({ url: '/pages/detail/detail?id=' + this.p.id }); }
});
