const store = require('../../utils/store.js');

// 户型图 345×270px（690rpx×540rpx），家具坐标都是相对户型图左上角的 px
const DEFAULTS = [
  { id: 'tv', name: '电视柜', w: 90, h: 14, x: 46, y: 16, color: '#8a6a4a' },
  { id: 'sofa', name: '三人沙发', w: 84, h: 30, x: 48, y: 120, color: '#7fa89b' },
  { id: 'tea', name: '茶几', w: 44, h: 24, x: 68, y: 78, color: '#c8a27a' },
  { id: 'dining', name: '餐桌', w: 50, h: 34, x: 150, y: 96, color: '#b8864e' },
  { id: 'bed', name: '双人床', w: 60, h: 74, x: 250, y: 22, color: '#9fb3c8' },
  { id: 'wardrobe', name: '衣柜', w: 70, h: 20, x: 240, y: 116, color: '#a58566' },
  { id: 'bed2', name: '单人床', w: 40, h: 70, x: 285, y: 166, color: '#c3b4d6' },
  { id: 'desk', name: '书桌', w: 46, h: 24, x: 225, y: 226, color: '#c9a57a' },
  { id: 'fridge', name: '冰箱', w: 26, h: 26, x: 18, y: 222, color: '#9aa3a8' }
];

const ROOMS = [
  { name: '客餐厅 32㎡', x: 148, y: 140 },
  { name: '主卧 14㎡', x: 250, y: 98 },
  { name: '次卧 11㎡', x: 230, y: 150 },
  { name: '厨房 6㎡', x: 40, y: 172 },
  { name: '卫生间 5㎡', x: 128, y: 172 }
];

Page({
  data: { items: [], rooms: ROOMS, sel: '', selName: '', selPos: '', savedAt: '', dirty: false },

  onLoad() {
    const saved = store.get(store.K.layout, null);
    const byId = {};
    ((saved && saved.items) || []).forEach((s) => { byId[s.id] = s; });
    const items = DEFAULTS.map((d) => this.shape(d, byId[d.id]));
    this._pos = {};
    items.forEach((it) => { this._pos[it.id] = { x: it.x, y: it.y }; });
    this.setData({ items, savedAt: saved ? saved.savedAt : '' });
  },
  onUnload() { clearTimeout(this._t); },

  // 默认值 + 存档（位置、旋转、缩放）→ 渲染用的一项
  shape(d, s) {
    const rot = s ? !!s.rot : false, scale = s ? s.scale || 1 : 1;
    return {
      id: d.id, name: d.name, color: d.color, rot, scale,
      x: s ? s.x : d.x, y: s ? s.y : d.y,
      w: rot ? d.h : d.w, h: rot ? d.w : d.h
    };
  },

  select(e) {
    const id = e.currentTarget.dataset.id;
    if (id === this.data.sel) return;
    const it = this.data.items.find((x) => x.id === id);
    const p = this._pos[id];
    this.setData({ sel: id, selName: it.name, selPos: Math.round(p.x) + ', ' + Math.round(p.y) });
  },

  // movable-view 的 change：记住位置，停手 400ms 后写 storage（拖动中每帧都会来）
  onMove(e) {
    const id = e.currentTarget.dataset.id;
    this._pos[id] = { x: e.detail.x, y: e.detail.y };
    if (e.detail.source === 'touch' && id !== this.data.sel) this.select(e);
    else if (id === this.data.sel) this.setData({ selPos: Math.round(e.detail.x) + ', ' + Math.round(e.detail.y) });
    if (e.detail.source === 'touch' || e.detail.source === 'out-of-bounds' || e.detail.source === 'touch-out-of-bounds') {
      clearTimeout(this._t);
      this._t = setTimeout(() => this.persist(false), 400);
    }
  },

  // 改尺寸 / 缩放时把当前位置一并写回 items，否则 x / y 属性会把家具拉回旧位置
  update(fn) {
    if (!this.data.sel) return wx.showToast({ title: '先点一下要调整的家具', icon: 'none' });
    const items = this.data.items.map((it) => {
      const p = this._pos[it.id];
      const n = Object.assign({}, it, { x: p.x, y: p.y });
      return it.id === this.data.sel ? fn(n) : n;
    });
    this.setData({ items });
    this.persist(false);
  },
  zoomIn() { this.update((it) => Object.assign(it, { scale: Math.min(2, Math.round((it.scale + 0.1) * 10) / 10) })); },
  zoomOut() { this.update((it) => Object.assign(it, { scale: Math.max(0.5, Math.round((it.scale - 0.1) * 10) / 10) })); },
  rotate() { this.update((it) => Object.assign(it, { rot: !it.rot, w: it.h, h: it.w })); },

  persist(manual) {
    const items = this.data.items.map((it) => ({ id: it.id, x: Math.round(this._pos[it.id].x), y: Math.round(this._pos[it.id].y), rot: it.rot, scale: it.scale }));
    const savedAt = store.ymdhm(Date.now());
    store.set(store.K.layout, { items, savedAt });
    this.setData({ savedAt });
    if (manual) wx.showToast({ title: '户型方案已保存', icon: 'success' });
  },
  save() { this.persist(true); },

  reset() {
    wx.showModal({
      title: '恢复默认摆放？', content: '所有家具回到初始位置，旋转和缩放也会清除',
      success: (r) => {
        if (!r.confirm) return;
        const items = DEFAULTS.map((d) => this.shape(d, null));
        items.forEach((it) => { this._pos[it.id] = { x: it.x, y: it.y }; });
        wx.removeStorageSync(store.K.layout);
        this.setData({ items, sel: '', selName: '', selPos: '', savedAt: '' });
      }
    });
  },

  book() { wx.navigateTo({ url: '/pages/booking/booking?area=89&houseType=' + encodeURIComponent('两室两厅') }); }
});
