const { goodsById, DELIVERY_SLOTS } = require('../../utils/data.js');
const store = require('../../utils/store.js');
const pay = require('../../utils/pay.js');

const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
function ymd(d) { return `${d.getFullYear()}-${store.pad(d.getMonth() + 1)}-${store.pad(d.getDate())}`; }
function maskPhone(p) { return String(p).replace(/^(\d{3})\d{4}(\d{4})$/, '$1****$2'); }

Page({
  data: {
    mode: 'single', items: [], address: null, groupTip: '', date: '', weekday: '', dateStart: '', dateEnd: '',
    slots: DELIVERY_SLOTS, slotIndex: 0, remark: '', goodsAmount: '0.00', freight: 0, freightFix: '0.00', freightText: '',
    saveAmount: 0, saveFix: '0.00', total: '0.00', shipTip: '', payText: '提交订单', submitting: false, cashier: false, orderNo: ''
  },

  onLoad(q) {
    this.q = q;
    const mode = q.mode || 'single';
    let items;
    if (mode === 'cart') {
      items = store.cart().filter((c) => c.checked).map((c) => this.line(c.goodsId, c.skuKey, c.qty, false));
    } else {
      items = [this.line(q.goods, decodeURIComponent(q.sku || ''), Number(q.qty) || 1, mode !== 'single')];
    }
    items = items.filter(Boolean);
    if (!items.length) { wx.showToast({ title: '没有可结算的商品', icon: 'none' }); setTimeout(() => wx.navigateBack(), 800); return; }
    const d1 = new Date(Date.now() + 2 * 86400e3), d2 = new Date(Date.now() + 9 * 86400e3);
    let groupTip = '';
    if (mode === 'group') groupTip = `开团成功后分享给好友，24 小时内凑满 ${goodsById(q.goods).groupSize} 人即发货，未满员全额退款`;
    if (mode === 'join') {
      const og = store.openGroups(q.goods).find((x) => x.id === q.join);
      this.joinSnap = og || null;
      groupTip = og ? `参与 ${og.name} 的团，还差 ${og.lack} 人，付款后${og.lack === 1 ? '立即成团' : '等待其他拼友'}` : '该团已满员或结束，将为你发起新团';
    }
    this.setData({ mode, items, groupTip, dateStart: ymd(d1), dateEnd: ymd(d2), payText: mode === 'join' ? '付款参团' : mode === 'group' ? '付款开团' : '提交订单' });
    this.setDate(ymd(d1));
    this.loadAddress();
    this.calc();
  },

  line(goodsId, key, qty, grp) {
    const g = goodsById(goodsId);
    const info = g && g.sku.map[key];
    if (!info || info.stock <= 0) return null;
    const price = grp ? info.groupPrice : info.price;
    return { key: goodsId + '#' + key, goodsId, skuKey: key, name: g.name, cover: g.cover, skuText: store.skuText(g, key), qty, price, single: info.price };
  },

  loadAddress(a) {
    const addr = a || (this.data.address && store.addresses().find((x) => x.id === this.data.address.id)) || store.defaultAddress();
    this.setData({ address: addr ? Object.assign({}, addr, { phoneMask: maskPhone(addr.phone), full: addr.region.join(' ') + ' ' + addr.detail }) : null });
    this.calc();
  },
  onShow() { if (this.data.items.length) this.loadAddress(); },

  calc() {
    const items = this.data.items;
    const goodsAmount = items.reduce((s, it) => s + it.price * it.qty, 0);
    const saveAmount = items.reduce((s, it) => s + (it.single - it.price) * it.qty, 0);
    const grp = this.data.mode === 'group' || this.data.mode === 'join';
    // 拼团单包邮；普通单满 49 包邮；新疆 / 西藏另加偏远运费
    const far = this.data.address && /新疆|西藏/.test(this.data.address.region[0]);
    let freight = grp || goodsAmount >= 49 ? 0 : 8;
    if (far) freight += 12;
    this.setData({
      goodsAmount: store.money(goodsAmount), freight, freightFix: store.money(freight), saveAmount, saveFix: store.money(saveAmount),
      total: store.money(goodsAmount + freight), freightText: freight ? `运费 ¥${store.money(freight)}` : '包邮',
      shipTip: far ? '偏远地区加收 12 元冷链运费' : grp ? '成团后 48 小时内从产地发货' : goodsAmount >= 49 ? '已满 49 元包邮' : `再买 ¥${store.money(49 - goodsAmount)} 即可包邮`
    });
  },

  setDate(v) { this.setData({ date: v, weekday: WEEK[new Date(v.replace(/-/g, '/')).getDay()] }); },
  onDate(e) { this.setDate(e.detail.value); },
  onSlot(e) { this.setData({ slotIndex: Number(e.detail.value) }); },
  onRemark(e) { this.setData({ remark: e.detail.value }); },

  pickAddress() {
    wx.navigateTo({
      url: '/pages/address/address?select=1',
      events: { picked: (a) => this.loadAddress(a) }
    });
  },
  // 微信地址：真机弹微信的地址簿；桌面直接 fail，退回自建地址簿
  useWxAddress() {
    wx.chooseAddress({
      success: (r) => {
        const list = store.saveAddress({
          name: r.userName, phone: r.telNumber, region: [r.provinceName, r.cityName, r.countyName], city: r.cityName, detail: r.detailInfo, tag: '', isDefault: false
        });
        this.loadAddress(list.find((x) => x.detail === r.detailInfo && x.name === r.userName));
      },
      fail: () => {
        wx.showToast({ title: '无法读取微信地址，请从地址簿选择', icon: 'none' });
        setTimeout(() => this.pickAddress(), 600);
      }
    });
  },

  submit() {
    if (this.data.submitting) return;
    if (!this.data.address) { wx.showToast({ title: '请先添加收货地址', icon: 'none' }); return; }
    if (this.order) { this.startPay(); return; }
    this.setData({ submitting: true });
    const d = this.data;
    const a = d.address;
    this.order = store.addOrder({
      mode: d.mode, joinFrom: d.mode === 'join' && this.joinSnap ? this.joinSnap.id : '', joinSnap: this.joinSnap || null,
      items: d.items.map((it) => ({ key: it.key, goodsId: it.goodsId, skuKey: it.skuKey, name: it.name, cover: it.cover, skuText: it.skuText, price: it.price, qty: it.qty })),
      address: { name: a.name, phone: a.phone, region: a.region, city: a.city, detail: a.detail },
      date: d.date, slot: d.slots[d.slotIndex], remark: d.remark.trim(),
      goodsAmount: d.goodsAmount, freight: d.freightFix, total: d.total
    });
    if (d.mode === 'cart') store.saveCart(store.cart().filter((c) => !c.checked));
    this.setData({ submitting: false, orderNo: this.order.no, payText: '继续支付' });
    this.startPay();
  },
  startPay() {
    pay.request(this.order, (unsupported) => {
      if (unsupported) this.setData({ cashier: true });
      else this.onPayCancel();
    });
  },
  onPaid() {
    this.setData({ cashier: false });
    pay.markPaid(this.order.id);
    wx.redirectTo({ url: '/pages/pay-result/pay-result?id=' + this.order.id });
  },
  onPayCancel() {
    this.setData({ cashier: false });
    wx.showModal({
      title: '确认放弃付款？', content: '订单会为你保留 30 分钟，超时自动取消', cancelText: '放弃', confirmText: '继续支付',
      success: (r) => {
        if (r.confirm) this.startPay();
        else wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + this.order.id });
      }
    });
  }
});
