const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');

function pad(n) {
  return n < 10 ? '0' + n : '' + n;
}

// 瀑布流：每次放到当前更矮的那一列（按估算高度）
function splitColumns(list) {
  const cols = [[], []];
  const h = [0, 0];
  list.forEach((x, i) => {
    const tall = i % 3 === 0;
    const item = Object.assign({}, x, { tall });
    const est = (tall ? 400 : 330) + 230 + (x.name.length > 11 ? 40 : 0);
    const k = h[0] <= h[1] ? 0 : 1;
    cols[k].push(item);
    h[k] += est;
  });
  return cols;
}

Page({
  data: {
    address: '',
    hotWord: '搜索 车厘子',
    categories: data.categories,
    slot: '10:00',
    hh: '00',
    mm: '00',
    ss: '00',
    flashList: [],
    feedTabs: [
      { name: '猜你喜欢', desc: '为你推荐', key: 'all' },
      { name: '新鲜水果', desc: '产地直发', key: 'fruit' },
      { name: '肉蛋水产', desc: '冷链到家', key: 'meat' },
      { name: '乳品烘焙', desc: '每日鲜', key: 'dairy' }
    ],
    feedIdx: 0,
    columns: [[], []]
  },

  onLoad() {
    const flashList = data.flash.map(f => {
      const gd = data.findGoods(f.id);
      return { id: f.id, img: gd.img, price: gd.price, flashPrice: f.price, stock: f.stock };
    });
    this.setData({ flashList, address: getApp().globalData.address });
    this.loadFeed();
    this.hotIdx = 0;
  },

  onShow() {
    this.tick();
    clearInterval(this.timer);
    this.timer = setInterval(() => this.tick(), 1000);
  },

  onHide() {
    clearInterval(this.timer);
  },

  onUnload() {
    clearInterval(this.timer);
  },

  // 每 2 小时一场，倒计时到本场结束
  tick() {
    const now = new Date();
    const startHour = Math.floor(now.getHours() / 2) * 2;
    const end = new Date(now.getFullYear(), now.getMonth(), now.getDate(), startHour + 2, 0, 0).getTime();
    const left = Math.max(0, Math.floor((end - now.getTime()) / 1000));
    const patch = {
      slot: pad(startHour) + ':00',
      hh: pad(Math.floor(left / 3600)),
      mm: pad(Math.floor((left % 3600) / 60)),
      ss: pad(left % 60)
    };
    // 每 5 秒轮换一次搜索框里的热词
    if (now.getSeconds() % 5 === 0) {
      this.hotIdx = ((this.hotIdx || 0) + 1) % data.hotWords.length;
      patch.hotWord = '搜索 ' + data.hotWords[this.hotIdx];
    }
    this.setData(patch);
  },

  loadFeed() {
    const key = this.data.feedTabs[this.data.feedIdx].key;
    let list = data.goods;
    if (key === 'meat') list = list.filter(x => x.cat === 'meat' || x.cat === 'sea');
    else if (key !== 'all') list = list.filter(x => x.cat === key);
    this.setData({ columns: splitColumns(list) });
  },

  onFeedTab(e) {
    this.setData({ feedIdx: Number(e.currentTarget.dataset.idx) });
    this.loadFeed();
  },

  onSearch() {
    wx.navigateTo({ url: '/pages/search/search' });
  },

  onAddress() {
    const list = ['望京SOHO T3 座', '花家地北里 8 号院', '方恒国际中心 A 座'];
    wx.showActionSheet({
      itemList: list,
      success: (res) => {
        getApp().globalData.address = list[res.tapIndex];
        this.setData({ address: list[res.tapIndex] });
      }
    });
  },

  onCate(e) {
    wx.setStorageSync('fm_cate', e.currentTarget.dataset.id);
    wx.switchTab({ url: '/pages/category/category' });
  },

  onFlashMore() {
    wx.showToast({ title: '下一场 12:00 开抢', icon: 'none' });
  },

  onDetail(e) {
    wx.navigateTo({ url: `/pages/detail/detail?id=${e.currentTarget.dataset.id}` });
  },

  onAdd(e) {
    const n = cart.add(e.currentTarget.dataset.id, 1);
    wx.showToast({ title: `已加入购物车（${n}）`, icon: 'none' });
  }
});
