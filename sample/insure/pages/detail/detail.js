const { getProduct } = require('../../utils/products.js');
const premium = require('../../utils/premium.js');
const util = require('../../utils/util.js');

function quoteOf(p, q) {
  const v = premium.calc(p, q);
  return Object.assign({}, q, {
    premium: v, premiumText: util.money(v), amountText: util.amountText(q.amount),
    genderText: q.gender === 'F' ? '女' : '男', period: premium.periodText(p)
  });
}

Page({
  data: { p: null, quote: null, fromCalc: false, faqs: [], showCov: false, cov: null, termText: '' },
  onLoad(q) {
    const p = getProduct(q.id || 'p01');
    if (!p) {
      wx.showToast({ title: '产品已下架', icon: 'none' });
      setTimeout(() => wx.navigateBack(), 800);
      return;
    }
    this.p = p;
    wx.setNavigationBarTitle({ title: p.catName });
    const age = Math.min(Math.max(30, p.ages[0]), p.ages[1]);
    this.setData({
      p: {
        id: p.id, name: p.name, cover: p.cover, tagline: p.tagline, tags: p.tags, insurer: p.insurer,
        ages: p.ages[0] + '~' + p.ages[1] + ' 周岁', wait: p.wait ? p.wait + ' 天' : '无等待期',
        coverage: p.coverage, clauses: p.clauses, sales: p.sales
      },
      termText: p.terms.join(' / '),
      faqs: p.faqs.map((f, i) => ({ q: f.q, a: f.a, open: i === 0 })),
      quote: quoteOf(p, { age, gender: 'M', amount: p.defAmount, term: p.defTerm })
    });
  },
  // 保障责任：点一行，半屏 page-container 弹出这条责任的完整说明
  openCov(e) { this.setData({ cov: this.data.p.coverage[e.currentTarget.dataset.i], showCov: true }); },
  closeCov() { this.setData({ showCov: false }); },
  toggleFaq(e) {
    const i = e.currentTarget.dataset.i;
    this.setData({ ['faqs[' + i + '].open']: !this.data.faqs[i].open });
  },
  // 测算页用 EventChannel 双向通信：这边先把当前条件发过去，那边点「用这个结果投保」再把报价发回来
  openCalc() {
    const self = this;
    wx.navigateTo({
      url: '/pages/calc/calc?id=' + this.p.id + '&from=detail',
      events: {
        quote(q) {
          self.setData({ quote: quoteOf(self.p, q), fromCalc: true });
          wx.showToast({ title: '已带回测算结果', icon: 'none' });
        }
      },
      success(res) {
        const q = self.data.quote;
        res.eventChannel.emit('init', { age: q.age, gender: q.gender, amount: q.amount, term: q.term });
      }
    });
  },
  apply() {
    const q = this.data.quote;
    wx.navigateTo({ url: '/pages/apply/apply?id=' + this.p.id + '&amount=' + q.amount + '&term=' + encodeURIComponent(q.term) + '&gender=' + q.gender + '&age=' + q.age });
  },
  onShareAppMessage() { return { title: this.p.name + '｜' + this.p.tagline, path: '/pages/detail/detail?id=' + this.p.id }; }
});
