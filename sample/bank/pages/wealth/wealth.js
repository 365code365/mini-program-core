const S = require('../../utils/store.js');
const M = require('../../utils/money.js');

const GROUPS = [
  { k: 'risk', t: '风险等级', opts: [{ v: 'R1', t: 'R1 低风险' }, { v: 'R2', t: 'R2 中低' }, { v: 'R3', t: 'R3 中风险' }, { v: 'R4', t: 'R4 中高' }] },
  { k: 'type', t: '产品类型', opts: [{ v: '活钱管理', t: '活钱管理' }, { v: '定期理财', t: '定期理财' }, { v: '稳健理财', t: '稳健理财' }, { v: '基金', t: '基金' }] },
  { k: 'term', t: '投资期限', opts: [{ v: 'short', t: '3 个月内' }, { v: 'mid', t: '3~12 个月' }, { v: 'long', t: '1 年以上' }] },
  { k: 'min', t: '起购金额', opts: [{ v: 'low', t: '1 千以下' }, { v: 'midm', t: '1 千~1 万' }, { v: 'high', t: '5 万起' }] }
];
const SORTS = [{ k: 'default', t: '综合' }, { k: 'rate', t: '收益率' }, { k: 'days', t: '期限' }, { k: 'min', t: '起购金额' }];
const DEF_DIR = { rate: -1, days: 1, min: 1 };

function termOf(days) { return days <= 90 ? 'short' : days <= 365 ? 'mid' : 'long'; }
function minOf(v) { return v < 1000 ? 'low' : v <= 10000 ? 'midm' : v >= 50000 ? 'high' : ''; }
function minText(v) { return v >= 10000 ? v / 10000 + '万元' : v + '元'; }

function match(p, f) {
  if (f.risk.length && f.risk.indexOf(p.risk) < 0) return false;
  if (f.type.length && f.type.indexOf(p.type) < 0) return false;
  if (f.term.length && f.term.indexOf(termOf(p.periods[0].days)) < 0) return false;
  if (f.min.length && f.min.indexOf(minOf(p.minBuy)) < 0) return false;
  return true;
}
function empty() { return { risk: [], type: [], term: [], min: [] }; }
function countOf(f) { return f.risk.length + f.type.length + f.term.length + f.min.length; }

Page({
  data: { sorts: SORTS, sortKey: 'default', sortDir: 0, sortDesc: '按热度排序', list: [], panel: false, groups: [], filterCount: 0, draftCount: 0, holdText: '0.00', incomeText: '0.00' },

  onLoad() {
    this.filter = empty();
    this.draft = empty();
    this.refresh();
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    const hs = S.holdings();
    this.setData({ holdText: M.fmt(hs.reduce((s, h) => s + h.amount, 0)), incomeText: M.fmt(hs.reduce((s, h) => s + h.income, 0)) });
  },

  refresh() {
    const { sortKey, sortDir } = this.data;
    const val = p => (sortKey === 'rate' ? Math.max.apply(null, p.periods.map(x => x.rate)) : sortKey === 'days' ? p.periods[0].days : p.minBuy);
    let list = S.PRODUCTS.filter(p => match(p, this.filter));
    if (sortKey !== 'default') list = list.slice().sort((a, b) => (val(a) - val(b)) * sortDir);
    const names = { rate: '收益率', days: '期限', min: '起购金额' };
    this.setData({
      sortDesc: sortKey === 'default' ? '按热度排序' : '按' + names[sortKey] + (sortDir > 0 ? '从低到高' : '从高到低'),
      filterCount: countOf(this.filter),
      list: list.map(p => {
        const rate = Math.max.apply(null, p.periods.map(x => x.rate));
        const days = p.periods[0].days;
        return { id: p.id, name: p.name, tag: p.tag, type: p.type, riskName: p.risk + ' ' + S.RISK_NAME[p.risk], rateText: rate.toFixed(2), days, rateLabel: days === 1 ? '七日年化' : '业绩比较基准', daysText: days === 1 ? '随存随取' : days + '天起', minText: minText(p.minBuy) };
      })
    });
  },

  onSort(e) {
    const k = e.currentTarget.dataset.k;
    let dir = 0;
    if (k !== 'default') dir = this.data.sortKey === k ? -this.data.sortDir : DEF_DIR[k];
    this.setData({ sortKey: k, sortDir: dir, panel: false });
    this.refresh();
    wx.pageScrollTo({ scrollTop: 0, duration: 0 });
  },

  // 面板里改的是草稿，点「查看」才生效；草稿实时数出能看到几款
  buildGroups() {
    const d = this.draft;
    this.setData({
      groups: GROUPS.map(g => ({ k: g.k, t: g.t, opts: g.opts.map(o => ({ v: o.v, t: o.t, checked: d[g.k].indexOf(o.v) >= 0 })) })),
      draftCount: S.PRODUCTS.filter(p => match(p, d)).length
    });
  },
  maskTap(e) { if (e.target.dataset.mask) this.togglePanel(); },
  togglePanel() {
    if (!this.data.panel) {
      this.draft = JSON.parse(JSON.stringify(this.filter));
      this.buildGroups();
    }
    this.setData({ panel: !this.data.panel });
  },
  onCheck(e) { 
    this.draft[e.currentTarget.dataset.g] = e.detail.value;
    this.buildGroups();
  },
  noop() {},
  onOpt(e) {
    const { g, v } = e.currentTarget.dataset;
    const arr = this.draft[g];
    const i = arr.indexOf(v);
    if (i >= 0) arr.splice(i, 1); else arr.push(v);
    this.buildGroups();
  },
  resetDraft() { this.draft = empty(); this.buildGroups(); },
  applyFilter() {
    this.filter = JSON.parse(JSON.stringify(this.draft));
    this.setData({ panel: false });
    this.refresh();
  },
  clearFilter() { this.filter = empty(); this.refresh(); },

  open(e) { wx.navigateTo({ url: '/pages/product/product?id=' + e.currentTarget.dataset.id }); },
  goMine() { wx.switchTab({ url: '/pages/mine/mine' }); }
});
