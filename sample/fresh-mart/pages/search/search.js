const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');

const HIS_KEY = 'fm_search_history';

Page({
  data: {
    keyword: '',
    searched: false,
    history: [],
    hot: [
      { word: '车厘子', flag: '爆' },
      { word: '蓝莓', flag: '新' },
      { word: '鲜牛奶', flag: '' },
      { word: '土鸡蛋', flag: '秒杀' },
      { word: '牛腱子', flag: '' },
      { word: '五花肉', flag: '' },
      { word: '番茄', flag: '' },
      { word: '五常大米', flag: '' }
    ],
    sorts: ['综合', '销量', '价格'],
    sortIdx: 0,
    results: []
  },

  onLoad() {
    const history = wx.getStorageSync(HIS_KEY);
    this.setData({ history: history && history.length ? history : ['芒果', '吐司', '椰青水'] });
  },

  onInput(e) {
    const keyword = e.detail.value;
    this.setData({ keyword });
    // 输入即搜；清空回到历史 / 热搜
    if (keyword.trim()) this.search(keyword, false);
    else this.setData({ searched: false });
  },

  onConfirm() {
    const kw = this.data.keyword.trim();
    if (!kw) {
      wx.showToast({ title: '请输入关键词', icon: 'none' });
      return;
    }
    this.search(kw, true);
  },

  onWord(e) {
    const kw = e.currentTarget.dataset.word;
    this.setData({ keyword: kw });
    this.search(kw, true);
  },

  // 关键词逐字匹配商品名 / 分类名 / 产地
  search(kw, remember) {
    const words = kw.trim();
    const cateNames = {};
    data.categories.forEach(c => { cateNames[c.id] = c.name; });
    let results = data.goods.filter(g => {
      const hay = g.name + cateNames[g.cat] + g.place;
      if (hay.indexOf(words) >= 0) return true;
      // 「土鸡蛋」这种多字词：拆成单字，命中 2 个以上算相关
      const hit = words.split('').filter(ch => hay.indexOf(ch) >= 0).length;
      return words.length >= 2 && hit >= Math.min(2, words.length);
    });
    if (this.data.sortIdx === 1) results = results.slice().sort((a, b) => b.sales - a.sales);
    if (this.data.sortIdx === 2) results = results.slice().sort((a, b) => a.price - b.price);
    const patch = { searched: true, results };
    if (remember) {
      const history = [words].concat(this.data.history.filter(h => h !== words)).slice(0, 10);
      wx.setStorageSync(HIS_KEY, history);
      patch.history = history;
    }
    this.setData(patch);
  },

  onSort(e) {
    this.setData({ sortIdx: Number(e.currentTarget.dataset.idx) });
    this.search(this.data.keyword, false);
  },

  onClear() {
    this.setData({ keyword: '', searched: false });
  },

  onClearHistory() {
    wx.showModal({
      title: '提示',
      content: '确定清空搜索历史吗？',
      success: (res) => {
        if (res.confirm) {
          wx.removeStorageSync(HIS_KEY);
          this.setData({ history: [] });
        }
      }
    });
  },

  onDetail(e) {
    wx.navigateTo({ url: `/pages/detail/detail?id=${e.currentTarget.dataset.id}` });
  },

  onAdd(e) {
    const n = cart.add(e.currentTarget.dataset.id, 1);
    wx.showToast({ title: `已加入购物车（${n}）`, icon: 'none' });
  }
});
