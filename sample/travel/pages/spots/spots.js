const { spotsOf } = require('../../utils/data.js');

const TYPES = ['全部', '人文', '自然', '街区', '美食', '休闲', '购物'];

Page({
  data: { city: '', day: 1, kw: '', types: TYPES, type: '全部', list: [], picked: {}, count: 0, custom: '', customList: [] },
  onLoad(q) {
    this.city = decodeURIComponent(q.city || '成都');
    this.all = spotsOf(this.city);
    this.taken = {};
    this.setData({ city: this.city, day: Number(q.day || 1) });
    wx.setNavigationBarTitle({ title: '添加到第' + (q.day || 1) + '天' });
    const ch = typeof this.getOpenerEventChannel === 'function' ? this.getOpenerEventChannel() : null;
    if (ch && ch.on) ch.on('init', d => { this.taken = d.taken || {}; this.filter(); });
    this.filter();
  },
  filter() {
    const kw = this.data.kw.trim();
    const type = this.data.type;
    const list = this.all
      .filter(s => (type === '全部' || s.type === type) && (!kw || s.name.indexOf(kw) >= 0))
      .map(s => Object.assign({}, s, { takenDay: this.taken[s.name] || 0, on: !!this.data.picked[s.name] }));
    this.setData({ list });
  },
  onKw(e) { this.setData({ kw: e.detail.value }); this.filter(); },
  clearKw() { this.setData({ kw: '' }); this.filter(); },
  onType(e) { this.setData({ type: e.currentTarget.dataset.t }); this.filter(); },
  toggle(e) {
    const name = e.currentTarget.dataset.name;
    if (this.taken[name]) { wx.showToast({ title: '已安排在第' + this.taken[name] + '天', icon: 'none' }); return; }
    const picked = Object.assign({}, this.data.picked);
    if (picked[name]) delete picked[name]; else picked[name] = true;
    this.setData({ picked, count: Object.keys(picked).length });
    this.filter();
  },
  onCustom(e) { this.setData({ custom: e.detail.value.slice(0, 16) }); },
  addCustom() {
    const name = this.data.custom.trim();
    if (!name) return;
    if (this.all.some(s => s.name === name)) { wx.showToast({ title: '列表里已有这个地点', icon: 'none' }); return; }
    const s = { name, type: '休闲', hours: 1.5, ticket: '待定' };
    this.all.unshift(s);
    const picked = Object.assign({}, this.data.picked, { [name]: true });
    this.setData({ custom: '', picked, count: Object.keys(picked).length });
    this.filter();
  },
  confirm() {
    if (!this.data.count) return;
    const list = this.all.filter(s => this.data.picked[s.name]);
    const ch = this.getOpenerEventChannel();
    ch.emit('picked', list);
    wx.navigateBack();
  }
});
