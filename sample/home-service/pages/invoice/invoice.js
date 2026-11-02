const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const RULES = {
  title: [{ required: true, msg: '请填写发票抬头' }, { min: 2, msg: '抬头至少 2 个字' }, { max: 40, msg: '抬头最多 40 个字' }],
  // 统一社会信用代码 18 位；老的税务登记号 15 / 20 位
  taxNo: [{ required: true, msg: '企业发票请填写税号' }, { pattern: '^([0-9A-Z]{15}|[0-9A-Z]{18}|[0-9A-Z]{20})$', msg: '税号是 15 / 18 / 20 位数字或大写字母' }],
  email: [{ required: true, msg: '请填写接收邮箱' }, { pattern: '^[\\w.+-]+@[\\w-]+(\\.[\\w-]+)+$', msg: '邮箱格式不对' }]
};
function eligible(o) { return (o.status === 'done' || o.status === 'review') && !o.invoiced && o.pay > 0; }
Page({
  data: {
    rules: RULES, orders: [], picked: [], total: 0, history: [],
    form: { type: 'personal', title: '', taxNo: '', email: '' }, single: false
  },
  onLoad(q) {
    this.single = q.id || '';
    const last = db.getInvoices()[0];
    if (last) this.setData({ form: { type: last.type, title: last.title, taxNo: last.taxNo || '', email: last.email } });
    this.load();
  },
  load() {
    let orders = db.getOrders().filter(eligible);
    if (this.single) orders = orders.filter((o) => o.id === this.single);
    const picked = this.single ? orders.map((o) => o.id) : this.data.picked.filter((id) => orders.some((o) => o.id === id));
    this.setData({
      single: !!this.single,
      orders: orders.map((o) => ({ id: o.id, name: o.svcName + ' · ' + o.specName, date: o.date, pay: o.pay, on: picked.indexOf(o.id) >= 0 })),
      picked,
      history: db.getInvoices().map((v) => Object.assign({}, v, { typeText: v.type === 'company' ? '企业' : '个人', n: v.orderIds.length }))
    });
    this.sum();
  },
  sum() {
    const total = this.data.orders.filter((o) => this.data.picked.indexOf(o.id) >= 0).reduce((s, o) => s + o.pay, 0);
    this.setData({ total });
  },
  onPick(e) {
    const picked = e.detail.value;
    this.setData({ picked, orders: this.data.orders.map((o) => Object.assign(o, { on: picked.indexOf(o.id) >= 0 })) });
    this.sum();
  },
  pickAll() {
    const all = this.data.picked.length < this.data.orders.length;
    const picked = all ? this.data.orders.map((o) => o.id) : [];
    this.setData({ picked, orders: this.data.orders.map((o) => Object.assign(o, { on: all })) });
    this.sum();
  },
  onType(e) { this.setData({ 'form.type': e.detail.value }); },
  onInput(e) {
    const k = e.currentTarget.dataset.k;
    const v = k === 'taxNo' ? e.detail.value.toUpperCase() : e.detail.value;
    this.setData({ ['form.' + k]: v });
  },
  submit() {
    if (!this.data.picked.length) { wx.showToast({ title: '请选择要开票的订单', icon: 'none' }); return; }
    // 本引擎没有 relations：用页面当前渲染出来的 form-item 整体同步给 form-group（税号一项随类型增减）
    const g = this.selectComponent('#invForm');
    g.sync(this.selectAllComponents('.fi'));
    const r = g.validate();
    if (!r.ok) { wx.showToast({ title: r.first, icon: 'none' }); return; }
    const f = this.data.form;
    const inv = {
      id: 'FP' + Date.now(), type: f.type, title: f.title.trim(), taxNo: f.type === 'company' ? f.taxNo : '', email: f.email.trim(),
      amount: this.data.total, orderIds: this.data.picked.slice(), t: util.stamp()
    };
    wx.showLoading({ title: '开具中', mask: true });
    setTimeout(() => {
      db.addInvoice(inv);
      wx.hideLoading();
      wx.showModal({
        title: '开票成功', content: '¥' + inv.amount + ' 的电子普通发票已发送至 ' + inv.email + '，一般 5 分钟内到达。',
        showCancel: false,
        success: () => {
          if (this.single) wx.navigateBack();
          else { this.setData({ picked: [] }); this.load(); }
        }
      });
    }, 600);
  }
});
