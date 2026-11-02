const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

Page({
  data: {
    address: '',
    list: [],
    allChecked: false,
    checkedCount: 0,
    total: 0,
    saved: 0,
    freeText: '',
    freeGap: 0,
    editing: false,
    recs: [],
    tabGap: 0
  },

  onLoad() {
    const info = wx.getWindowInfo();
    // 兼容：本引擎 tab 页的 windowHeight 不扣 tabBar（微信会扣），fixed 的 bottom:0 会落到 tabBar 底下
    this.setData({ tabGap: info.screenHeight - info.windowHeight > 90 ? 0 : 50 });
  },

  onShow() {
    const a = store.currentAddr();
    this.setData({ address: a ? a.detail : '请添加收货地址' });
    this.refresh();
  },

  refresh() {
    const raw = store.getCart();
    const list = raw.map(store.expand);
    const on = list.filter(x => x.checked);
    const total = store.round2(on.reduce((n, x) => n + x.total, 0));
    const origin = store.round2(on.reduce((n, x) => n + (x.origin + (x.unit - (data.flashOf(x.id) ? data.flashOf(x.id).price : data.findGoods(x.id).price))) * x.qty, 0));
    const freeGap = store.round2(store.FREE_LINE - total);
    const inCart = raw.map(x => x.id);
    const recs = data.goods.filter(g => inCart.indexOf(g.id) < 0).sort((a, b) => b.sales - a.sales).slice(0, 4);
    this.setData({
      list,
      total,
      saved: store.round2(origin - total),
      checkedCount: on.reduce((n, x) => n + x.qty, 0),
      allChecked: raw.length > 0 && raw.every(x => x.checked),
      freeGap,
      freeText: total === 0 ? `满 ¥${store.FREE_LINE} 免运费，不满收 ¥${store.SHIP_FEE}` : freeGap > 0 ? `再买 ¥${freeGap} 免运费（运费 ¥${store.SHIP_FEE}）` : `已满 ¥${store.FREE_LINE}，本单免运费`,
      recs
    });
    // 自定义 tabBar：微信里每个 tab 页一个实例，选中态与购物车角标要各自设
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2, cartCount: store.cartCount(raw) });
  },

  onCheck(e) {
    const raw = store.getCart();
    const i = Number(e.currentTarget.dataset.index);
    raw[i].checked = !raw[i].checked;
    store.saveCart(raw);
    this.refresh();
  },

  onCheckAll() {
    const raw = store.getCart();
    const to = !this.data.allChecked;
    raw.forEach(x => { x.checked = to; });
    store.saveCart(raw);
    this.refresh();
  },

  onMinus(e) {
    const raw = store.getCart();
    const i = Number(e.currentTarget.dataset.index);
    if (raw[i].qty <= 1) {
      wx.showModal({
        title: '提示',
        content: '确定不要这件商品了吗？',
        confirmText: '删除',
        success: (res) => {
          if (res.confirm) {
            raw.splice(i, 1);
            store.saveCart(raw);
            this.refresh();
          }
        }
      });
      return;
    }
    raw[i].qty -= 1;
    store.saveCart(raw);
    this.refresh();
  },

  onPlus(e) {
    const raw = store.getCart();
    const i = Number(e.currentTarget.dataset.index);
    if (store.flashRoom(raw[i].id, raw) <= 0) {
      wx.showToast({ title: `秒杀价每单限购 ${data.FLASH_LIMIT} 件`, icon: 'none' });
      return;
    }
    if (raw[i].qty >= 99) return;
    raw[i].qty += 1;
    store.saveCart(raw);
    this.refresh();
  },

  onEdit() {
    this.setData({ editing: !this.data.editing });
  },

  onAddress() {
    wx.navigateTo({ url: '/pages/address/address?select=1' });
  },

  onDelete() {
    const raw = store.getCart();
    if (!raw.some(x => x.checked)) {
      wx.showToast({ title: '请先勾选商品', icon: 'none' });
      return;
    }
    wx.showModal({
      title: '删除商品',
      content: `确定删除选中的 ${raw.filter(x => x.checked).length} 种商品吗？`,
      success: (res) => {
        if (!res.confirm) return;
        store.saveCart(raw.filter(x => !x.checked));
        this.setData({ editing: false });
        this.refresh();
      }
    });
  },

  // 去结算：勾选的商品写进结算单，结算页下单后再从购物车里移掉
  onSettle() {
    if (!this.data.checkedCount) {
      wx.showToast({ title: '请选择要结算的商品', icon: 'none' });
      return;
    }
    store.setCheckout(store.getCart().filter(x => x.checked), 'cart');
    wx.navigateTo({ url: '/pages/checkout/checkout' });
  },

  onGoShop() {
    wx.switchTab({ url: '/pages/index/index' });
  },

  onDetail(e) {
    wx.navigateTo({ url: `/pages/detail/detail?id=${e.currentTarget.dataset.id}` });
  },

  onAddRec(e) {
    const r = store.addToCart(e.currentTarget.dataset.id, 1);
    this.refresh();
    wx.showToast({ title: r.msg || '已加入购物车', icon: r.msg ? 'none' : 'success' });
  }
});
