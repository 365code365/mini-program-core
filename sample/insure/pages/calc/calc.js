const { PRODUCTS } = require('../../utils/products.js');
const premium = require('../../utils/premium.js');
const util = require('../../utils/util.js');
const chart = require('../../utils/chart.js');

Page({
  data: {
    pid: '', name: '', ages: [0, 65], age: 30, gender: 'M',
    amounts: [], amount: 0, terms: [], term: '',
    result: '0.00', perDay: '0.00', period: '每年', compare: '',
    fromDetail: false, productNames: [], productIdx: 0
  },
  onLoad(q) {
    this.fromDetail = q.from === 'detail';
    const idx = Math.max(0, PRODUCTS.findIndex(p => p.id === (q.id || 'p01')));
    this.setData({ fromDetail: this.fromDetail, productNames: PRODUCTS.map(p => p.name), productIdx: idx });
    this.useProduct(PRODUCTS[idx]);
    // 从详情页打开时，详情页通过 EventChannel 把当前条件发过来
    const ch = this.getOpenerEventChannel && this.getOpenerEventChannel();
    if (ch && ch.on) {
      ch.on('init', c => {
        const p = this.p;
        this.setData({
          age: Math.min(Math.max(Number(c.age), p.ages[0]), p.ages[1]), gender: c.gender || 'M',
          amount: p.amounts.indexOf(Number(c.amount)) >= 0 ? Number(c.amount) : p.defAmount,
          term: p.terms.indexOf(c.term) >= 0 ? c.term : p.defTerm
        });
        this.recalc();
      });
    }
  },
  onReady() {
    this._ready = true;
    wx.createSelectorQuery().select('#chart').fields({ node: true, size: true }).exec(res => {
      const r = res && res[0];
      if (!r || !r.node) { console.warn('chart canvas node not found'); return; }
      const dpr = wx.getWindowInfo().pixelRatio || 2;
      r.node.width = r.width * dpr;
      r.node.height = r.height * dpr;
      this.ctx = r.node.getContext('2d');
      this.ctx.scale(dpr, dpr);
      this.cw = r.width; this.chh = r.height;
      this.drawChart();
    });
  },
  useProduct(p) {
    this.p = p;
    this.setData({
      pid: p.id, name: p.name, ages: p.ages, age: Math.min(Math.max(30, p.ages[0]), p.ages[1]),
      amounts: p.amounts.map(a => ({ v: a, t: util.amountText(a) })), amount: p.defAmount,
      terms: p.terms, term: p.defTerm, period: premium.periodText(p)
    });
    this.recalc();
  },
  onProduct(e) {
    const idx = Number(e.detail.value);
    this.setData({ productIdx: idx });
    this.useProduct(PRODUCTS[idx]);
  },
  onAgeChanging(e) { this.setData({ age: e.detail.value }); this.recalc(); },
  onAge(e) { this.setData({ age: e.detail.value }); this.recalc(); },
  ageStep(e) {
    const d = Number(e.currentTarget.dataset.d);
    const age = Math.min(Math.max(this.data.age + d, this.p.ages[0]), this.p.ages[1]);
    this.setData({ age });
    this.recalc();
  },
  onGender(e) { this.setData({ gender: e.currentTarget.dataset.v }); this.recalc(); },
  onAmount(e) { this.setData({ amount: Number(e.currentTarget.dataset.v) }); this.recalc(); },
  onTerm(e) { this.setData({ term: e.currentTarget.dataset.v }); this.recalc(); },
  cond() { const d = this.data; return { age: d.age, gender: d.gender, amount: d.amount, term: d.term }; },
  recalc() {
    const p = this.p, c = this.cond();
    const v = premium.calc(p, c);
    const days = p.model === 'travel' ? Number(/\d+/.exec(c.term)[0]) : 365;
    let compare = '';
    if (c.age + 10 <= p.ages[1]) {
      const later = premium.calc(p, Object.assign({}, c, { age: c.age + 10 }));
      if (later > v) compare = '晚 10 年投保（' + (c.age + 10) + ' 岁），每期要多交 ¥' + util.money(later - v);
    }
    this.setData({ result: util.money(v), perDay: util.money(v / days), compare });
    this.drawChart();
  },
  drawChart() {
    if (!this.ctx) return;
    const c = this.cond();
    chart.draw(this.ctx, this.cw, this.chh, premium.series(this.p, c), c.age, premium.calc(this.p, c));
  },
  // 详情页打开的：把结果发回去再返回；其它入口直接去投保
  useResult() {
    const c = this.cond();
    // 详情页打开时锁定产品（不显示产品切换），回传的一定是那款产品的条件
    if (this.fromDetail && this.getOpenerEventChannel) {
      this.getOpenerEventChannel().emit('quote', c);
      wx.navigateBack();
      return;
    }
    wx.navigateTo({ url: '/pages/apply/apply?id=' + this.p.id + '&amount=' + c.amount + '&term=' + encodeURIComponent(c.term) + '&gender=' + c.gender + '&age=' + c.age });
  },
  openDetail() { wx.navigateTo({ url: '/pages/detail/detail?id=' + this.p.id }); }
});
