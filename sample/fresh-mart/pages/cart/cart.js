const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');

const FREE_LINE = 39;

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
    this.setData({ address: getApp().globalData.address });
    this.refresh();
  },

  refresh() {
    const raw = cart.get();
    let total = 0;
    let origin = 0;
    let checkedCount = 0;
    const list = raw.map(x => {
      const g = data.findGoods(x.id);
      const unit = cart.unitPrice(x.id, x.spec);
      if (x.checked) {
        total += unit * x.qty;
        origin += (g.origin + (unit - (data.flashOf(x.id) ? data.flashOf(x.id).price : g.price))) * x.qty;
        checkedCount += x.qty;
      }
      return { key: x.id + '|' + x.spec, id: x.id, name: g.name, img: g.img, spec: x.spec, qty: x.qty,
        checked: x.checked, unit, flash: !!data.flashOf(x.id) };
    });
    total = cart.round2(total);
    const freeGap = cart.round2(FREE_LINE - total);
    const inCart = raw.map(x => x.id);
    const recs = data.goods.filter(g => inCart.indexOf(g.id) < 0).sort((a, b) => b.sales - a.sales).slice(0, 4);
    this.setData({
      list,
      total,
      saved: cart.round2(origin - total),
      checkedCount,
      allChecked: raw.length > 0 && raw.every(x => x.checked),
      freeGap,
      freeText: total === 0 ? `满 ¥${FREE_LINE} 免运费，不满收 ¥5` : freeGap > 0 ? `再买 ¥${freeGap} 免运费（运费 ¥5）` : '已满 ¥39，本单免运费',
      recs
    });
  },

  onCheck(e) {
    const raw = cart.get();
    const i = Number(e.currentTarget.dataset.index);
    raw[i].checked = !raw[i].checked;
    cart.save(raw);
    this.refresh();
  },

  onCheckAll() {
    const raw = cart.get();
    const to = !this.data.allChecked;
    raw.forEach(x => { x.checked = to; });
    cart.save(raw);
    this.refresh();
  },

  onMinus(e) {
    const raw = cart.get();
    const i = Number(e.currentTarget.dataset.index);
    if (raw[i].qty <= 1) {
      wx.showModal({
        title: '提示',
        content: '确定不要这件商品了吗？',
        confirmText: '删除',
        success: (res) => {
          if (res.confirm) {
            raw.splice(i, 1);
            cart.save(raw);
            this.refresh();
          }
        }
      });
      return;
    }
    raw[i].qty -= 1;
    cart.save(raw);
    this.refresh();
  },

  onPlus(e) {
    const raw = cart.get();
    const i = Number(e.currentTarget.dataset.index);
    raw[i].qty += 1;
    cart.save(raw);
    this.refresh();
  },

  onEdit() {
    this.setData({ editing: !this.data.editing });
  },

  onDelete() {
    const raw = cart.get();
    if (!raw.some(x => x.checked)) {
      wx.showToast({ title: '请先勾选商品', icon: 'none' });
      return;
    }
    wx.showModal({
      title: '删除商品',
      content: `确定删除选中的 ${raw.filter(x => x.checked).length} 种商品吗？`,
      success: (res) => {
        if (!res.confirm) return;
        cart.save(raw.filter(x => !x.checked));
        this.setData({ editing: false });
        this.refresh();
      }
    });
  },

  onSettle() {
    if (!this.data.checkedCount) {
      wx.showToast({ title: '请选择要结算的商品', icon: 'none' });
      return;
    }
    const fee = this.data.freeGap > 0 ? 5 : 0;
    wx.showModal({
      title: '确认下单',
      content: `共 ${this.data.checkedCount} 件，商品 ¥${this.data.total}，运费 ¥${fee}\n预计 30 分钟内送达 ${this.data.address}`,
      confirmText: '微信支付',
      success: (res) => {
        if (!res.confirm) return;
        cart.save(cart.get().filter(x => !x.checked));
        const n = (wx.getStorageSync('fm_order_count') || 0) + 1;
        wx.setStorageSync('fm_order_count', n);
        wx.showToast({ title: '下单成功', icon: 'success' });
        this.refresh();
      }
    });
  },

  onGoShop() {
    wx.switchTab({ url: '/pages/index/index' });
  },

  onDetail(e) {
    wx.navigateTo({ url: `/pages/detail/detail?id=${e.currentTarget.dataset.id}` });
  },

  onAddRec(e) {
    cart.add(e.currentTarget.dataset.id, 1);
    this.refresh();
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  }
});
