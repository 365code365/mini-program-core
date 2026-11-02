// 证照夹：电子证照卡片（号码默认脱敏，点「显示」才露全号）、亮证（全屏 + 动态核验码 30 秒一换）、领取新证照。
const store = require('../../utils/store');

const ALL = [
  { id: 'L1', type: '居民身份证', no: store.USER.idcard, issuer: '云川市公安局五华分局', valid: '2020.08.16-2040.08.16', c1: '#1d64d8', c2: '#4f8ef7', ch: '身' },
  { id: 'L2', type: '机动车驾驶证', no: store.USER.idcard, extra: '准驾车型 C1', issuer: '云川市公安局交通警察支队', valid: '2018.05.11-2028.05.11', c1: '#0f766e', c2: '#2dd4bf', ch: '驾' },
  { id: 'L3', type: '社会保障卡', no: 'YC5301' + store.USER.idcard.slice(-8), extra: '金融账户已激活', issuer: '云川市人力资源和社会保障局', valid: '2021.03.02-2031.03.02', c1: '#b45309', c2: '#f59e0b', ch: '社' },
  { id: 'L4', type: '居民户口簿', no: '530102000' + store.USER.idcard.slice(-6), extra: '户主 张明远', issuer: '云川市公安局五华分局', valid: '长期', c1: '#7c3aed', c2: '#a78bfa', ch: '户' },
  { id: 'L5', type: '不动产权证书', no: '云(2023)五华区不动产权第0031725号', extra: '滨江区江湾大道 216 号 3 幢 1102 室', issuer: '云川市自然资源和规划局', valid: '2023.06.20 登记', c1: '#be123c', c2: '#fb7185', ch: '房' }
];

function mask(no) { return no.length > 10 ? no.slice(0, 3) + '*'.repeat(no.length - 7) + no.slice(-4) : no; }

Page({
  data: { cards: [], pending: [], openId: '', showFull: {}, show: null, code: '', left: 30, holder: store.USER.name },
  onShow() {
    store.syncTab(this, 2);
    this.refresh();
  },
  onHide() { this.closeShow(); },
  onUnload() { this.closeShow(); },
  refresh() {
    let owned = wx.getStorageSync('gov_licenses');
    if (!Array.isArray(owned)) { owned = ['L1', 'L2', 'L3']; wx.setStorageSync('gov_licenses', owned); }
    const cards = ALL.filter(c => owned.indexOf(c.id) >= 0).map(c => Object.assign({}, c, { masked: mask(c.no) }));
    const pending = ALL.filter(c => owned.indexOf(c.id) < 0);
    this.setData({ cards, pending });
  },
  toggle(e) {
    const id = e.currentTarget.dataset.id;
    this.setData({ openId: this.data.openId === id ? '' : id });
  },
  toggleNo(e) {
    const id = e.currentTarget.dataset.id;
    this.setData({ ['showFull.' + id]: !this.data.showFull[id] });
  },
  claim(e) {
    const c = ALL.find(x => x.id === e.currentTarget.dataset.id);
    wx.showModal({
      title: '领取电子证照', content: `将从${c.issuer}获取您的「${c.type}」电子证照，需验证实名信息。`, confirmText: '确认领取',
      success: (r) => {
        if (!r.confirm) return;
        const owned = wx.getStorageSync('gov_licenses') || [];
        owned.push(c.id);
        wx.setStorageSync('gov_licenses', owned);
        this.refresh();
        wx.showToast({ title: '已领取', icon: 'success' });
      }
    });
  },
  // 亮证：动态核验码按 30 秒窗口生成，窗口内不变（核验方扫码时比对）
  showCard(e) {
    const c = this.data.cards.find(x => x.id === e.currentTarget.dataset.id);
    this.setData({ show: c });
    this.tick();
    this.timer = setInterval(() => this.tick(), 1000);
  },
  tick() {
    const now = Date.now();
    const win = Math.floor(now / 30000);
    const code = String((win * 7919 + this.data.show.id.charCodeAt(1) * 104729) % 1000000).padStart(6, '0');
    this.setData({ code, left: 30 - Math.floor((now % 30000) / 1000), stamp: store.fmtTime(now) });
  },
  closeShow() {
    if (this.timer) { clearInterval(this.timer); this.timer = null; }
    if (this.data.show) this.setData({ show: null });
  },
  noop() {}
});
