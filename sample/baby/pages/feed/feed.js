const store = require('../../utils/store.js');
const T = require('../../utils/time.js');

// 母乳计时：storage 里只存开始时间戳与已累计秒数（{side, startedAt, left, right, beganAt}），
// 页面 onHide / 退出时只停掉刷新用的 setInterval，回来按时间戳重算 —— 切到后台、锁屏，时间照样在走。
Page({
  data: {
    mode: 'breast', side: '', leftText: '00:00', rightText: '00:00', totalText: '00:00', hasTimer: false,
    ml: 120, date: '', time: '', today: '', note: '', err: '', list: [], totalMl: 0
  },

  onLoad(q) {
    const now = Date.now();
    this.setData({ mode: q.mode === 'formula' ? 'formula' : 'breast', date: T.ymd(now), today: T.ymd(now), time: T.hm(now) });
  },
  onShow() {
    this.tick();
    this.loadList();
    this._iv = setInterval(() => this.tick(), 1000);
  },
  onHide() { clearInterval(this._iv); },
  onUnload() { clearInterval(this._iv); },

  timer() { return store.get(store.K.feedTimer, null); },

  tick() {
    const t = this.timer();
    if (!t) {
      if (this.data.hasTimer || this.data.side) this.setData({ hasTimer: false, side: '', leftText: '00:00', rightText: '00:00', totalText: '00:00' });
      return;
    }
    const run = t.side ? (Date.now() - t.startedAt) / 1000 : 0;
    const l = t.left + (t.side === 'left' ? run : 0), r = t.right + (t.side === 'right' ? run : 0);
    this.setData({ hasTimer: true, side: t.side, leftText: T.clock(l), rightText: T.clock(r), totalText: T.clock(l + r) });
  },

  loadList() {
    const list = store.list(store.K.feeds).slice(0, 12).map((r) => ({
      id: r.id,
      day: T.isToday(r.ts) ? '今天' : T.ymd(r.ts).slice(5),
      time: T.hm(r.ts),
      title: r.type === 'breast' ? '母乳亲喂' : '奶粉 ' + r.ml + 'ml',
      desc: r.type === 'breast' ? '左 ' + T.clock(r.left) + ' · 右 ' + T.clock(r.right) : (r.note || '瓶喂'),
      breast: r.type === 'breast'
    }));
    const totalMl = store.today().ml;
    this.setData({ list, totalMl });
  },

  switchMode(e) { this.setData({ mode: e.currentTarget.dataset.mode, err: '' }); },

  // 点左 / 右：没在计时就开始；点正在计时的一侧是暂停；点另一侧是换边
  tapSide(e) {
    const side = e.currentTarget.dataset.side;
    const now = Date.now();
    let t = this.timer();
    if (!t) {
      t = { side, startedAt: now, left: 0, right: 0, beganAt: now };
      wx.showToast({ title: (side === 'left' ? '左侧' : '右侧') + '开始计时', icon: 'none' });
    } else {
      if (t.side) t[t.side] += (now - t.startedAt) / 1000;
      t.side = t.side === side ? '' : side;
      t.startedAt = now;
    }
    wx.vibrateShort({ type: 'light' });
    store.set(store.K.feedTimer, t);
    this.tick();
  },

  finish() {
    const t = this.timer();
    if (!t) return;
    if (t.side) t[t.side] += (Date.now() - t.startedAt) / 1000;
    const left = Math.round(t.left), right = Math.round(t.right);
    if (left + right < 10) {
      wx.showToast({ title: '喂奶时间太短，继续计时或放弃', icon: 'none' });
      return;
    }
    store.add(store.K.feeds, { type: 'breast', ts: t.beganAt, left, right });
    wx.removeStorageSync(store.K.feedTimer);
    wx.vibrateShort({ type: 'medium' });
    wx.showToast({ title: '已保存 ' + Math.round((left + right) / 60) + ' 分钟', icon: 'success' });
    this.tick();
    this.loadList();
  },

  discard() {
    wx.showModal({
      title: '放弃这次计时？', content: '计时数据不会保存', confirmText: '放弃', confirmColor: '#f0506e',
      success: (r) => {
        if (!r.confirm) return;
        wx.removeStorageSync(store.K.feedTimer);
        this.tick();
      }
    });
  },

  onMl(e) { this.setData({ ml: e.detail.value }); },
  onMlChanging(e) { this.setData({ ml: e.detail.value }); },
  onDate(e) { this.setData({ date: e.detail.value, err: '' }); },
  onTime(e) { this.setData({ time: e.detail.value, err: '' }); },
  onNote(e) { this.setData({ note: e.detail.value }); },

  saveFormula() {
    const ts = T.parse(this.data.date, this.data.time);
    if (ts > Date.now() + 60000) {
      this.setData({ err: '喂奶时间不能晚于现在' });
      return;
    }
    store.add(store.K.feeds, { type: 'formula', ts, ml: this.data.ml, note: this.data.note.trim() });
    wx.vibrateShort({ type: 'light' });
    wx.showToast({ title: '已记录 ' + this.data.ml + 'ml', icon: 'success' });
    this.setData({ note: '', err: '' });
    this.loadList();
  },

  onItem(e) {
    const id = e.currentTarget.dataset.id;
    wx.showActionSheet({
      itemList: ['删除这条记录'],
      itemColor: '#f0506e',
      success: () => {
        store.remove(store.K.feeds, id);
        wx.showToast({ title: '已删除', icon: 'none' });
        this.loadList();
      }
    });
  }
});
