const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
// 号段 → 运营商（只覆盖常见号段，够演示用）
function carrierOf(p) {
  const h = p.slice(0, 3);
  if (/^(13[4-9]|15[012789]|18[23478]|19[578]|147|178)$/.test(h)) return '中国移动';
  if (/^(13[012]|15[56]|18[56]|166|17[56]|145)$/.test(h)) return '中国联通';
  if (/^(133|153|18[019]|17[37]|19[0139])$/.test(h)) return '中国电信';
  return '';
}
Page({
  data: { phone: '13816626621', phoneTx: '', carrier: '', faces: [], idx: 4, recent: ['13816626621', '13917522086'], tab: 'fee',
    flows: [{ v: '5GB', d: '7 天有效', price: 15 }, { v: '10GB', d: '当月有效', price: 30 }, { v: '20GB', d: '当月有效', price: 50 }, { v: '30GB', d: '30 天有效', price: 70 }] },
  onLoad() {
    this.setData({ faces: data.faces.map(f => Object.assign({}, f, { priceTx: f.price.toFixed(2).replace(/\.?0+$/, ''), off: f.v > f.price })) });
    this.check(this.data.phone);
  },
  fmt(p) { return p.replace(/^(\d{3})(\d{0,4})(\d{0,4}).*/, (m, a, b, c) => [a, b, c].filter(Boolean).join(' ')); },
  check(p) { this.setData({ phone: p, phoneTx: this.fmt(p), carrier: p.length === 11 ? carrierOf(p) : '' }); },
  onPhone(e) { this.check(e.detail.value.replace(/\D/g, '').slice(0, 11)); },
  clear() { this.check(''); },
  useRecent(e) { this.check(e.currentTarget.dataset.p); },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.t, idx: e.currentTarget.dataset.t === 'fee' ? 4 : 1 }); },
  onFace(e) { this.setData({ idx: Number(e.currentTarget.dataset.i) }); },
  pay() {
    const d = this.data;
    if (d.phone.length !== 11 || !d.carrier) { wx.showToast({ title: '请输入正确的手机号', icon: 'none' }); return; }
    const isFee = d.tab === 'fee';
    const item = isFee ? d.faces[d.idx] : d.flows[d.idx];
    const name = isFee ? item.v + ' 元话费' : item.v + ' 流量包';
    wx.showModal({ title: '确认充值', content: '为 ' + d.phoneTx + '（' + d.carrier + '）充值 ' + name + '\n实付 ¥' + store.money(item.price), confirmText: '确认支付', confirmColor: '#14a06f', success: r => {
      if (!r.confirm) return;
      const masked = d.phone.slice(0, 3) + '****' + d.phone.slice(7);
      const rec = store.addRecord({ type: 'phone', title: isFee ? '话费充值' : '流量充值', target: masked, amount: isFee ? item.v : item.price, paid: item.price, unit: d.carrier });
      const recent = [d.phone].concat(d.recent.filter(x => x !== d.phone)).slice(0, 3);
      wx.setStorageSync('lp_recent_phone', recent);
      wx.redirectTo({ url: '/pages/result/result?id=' + rec.id });
    } });
  },
  onShow() { const r = wx.getStorageSync('lp_recent_phone'); if (Array.isArray(r) && r.length) this.setData({ recent: r }); }
});
