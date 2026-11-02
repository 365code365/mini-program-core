const D = require('../../utils/data.js');
const store = require('../../utils/store.js');

const HOUSE = ['一室一厅', '两室一厅', '两室两厅', '三室一厅', '三室两厅', '四室及以上', '复式 / 别墅'];
const BUDGETS = ['10 万以下', '10~20 万', '20~30 万', '30~45 万', '45~60 万', '60~80 万', '80 万以上'];
const SLOTS = ['上午 9:00-12:00', '下午 14:00-18:00', '晚上 19:00-21:00'];

Page({
  data: {
    f: { name: '', phone: '', community: '', area: '', remark: '' },
    houses: HOUSE, houseIdx: 4, budgets: BUDGETS, budgetIdx: 2, customBudget: '',
    slots: SLOTS, slot: 0, date: '', start: '', end: '',
    designer: null, caseTitle: '', agree: true, errs: {}
  },

  onLoad(q) {
    const day = store.DAY;
    const tomorrow = store.ymd(Date.now() + day);
    const user = store.get(store.K.user, {});
    const f = Object.assign({}, this.data.f, { name: user.nick || '', phone: user.phone || '', area: q.area || '' });
    let houseIdx = this.data.houseIdx;
    if (q.houseType) {
      const i = HOUSE.indexOf(decodeURIComponent(q.houseType));
      if (i >= 0) houseIdx = i;
    }
    const c = q.case ? D.caseById(q.case) : null;
    this.setData({
      f, houseIdx, date: tomorrow, start: tomorrow, end: store.ymd(Date.now() + 30 * day),
      designer: q.designer ? D.designer(q.designer) : null,
      caseTitle: c ? c.title : '',
      customBudget: q.budget ? decodeURIComponent(q.budget) : '',
      caseId: q.case || ''
    });
  },

  onInput(e) {
    const k = e.currentTarget.dataset.k;
    const errs = Object.assign({}, this.data.errs);
    delete errs[k];
    this.setData({ ['f.' + k]: e.detail.value, errs });
  },
  onHouse(e) { this.setData({ houseIdx: Number(e.detail.value) }); },
  onBudget(e) { this.setData({ budgetIdx: Number(e.detail.value), customBudget: '' }); },
  onDate(e) { this.setData({ date: e.detail.value }); },
  onSlot(e) { this.setData({ slot: Number(e.detail.value) }); },
  onAgree(e) { this.setData({ agree: e.detail.value.length > 0 }); },
  pickDesigner() {
    const list = D.DESIGNERS;
    wx.showActionSheet({
      itemList: ['由平台推荐'].concat(list.map((d) => d.name + ' · ' + d.styles[0])),
      success: (r) => this.setData({ designer: r.tapIndex === 0 ? null : list[r.tapIndex - 1] })
    });
  },

  validate() {
    const f = this.data.f, errs = {};
    if (!f.name.trim()) errs.name = '请填写怎么称呼您';
    if (!/^1[3-9]\d{9}$/.test(f.phone.trim())) errs.phone = '请输入 11 位手机号';
    if (f.community.trim().length < 2) errs.community = '请填写小区名称，方便设计师上门';
    const a = Number(f.area);
    if (!f.area || isNaN(a) || a < 20 || a > 1000) errs.area = '面积请填 20~1000 之间的数字';
    if (!this.data.agree) errs.agree = '请先阅读并同意《量房服务说明》';
    this.setData({ errs });
    return !Object.keys(errs).length;
  },

  submit() {
    if (!this.validate()) return wx.showToast({ title: '还有信息没填好', icon: 'none' });
    const d = this.data, f = d.f, now = Date.now();
    const p = {
      id: 'p' + now.toString(36), no: store.orderNo(now), name: f.name.trim(), phone: f.phone.trim(), community: f.community.trim(),
      area: Number(f.area), houseType: HOUSE[d.houseIdx], date: d.date, slot: SLOTS[d.slot],
      budget: d.customBudget || BUDGETS[d.budgetIdx], designer: d.designer ? d.designer.id : '', caseId: d.caseId || '',
      remark: f.remark.trim(), stage: 0, createdAt: now,
      logs: [{ stage: 0, ts: now, text: '提交预约，等待设计师确认上门时间' }], diaries: []
    };
    store.set(store.K.user, { nick: p.name, phone: p.phone });
    wx.showLoading({ title: '提交中', mask: true });
    setTimeout(() => {
      store.saveProject(p);
      wx.hideLoading();
      wx.redirectTo({ url: '/pages/progress/progress?id=' + p.id + '&new=1' });
    }, 600);
  },

  showTerms() {
    wx.showModal({
      title: '量房服务说明',
      content: '1. 量房免费，不收取任何费用；2. 设计师会在上门前 1 天电话确认；3. 您的手机号仅用于本次预约联系。',
      showCancel: false, confirmText: '我知道了'
    });
  }
});
