const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const vip = require('../../utils/vip.js');

const BANNERS = [
  { kicker: '酸菜鱼节 · 限时 3 天', title: '一人食 满 50 减 18', desc: '鱼你在一起 · 巴沙鱼无刺嫩滑', btn: '去抢购', c1: '#2a7fff', c2: '#6a5cff', shop: 's4' },
  { kicker: '下午茶时间', title: '奶茶满 30 减 5', desc: '茶百道 · 杨枝甘露 3 分钟出杯', btn: '马上喝', c1: '#ff6fa2', c2: '#ff9a44', shop: 's5' }
];

Page({
  data: {
    location: '',
    keyword: '',
    // 清空搜索词时 +1：输入框换个 id 重建（用户打过字的 input，setData 改 value 画面不跟）
    kwSeq: 0,
    category: '',
    filterText: '',
    categories: data.categories,
    sorts: ['综合', '销量', '距离', '评分'],
    sortIdx: 0,
    list: data.shops,
    banners: []
  },

  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    const a = store.currentAddr();
    const v = store.getVip();
    const vipBanner = v.active
      ? { kicker: '饱了么 · 超级会员', title: '会员红包 天天可用', desc: `有效期至 ${v.untilText}，续费再得 4 张 ¥5 红包`, btn: '去续费', c1: '#ff7a45', c2: '#ff3d3d', shop: '' }
      : { kicker: '饱了么 · 超级会员', title: '天天领 ¥20 无门槛', desc: '开通即送 4 张 ¥5 红包，首月 ¥1', btn: '立即开通', c1: '#ff7a45', c2: '#ff3d3d', shop: '' };
    this.setData({ location: a ? a.detail : '选择收货地址', banners: [vipBanner].concat(BANNERS) });
  },

  // 搜索 + 分类 + 排序统一在这里算
  refresh() {
    const kw = this.data.keyword.trim();
    const cat = this.data.category;
    let list = data.shops.filter(s => {
      if (cat && s.cats.indexOf(cat) < 0) return false;
      if (!kw || s.name.indexOf(kw) >= 0 || s.cat.indexOf(kw) >= 0) return true;
      return (data.menus[s.id] || []).some(g => g.items.some(d => d.name.indexOf(kw) >= 0));
    });
    const idx = this.data.sortIdx;
    if (idx === 1) list = list.slice().sort((a, b) => b.sales - a.sales);
    if (idx === 2) list = list.slice().sort((a, b) => parseFloat(a.distance) - parseFloat(b.distance));
    if (idx === 3) list = list.slice().sort((a, b) => b.rating - a.rating);
    let filterText = '';
    if (kw) filterText = `${cat ? cat + ' · ' : ''}「${kw}」共找到 ${list.length} 家商家`;
    else if (cat) filterText = `分类：${cat} · 共 ${list.length} 家商家`;
    this.setData({ list, filterText });
  },

  onInput(e) {
    this.setData({ keyword: e.detail.value });
    this.refresh();
  },

  // 点「搜索」/ 键盘上的搜索键：收起键盘，跳到结果列表
  onSearch() {
    if (!this.data.keyword.trim()) {
      wx.showToast({ title: '输入商家或菜品名再搜', icon: 'none' });
      return;
    }
    this.refresh();
    wx.hideKeyboard();
    this.toList();
  },

  toList() {
    wx.pageScrollTo({ selector: '.section-head', duration: 200 });
  },

  onClear() {
    this.setData({ keyword: '', category: '', kwSeq: this.data.kwSeq + 1 });
    this.refresh();
  },

  onSort(e) {
    this.setData({ sortIdx: Number(e.currentTarget.dataset.idx) });
    this.refresh();
  },

  onCategory(e) {
    const name = e.currentTarget.dataset.name;
    this.setData({ category: name === '全部分类' || name === this.data.category ? '' : name });
    this.refresh();
    this.toList();
  },

  onBanner(e) {
    const shop = e.currentTarget.dataset.shop;
    if (shop) {
      wx.navigateTo({ url: `/pages/shop/shop?id=${shop}` });
      return;
    }
    vip.buy().then(ok => { if (ok) this.onShow(); });
  },

  onLocation() {
    wx.navigateTo({ url: '/pages/address/address?select=1' });
  },

  onShop(e) {
    wx.navigateTo({ url: `/pages/shop/shop?id=${e.currentTarget.dataset.id}` });
  }
});
