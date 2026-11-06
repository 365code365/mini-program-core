// 搜索：按药名 / 品牌 / 适应症匹配；输入 300ms 防抖出结果，回车或点热词记入历史（最多 10 条，存 storage）
const { DRUGS, BY_ID, HOT, card } = require('../../utils/drugs.js');
const S = require('../../utils/store.js');

// 口语症状词 → 说明书里的写法
const ALIAS = { '退烧': '发热', '发烧': '发热', '拉肚子': '腹泻', '胃疼': '胃', '胃痛': '胃', '嗓子疼': '咽', '降压': '高血压', '降糖': '糖尿病', '补钙': '钙' };

Page({
  // referrer：被别的小程序打开（健康挂号的处方「去药房」）时顶上一条来源 + 返回；带着药名来的不弹键盘
  data: { kw: '', hot: HOT, history: [], results: [], searched: false, sort: 'default', referrer: null, focus: true },
  onLoad(q) {
    this.setData({ history: S.get('search', []) });
    // 药名在 referrerInfo.extraData（不在 query 里，中文不用编解码）。只在小程序被打开时的第一页认
    const ref = wx.getEnterOptionsSync().referrerInfo || {};
    const ex = ref.extraData || {};
    if (ref.appId && getCurrentPages().length === 1) {
      this.setData({ referrer: { name: ex.fromName || '上一个小程序', tip: ex.drug ? '处方：' + ex.drug : '' }, focus: !ex.kw });
      if (ex.kw) return this.run(String(ex.kw), false);
    }
    if (q.kw) this.run(decodeURIComponent(q.kw), true);
  },
  // 回到打开它的小程序（微信：只有被别的小程序打开时才回得去）
  onBackReferrer() {
    wx.navigateBackMiniProgram({
      extraData: { from: 'pharmacy', kw: this.data.kw },
      fail: () => wx.navigateBack({ fail: () => wx.switchTab({ url: '/pages/index/index' }) })
    });
  },
  onInput(e) {
    const kw = e.detail.value;
    this.setData({ kw });
    clearTimeout(this.t);
    this.t = setTimeout(() => this.run(kw, false), 300);
  },
  onConfirm(e) { this.run(e.detail.value, true); },
  onHot(e) { this.run(e.currentTarget.dataset.k, true); },
  run(kw, remember) {
    kw = (kw || '').trim();
    if (!kw) { this.setData({ kw: '', results: [], searched: false }); return; }
    const key = ALIAS[kw] || kw;
    let results = DRUGS.filter(d => (d.name + d.brand + d.ind + d.comp).indexOf(key) >= 0).map(card);
    if (this.data.sort === 'price') results.sort((a, b) => a.specs[0].p - b.specs[0].p);
    else if (this.data.sort === 'sales') results.sort((a, b) => b.sales - a.sales);
    this.setData({ kw, results, searched: true });
    if (remember) {
      const h = [kw].concat(this.data.history.filter(x => x !== kw)).slice(0, 10);
      S.set('search', h);
      this.setData({ history: h });
    }
  },
  onSort(e) { this.setData({ sort: e.currentTarget.dataset.s }); this.run(this.data.kw, false); },
  clear() { this.setData({ kw: '', results: [], searched: false }); },
  clearHistory() {
    wx.showModal({
      title: '清空搜索历史', content: '确定清空全部搜索历史吗？', confirmText: '清空',
      success: r => { if (r.confirm) { S.set('search', []); this.setData({ history: [] }); } }
    });
  },
  openDrug(e) { wx.navigateTo({ url: '/pages/drug/drug?id=' + e.currentTarget.dataset.id }); },
  addCart(e) {
    const id = e.currentTarget.dataset.id;
    S.addCart(id, 0, 1);
    wx.showToast({ title: BY_ID[id].type === 'RX' ? '已加入，结算前需审方' : '已加入购物车', icon: 'none' });
  }
});
