const T = require('../../utils/store.js');
const { CITIES } = require('../../utils/data.js');

const OPEN_RPX = 300; // 左滑露出「置顶 + 删除」两个按钮的宽度

Page({
  data: { list: [], offsets: {}, openId: '', upcoming: 0, highlight: '' },
  onLoad() {
    const r = wx.getSystemInfoSync().windowWidth / 750;
    this.openPx = OPEN_RPX * r;
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    this.reload();
  },
  reload() {
    const now = T.toStr(T.today());
    const all = T.all().slice().sort((a, b) => (b.pinned ? 1 : 0) - (a.pinned ? 1 : 0) || (a.start < b.start ? -1 : 1));
    const list = all.map(t => {
      const city = CITIES.find(c => c.name === t.city) || { color: '#1e78ff', prov: '' };
      const left = T.diff(now, t.start);
      const days = T.diff(t.start, t.end) + 1;
      const spots = t.plan.reduce((n, d) => n + d.length, 0);
      const pk = T.packingStat(t.packing);
      return {
        id: t.id, title: t.title, city: t.city, prov: city.prov, color: city.color, pinned: t.pinned,
        range: T.short(t.start) + ' - ' + T.short(t.end), days, people: t.people, spots, type: t.type,
        badge: left > 0 ? left + ' 天后出发' : (T.diff(now, t.end) >= 0 ? '旅行中' : '已结束'),
        packPct: pk.pct
      };
    });
    this.setData({ list, upcoming: list.filter(t => t.badge.indexOf('天后') > 0).length, offsets: {}, openId: '' });
  },
  // —— 左滑删除：手指横向移动时卡片跟手，抬手过半就停在打开位置 ——
  onTouchStart(e) {
    const id = e.currentTarget.dataset.id;
    const t = e.touches[0];
    this.sw = { id, x: t.clientX, y: t.clientY, base: this.data.openId === id ? -this.openPx : 0, dir: '' };
    if (this.data.openId && this.data.openId !== id) this.setData({ ['offsets.' + this.data.openId]: 0, openId: '' });
  },
  onTouchMove(e) {
    const s = this.sw;
    if (!s) return;
    const t = e.touches[0];
    const dx = t.clientX - s.x, dy = t.clientY - s.y;
    if (!s.dir) {
      if (Math.abs(dx) < 6 && Math.abs(dy) < 6) return;
      s.dir = Math.abs(dx) > Math.abs(dy) ? 'h' : 'v';
    }
    if (s.dir !== 'h') return;
    const x = Math.max(-this.openPx - 20, Math.min(0, s.base + dx));
    this.setData({ ['offsets.' + s.id]: x });
  },
  onTouchEnd() {
    const s = this.sw;
    this.sw = null;
    if (this.longId) {
      // 引擎问题：长按里直接 showActionSheet，抬手那一下会把刚弹出的菜单当成点遮罩取消掉（fail cancel）。
      // 所以长按只记下是哪张卡，等抬手后再弹；微信里两种写法效果一样。
      const id = this.longId;
      this.longId = '';
      this.longAt = Date.now();
      setTimeout(() => this.showSheet(id), 30);
      return;
    }
    if (!s || s.dir !== 'h') return;
    this.justSwiped = Date.now();
    const cur = this.data.offsets[s.id] || 0;
    const open = cur < -this.openPx / 2;
    this.setData({ ['offsets.' + s.id]: open ? -this.openPx : 0, openId: open ? s.id : '' });
  },
  open(e) {
    if (this.justSwiped && Date.now() - this.justSwiped < 300) return;
    // 引擎问题：触发了 longpress 之后抬手仍会派发 tap（微信里长按触发后不再有 tap），这里挡掉
    if (this.longAt && Date.now() - this.longAt < 300) return;
    if (this.data.openId) { this.setData({ ['offsets.' + this.data.openId]: 0, openId: '' }); return; }
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  },
  // 长按：操作菜单
  onLong(e) {
    if (this.sw && this.sw.dir) return; // 已经在横滑了，不算长按
    this.longId = e.currentTarget.dataset.id;
  },
  showSheet(id) {
    const t = this.data.list.find(x => x.id === id);
    wx.showActionSheet({
      itemList: [t.pinned ? '取消置顶' : '置顶', '删除行程'],
      success: r => { if (r.tapIndex === 0) this.togglePin(id); else this.confirmDelete(id); }
    });
  },
  pinTap(e) { this.togglePin(e.currentTarget.dataset.id); },
  togglePin(id) {
    const t = T.get(id);
    t.pinned = !t.pinned;
    T.put(t);
    this.reload();
  },
  delTap(e) { this.confirmDelete(e.currentTarget.dataset.id); },
  confirmDelete(id) {
    const t = T.get(id);
    wx.showModal({
      title: '删除行程', content: '「' + t.title + '」的景点、预算和行李清单会一起删除', confirmText: '删除', confirmColor: '#f5222d',
      success: r => {
        if (!r.confirm) { this.setData({ ['offsets.' + id]: 0, openId: '' }); return; }
        T.remove(id);
        this.reload();
        wx.showToast({ title: '已删除', icon: 'none' });
      }
    });
  },
  // 新建：用 EventChannel 接收新建页回传的行程，回来后高亮那一张
  create() {
    wx.navigateTo({
      url: '/pages/create/create',
      events: {
        created: trip => {
          this.reload();
          this.setData({ highlight: trip.id });
          clearTimeout(this.hl);
          this.hl = setTimeout(() => this.setData({ highlight: '' }), 2500);
        }
      }
    });
  }
});
