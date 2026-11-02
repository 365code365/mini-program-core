const data = require('../../utils/data.js');
const S = require('../../utils/store.js');
Page({
  data: {
    intent: '前端开发工程师',
    cities: data.CITIES,
    cityIdx: 0,
    keyword: '',
    tabs: ['推荐', '最新', '附近'],
    tabIdx: 0,
    list: [],
    // 筛选面板：经验 / 学历 / 薪资；panel 里是没确定前的草稿
    exps: data.EXPS,
    edus: data.EDUS,
    salaries: data.SALARIES,
    f: { exp: 0, edu: 0, sal: 0 },
    draft: { exp: 0, edu: 0, sal: 0 },
    fCount: 0,
    showFilter: false,
    blockedCount: 0
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    // 求职意向在「我的 → 求职偏好」里也能改，回到首页时重新读
    const it = S.getIntent();
    this.setData({ intent: it.intent, cityIdx: it.cityIdx });
    this.refresh();
  },
  refresh() {
    const { cities, cityIdx, f, tabIdx, intent } = this.data;
    const city = cities[cityIdx];
    const kw = this.data.keyword.trim().toLowerCase();
    const blocked = S.getSettings().blocked;
    let all = data.jobs.map(data.decorate);
    const before = all.length;
    all = all.filter(j => blocked.indexOf(j.company) < 0);
    const blockedCount = before - all.length;
    let list = all;
    if (city !== '全国') list = list.filter(j => j.city === city);
    if (kw) list = list.filter(j => (j.title + j.co.name + j.tags.join(' ')).toLowerCase().indexOf(kw) >= 0);
    if (f.exp) list = list.filter(j => j.exp === data.EXPS[f.exp]);
    if (f.edu) list = list.filter(j => j.edu === data.EDUS[f.edu]);
    if (f.sal) list = list.filter(j => data.matchSalary(j, f.sal));
    // 推荐：与求职意向匹配的排前面，再按热度
    if (tabIdx === 0) list.sort((a, b) => (data.intentScore(b, intent) - data.intentScore(a, intent)) || (b.hot - a.hot));
    else if (tabIdx === 1) list.sort((a, b) => a.posted - b.posted);
    else list.sort((a, b) => a.dist - b.dist);
    const applied = S.getApps().map(a => a.id);
    list = list.map(j => Object.assign({}, j, { applied: applied.indexOf(j.id) >= 0, match: tabIdx === 0 && data.intentScore(j, intent) > 0 }));
    this.setData({ list, blockedCount, fCount: (f.exp ? 1 : 0) + (f.edu ? 1 : 0) + (f.sal ? 1 : 0) });
  },
  onIntent() {
    const items = data.INTENTS;
    wx.showActionSheet({
      itemList: items,
      success: (r) => {
        S.saveIntent({ intent: items[r.tapIndex] });
        this.setData({ intent: items[r.tapIndex], tabIdx: 0 });
        this.refresh();
      }
    });
  },
  onCity(e) {
    const cityIdx = Number(e.detail.value);
    S.saveIntent({ cityIdx });
    this.setData({ cityIdx });
    this.refresh();
  },
  onSearch(e) { this.setData({ keyword: e.detail.value }); this.refresh(); },
  onClear() { this.setData({ keyword: '' }); this.refresh(); },
  onTab(e) { this.setData({ tabIdx: Number(e.currentTarget.dataset.idx) }); this.refresh(); },
  onJob(e) { wx.navigateTo({ url: '/pages/job/job?id=' + e.currentTarget.dataset.id }); },
  onOpenFilter() { this.setData({ showFilter: true, draft: Object.assign({}, this.data.f) }); },
  noop() {},
  onCloseFilter() { this.setData({ showFilter: false }); },
  onPickOpt(e) {
    const k = e.currentTarget.dataset.k;
    this.setData({ ['draft.' + k]: Number(e.currentTarget.dataset.i) });
  },
  onResetFilter() { this.setData({ draft: { exp: 0, edu: 0, sal: 0 } }); },
  onApplyFilter() {
    this.setData({ f: Object.assign({}, this.data.draft), showFilter: false });
    this.refresh();
  },
  onClearFilter() {
    this.setData({ f: { exp: 0, edu: 0, sal: 0 }, keyword: '' });
    this.refresh();
  }
});
