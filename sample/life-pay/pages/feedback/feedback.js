const store = require('../../utils/store.js');
const KINDS = ['缴费未到账', '户号问题', '发票问题', '功能建议', '其他'];
Page({
  data: { kinds: KINDS, kind: '', text: '', contact: '138****6621', list: [], formSeq: 0 },
  onLoad() { this.refresh(); },
  refresh() {
    const v = wx.getStorageSync('lp_feedback');
    this.setData({ list: Array.isArray(v) ? v : [] });
  },
  onKind(e) { this.setData({ kind: e.currentTarget.dataset.k }); },
  onText(e) { this.setData({ text: e.detail.value }); },
  onContact(e) { this.setData({ contact: e.detail.value.trim() }); },
  submit() {
    const d = this.data;
    if (!d.kind) { wx.showToast({ title: '请选择问题类型', icon: 'none' }); return; }
    if (d.text.trim().length < 5) { wx.showToast({ title: '请至少写 5 个字描述问题', icon: 'none' }); return; }
    const list = [{ id: 'f' + Date.now(), kind: d.kind, text: d.text.trim(), contact: d.contact, time: store.stamp(new Date()), status: '已受理' }].concat(d.list);
    wx.setStorageSync('lp_feedback', list);
    this.setData({ kind: '', text: '', formSeq: d.formSeq + 1 });
    this.refresh();
    wx.showToast({ title: '已提交，1 个工作日内回复', icon: 'none' });
  }
});
