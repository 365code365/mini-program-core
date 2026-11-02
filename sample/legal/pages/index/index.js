const D = require('../../utils/data.js');
const S = require('../../utils/store.js');
const app = getApp();

Page({
  data: {
    cats: D.categories,
    qa: D.qa.slice(0, 4),
    lawyers: [],
    active: 0,
    disclaimer: ''
  },
  onLoad() {
    const top = D.lawyers.slice().sort((a, b) => b.rating - a.rating || b.consults - a.consults).slice(0, 3).map(D.decorate);
    this.setData({ lawyers: top, disclaimer: app.globalData.disclaimer });
  },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 0 });
    const active = S.consults().filter(c => c.status === 'active').length;
    this.setData({ active });
  },
  goSearch() { wx.switchTab({ url: '/pages/lawyers/lawyers' }); },
  goCat(e) {
    app.globalData.lawyerCat = e.currentTarget.dataset.id;
    wx.switchTab({ url: '/pages/lawyers/lawyers' });
  },
  goQa(e) { wx.navigateTo({ url: '/pages/qa/qa?id=' + e.currentTarget.dataset.id }); },
  goLawyer(e) { wx.navigateTo({ url: '/pages/lawyer/lawyer?id=' + e.currentTarget.dataset.id }); },
  goContracts() { wx.switchTab({ url: '/pages/contracts/contracts' }); },
  goCalc() { wx.navigateTo({ url: '/pages/calc/calc' }); },
  goConsults() { wx.navigateTo({ url: '/pages/consults/consults' }); },
  quickAsk() {
    // 快速咨询：系统按评分派单给第一位律师
    wx.navigateTo({ url: '/pages/order/order?lawyer=' + this.data.lawyers[0].id });
  }
});
