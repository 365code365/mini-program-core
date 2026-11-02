const D = require('../../utils/data.js');
const H = require('../../utils/html.js');

Page({
  data: { q: null, cat: '', related: [], lawyer: null, helpful: false },
  onLoad(o) {
    const q = D.qa.find(x => x.id === o.id) || D.qa[0];
    const lawyer = D.decorate(D.lawyers.find(l => l.fields.indexOf(q.cat) >= 0) || D.lawyers[0]);
    const helped = wx.getStorageSync('legal_helpful') || [];
    this.setData({
      q, paras: H.paras(q.answer), cat: D.catName(q.cat), lawyer, helpful: helped.indexOf(q.id) >= 0,
      related: D.qa.filter(x => x.id !== q.id).slice(0, 3)
    });
  },
  markHelpful() {
    const helped = wx.getStorageSync('legal_helpful') || [];
    if (helped.indexOf(this.data.q.id) < 0) helped.push(this.data.q.id);
    wx.setStorageSync('legal_helpful', helped);
    this.setData({ helpful: true });
    wx.showToast({ title: '感谢反馈', icon: 'none' });
  },
  copy() { wx.setClipboardData({ data: this.data.q.title + '\n' + this.data.q.answer.replace(/<[^>]+>/g, '') }); },
  goRelated(e) { wx.redirectTo({ url: '/pages/qa/qa?id=' + e.currentTarget.dataset.id }); },
  ask() { wx.navigateTo({ url: '/pages/order/order?lawyer=' + this.data.lawyer.id }); }
});
