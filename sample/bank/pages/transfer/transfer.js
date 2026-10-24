const S = require('../../utils/store.js');
const M = require('../../utils/money.js');

const LIMIT = 50000;
const NAME_RE = /^[\u4e00-\u9fa5·a-zA-Z]{2,20}$/;

Page({
  data: {
    accounts: [], acctNames: [], acctIdx: 0, acct: {},
    banks: S.BANKS, bankIdx: -1, bankPick: 0, bankName: '',
    name: '', card: '', cardDigits: '', cardTail: '', amount: '', amountView: '', upper: '', remark: '',
    recent: [], quick: ['500', '1000', '5000'],
    errors: {}, canSubmit: false, locked: false,
    showPad: false, pwd: '', dots: [0, 0, 0, 0, 0, 0], pwdErrors: 0, shake: false,
    keys: ['1', '2', '3', '4', '5', '6', '7', '8', '9', '', '0', 'del']
  },

  onLoad(options) {
    const accounts = S.accounts();
    const acctIdx = Math.min(Number(options.acct || 0), accounts.length - 1);
    this.setData({ accounts, acctNames: accounts.map(a => a.name + '（' + a.no.slice(-4) + '）'), acctIdx });
    this.applyAcct(acctIdx);
    // 从「收款人」页直接点进来：带着通讯录下标
    if (options.pi !== undefined && S.PAYEES[Number(options.pi)]) this.applyPayee(S.PAYEES[Number(options.pi)]);
  },

  onShow() {
    const pwd = S.pwdState();
    const recent = S.payees().map(p => Object.assign({}, p, { tail: p.card.slice(-4) }));
    this.setData({ locked: !!pwd.locked, pwdErrors: pwd.errors || 0, recent });
    // EventChannel 之外再走一次 storage：两条都到也只是重复填同一个人
    const picked = S.get('picked', null);
    if (picked) { wx.removeStorageSync('bank_picked'); this.applyPayee(picked); }
  },

  applyAcct(i) {
    const a = this.data.accounts[i];
    this.setData({ acctIdx: i, acct: { name: a.name, tail: a.no.slice(-4), bg: a.bg, balance: a.balance, balText: M.fmt(a.balance) } });
    this.check();
  },
  onAcct(e) { this.applyAcct(Number(e.detail.value)); },

  applyPayee(p) {
    const bankIdx = S.BANKS.indexOf(p.bank);
    const digits = p.card.replace(/\D/g, '');
    this.setData({
      name: p.name, card: M.formatCard(digits), cardDigits: digits, cardTail: digits.slice(-4),
      bankIdx, bankPick: bankIdx < 0 ? 0 : bankIdx, bankName: bankIdx < 0 ? '' : S.BANKS[bankIdx],
      'errors.name': '', 'errors.card': '', 'errors.bank': ''
    });
    this.check();
  },

  openPayees() {
    wx.navigateTo({
      url: '/pages/payees/payees?mode=pick',
      events: { pick: p => this.applyPayee(p) }
    });
  },
  useRecent(e) { this.applyPayee(this.data.recent[e.currentTarget.dataset.i]); },

  onName(e) {
    this.setData({ name: e.detail.value, 'errors.name': '' });
    this.check();
  },
  onCard(e) {
    // 受控输入：每 4 位插一个空格，返回值替换输入框内容
    const card = M.formatCard(e.detail.value);
    const digits = card.replace(/\s/g, '');
    this.setData({ card, cardDigits: digits, cardTail: digits.slice(-4), 'errors.card': '' });
    this.check();
    return card;
  },
  onBank(e) {
    const i = Number(e.detail.value);
    this.setData({ bankIdx: i, bankPick: i, bankName: S.BANKS[i], 'errors.bank': '' });
    this.check();
  },
  setAmount(raw) {
    const amount = M.cleanAmount(String(raw).replace(/,/g, ''));
    const amountView = M.fmtTyping(amount);
    this.setData({ amount, amountView, upper: M.upper(amount), 'errors.amount': '' });
    this.check();
    return amountView;
  },
  onAmount(e) { return this.setAmount(e.detail.value); },
  quickAmount(e) { this.setAmount(e.currentTarget.dataset.v); },
  allAmount() { this.setAmount(Math.min(this.data.acct.balance, LIMIT).toFixed(2)); },
  onRemark(e) { this.setData({ remark: e.detail.value }); },
  clearField(e) {
    const f = e.currentTarget.dataset.f;
    if (f === 'amount') return this.setAmount('');
    if (f === 'card') this.setData({ card: '', cardDigits: '', cardTail: '' });
    else this.setData({ [f]: '' });
    this.check();
  },

  check() {
    const d = this.data;
    this.setData({ canSubmit: !!(d.name && d.cardDigits.length >= 16 && d.bankIdx >= 0 && Number(d.amount) > 0) });
  },

  validate() {
    const d = this.data;
    const errors = {};
    if (!d.name) errors.name = '请填写收款人户名';
    else if (!NAME_RE.test(d.name)) errors.name = '户名应为 2~20 位中文或字母';
    if (!d.cardDigits) errors.card = '请填写收款卡号';
    else if (d.cardDigits.length < 16) errors.card = '卡号应为 16~19 位数字，当前 ' + d.cardDigits.length + ' 位';
    else if (d.cardDigits === S.accounts()[d.acctIdx].no) errors.card = '不能转给付款账户本身';
    if (d.bankIdx < 0) errors.bank = '请选择开户银行';
    const amt = Number(d.amount);
    if (!(amt > 0)) errors.amount = '请输入转账金额';
    else if (amt > LIMIT) errors.amount = '超过单笔限额 50,000.00 元';
    else if (amt > d.acct.balance) errors.amount = '余额不足，可用 ¥' + d.acct.balText;
    this.setData({ errors });
    return Object.keys(errors).map(k => errors[k]);
  },

  submit() {
    if (this.data.locked) {
      return wx.showModal({ title: '支付密码已锁定', content: '连续输错 3 次，请到「我的 - 安全中心」解除锁定后再试', showCancel: false, confirmText: '知道了' });
    }
    const errs = this.validate();
    if (errs.length) return wx.showToast({ title: errs[0], icon: 'none' });
    const d = this.data;
    wx.showModal({
      title: '确认转账',
      content: '向 ' + d.name + '（' + d.bankName + ' 尾号 ' + d.cardTail + '）转账 ¥' + M.fmt(d.amount) + '，大写：' + d.upper,
      confirmText: '确认转账',
      confirmColor: '#D7263D',
      success: r => {
        if (!r.confirm) return;
        // 「我的」里开了小额免密：1000 元以内不弹密码键盘
        if (S.get('nopwd', false) && Number(d.amount) <= 1000) this.doTransfer();
        else this.openPad();
      }
    });
  },

  openPad() { this.setData({ showPad: true, pwd: '', dots: [0, 0, 0, 0, 0, 0], shake: false }); },
  noop() {},
  closePad() { this.setData({ showPad: false, pwd: '', dots: [0, 0, 0, 0, 0, 0] }); },

  press(e) {
    const k = e.currentTarget.dataset.k;
    if (k === '' || this.verifying) return;
    let pwd = this.data.pwd;
    if (k === 'del') pwd = pwd.slice(0, -1);
    else if (pwd.length < 6) pwd += k;
    this.setData({ pwd, dots: [0, 1, 2, 3, 4, 5].map(i => (i < pwd.length ? 1 : 0)) });
    if (pwd.length === 6) {
      this.verifying = true;
      setTimeout(() => { this.verifying = false; this.verify(pwd); }, 250);
    }
  },

  verify(pwd) {
    if (pwd === S.PWD) {
      S.set('pwd', { errors: 0, locked: false });
      this.setData({ showPad: false, pwdErrors: 0 });
      return this.doTransfer();
    }
    const errors = (this.data.pwdErrors || 0) + 1;
    const locked = errors >= 3;
    S.set('pwd', { errors, locked });
    if (locked) {
      this.setData({ showPad: false, pwdErrors: errors, locked: true, pwd: '', dots: [0, 0, 0, 0, 0, 0] });
      return wx.showModal({ title: '支付密码已锁定', content: '支付密码连续输错 3 次，已锁定。请到「我的 - 安全中心」解除锁定', showCancel: false, confirmText: '知道了' });
    }
    this.setData({ pwdErrors: errors, pwd: '', dots: [0, 0, 0, 0, 0, 0], shake: true });
    wx.vibrateShort && wx.vibrateShort({ type: 'heavy' });
    setTimeout(() => this.setData({ shake: false }), 450);
  },

  doTransfer() {
    wx.showLoading({ title: '转账中', mask: true });
    setTimeout(() => {
      const d = this.data;
      const amt = Math.round(Number(d.amount) * 100) / 100;
      const accounts = S.accounts();
      accounts[d.acctIdx].balance = Math.round((accounts[d.acctIdx].balance - amt) * 100) / 100;
      S.set('accounts', accounts);
      const id = 'b' + Date.now();
      S.addBill({
        id, acct: accounts[d.acctIdx].id, type: 'out', title: '转账 · ' + d.name, cat: '转账', amount: amt, time: Date.now(),
        counter: d.name + ' ' + M.maskCard(d.cardDigits), bank: d.bankName, remark: d.remark
      });
      const rp = S.payees().filter(p => p.card !== d.cardDigits);
      rp.unshift({ name: d.name, card: d.cardDigits, bank: d.bankName });
      S.set('recentPayees', rp.slice(0, 6));
      wx.hideLoading();
      wx.redirectTo({ url: '/pages/result/result?id=' + id });
    }, 600);
  }
});
