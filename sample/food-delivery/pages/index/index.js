const data = require('../../utils/data.js');

Page({
  data: {
    location: '望京SOHO T3',
    keyword: '',
    filterText: '',
    categories: data.categories,
    sorts: ['综合', '销量', '距离', '评分'],
    sortIdx: 0,
    list: data.shops,
    banners: [
      { kicker: '饱了么 · 超级会员', title: '天天领 ¥20 无门槛', desc: '开通即送 4 张红包，首月 ¥1', btn: '立即开通', c1: '#ff7a45', c2: '#ff3d3d', shop: '' },
      { kicker: '酸菜鱼节 · 限时 3 天', title: '一人食 满 50 减 18', desc: '鱼你在一起 · 巴沙鱼无刺嫩滑', btn: '去抢购', c1: '#2a7fff', c2: '#6a5cff', shop: 's4' },
      { kicker: '下午茶时间', title: '奶茶第二杯半价', desc: '茶百道 · 杨枝甘露 3 分钟出杯', btn: '马上喝', c1: '#ff6fa2', c2: '#ff9a44', shop: 's5' }
    ]
  },

  onLoad() {
    const app = getApp();
    this.setData({ location: app.globalData.location });
  },

  // 搜索 + 分类 + 排序统一在这里算
  refresh() {
    const kw = this.data.keyword.trim();
    const cat = this.category || '';
    let list = data.shops.filter(s => {
      if (kw && s.name.indexOf(kw) < 0 && s.cat.indexOf(kw) < 0) {
        const menu = data.menus[s.id] || [];
        const hit = menu.some(g => g.items.some(d => d.name.indexOf(kw) >= 0));
        if (!hit) return false;
      }
      if (cat && cat !== '美食' && cat !== '全部分类') {
        const map = { '甜品饮品': '奶茶饮品', '汉堡披萨': '小吃', '夜宵': '川湘菜' };
        if (map[cat] && s.cat !== map[cat]) return false;
      }
      return true;
    });
    const idx = this.data.sortIdx;
    if (idx === 1) list = list.slice().sort((a, b) => b.sales - a.sales);
    if (idx === 2) list = list.slice().sort((a, b) => parseFloat(a.distance) - parseFloat(b.distance));
    if (idx === 3) list = list.slice().sort((a, b) => b.rating - a.rating);
    let filterText = '';
    if (kw) filterText = `「${kw}」共找到 ${list.length} 家商家`;
    else if (cat && cat !== '全部分类') filterText = `分类：${cat}`;
    this.setData({ list, filterText });
  },

  onInput(e) {
    this.setData({ keyword: e.detail.value });
    this.refresh();
  },

  onClear() {
    this.category = '';
    this.setData({ keyword: '' });
    this.refresh();
  },

  onSort(e) {
    this.setData({ sortIdx: Number(e.currentTarget.dataset.idx) });
    this.refresh();
  },

  onCategory(e) {
    const name = e.currentTarget.dataset.name;
    if (name === '看病买药' || name === '浪漫鲜花' || name === '跑腿代购' || name === '超市便利' || name === '蔬菜水果') {
      wx.showToast({ title: `${name} 即将开放`, icon: 'none' });
      return;
    }
    this.category = name;
    this.refresh();
  },

  onBanner(e) {
    const shop = e.currentTarget.dataset.shop;
    if (shop) wx.navigateTo({ url: `/pages/shop/shop?id=${shop}` });
    else wx.showModal({
      title: '超级会员',
      content: '首月 ¥1 开通，每月 4 张 ¥5 无门槛红包。确认开通？',
      confirmText: '开通',
      success: (res) => {
        if (res.confirm) wx.showToast({ title: '开通成功', icon: 'success' });
      }
    });
  },

  onLocation() {
    const opts = ['望京SOHO T3', '花家地北里 8 号院', '北京工业大学'];
    wx.showActionSheet({
      itemList: opts,
      success: (res) => {
        const location = opts[res.tapIndex];
        getApp().globalData.location = location;
        this.setData({ location });
        wx.showToast({ title: '已切换收货地址', icon: 'none' });
      }
    });
  },

  onShop(e) {
    wx.navigateTo({ url: `/pages/shop/shop?id=${e.currentTarget.dataset.id}` });
  }
});
