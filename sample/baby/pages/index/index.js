const store = require('../../utils/store.js');
const T = require('../../utils/time.js');
const V = require('../../utils/vaccines.js');
const KN = require('../../utils/knowledge.js');

const DIAPER = { wet: '嘘嘘', dirty: '便便', both: '嘘嘘+便便' };

Page({
  data: {
    baby: {}, age: {}, sum: {}, timeline: [], nextVac: null, running: '', article: null,
    quick: [
      { key: 'feed', text: '喂奶', icon: '/images/quick/feed.png' },
      { key: 'sleep', text: '睡眠', icon: '/images/quick/sleep.png' },
      { key: 'diaper', text: '换尿布', icon: '/images/quick/diaper.png' },
      { key: 'growth', text: '身高体重', icon: '/images/quick/growth.png' },
      { key: 'vaccine', text: '疫苗', icon: '/images/quick/vaccine.png' },
      { key: 'camera', text: '拍照记录', icon: '/images/quick/camera.png' },
      { key: 'book', text: '育儿知识', icon: '/images/quick/book.png' },
      { key: 'album', text: '成长相册', icon: '/images/quick/heart.png' }
    ]
  },

  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 0 });
    this.refresh();
    this.tickRunning();
    this._timer = setInterval(() => this.tickRunning(), 1000);
  },
  onHide() { clearInterval(this._timer); },
  onUnload() { clearInterval(this._timer); },

  refresh() {
    const baby = store.profile();
    const age = T.age(baby.birthday);
    const t = store.today();
    const items = [];
    t.feeds.forEach((r) => items.push({
      id: r.id, key: store.K.feeds, ts: r.ts, kind: 'feed',
      title: r.type === 'breast' ? '母乳亲喂' : '奶粉 ' + r.ml + 'ml',
      desc: r.type === 'breast' ? '左 ' + Math.round(r.left / 60) + ' 分钟 · 右 ' + Math.round(r.right / 60) + ' 分钟' : (r.note || '瓶喂')
    }));
    t.sleeps.forEach((r) => items.push({
      id: r.id, key: store.K.sleeps, ts: r.end, kind: 'sleep', title: '睡醒了',
      desc: T.hm(r.start) + ' 入睡 · 睡了 ' + T.duration((r.end - r.start) / 60000)
    }));
    t.diapers.forEach((r) => items.push({ id: r.id, key: store.K.diapers, ts: r.ts, kind: 'diaper', title: '换尿布', desc: DIAPER[r.kind] || '' }));
    items.sort((a, b) => b.ts - a.ts);
    items.forEach((it) => { it.time = T.hm(it.ts); });

    const done = store.get(store.K.vaccines, {});
    const pending = V.LIST.filter((v) => !done[v.code]);
    let nextVac = null;
    if (pending.length) {
      const v = pending[0];
      const date = T.addMonths(baby.birthday, v.month);
      const left = T.daysBetween(T.ymd(Date.now()), date);
      nextVac = { name: v.name + ' ' + v.dose, date, left, overdue: left < 0, leftAbs: Math.abs(left) };
    }
    const stage = KN.stageOf(age.months);
    const article = KN.ARTICLES.filter((a) => a.stage === stage)[age.days % 2] || KN.ARTICLES[0];
    this.setData({
      baby, age, timeline: items, nextVac, article,
      sum: {
        feedCount: t.feeds.length, ml: t.ml, breastMin: t.breastMin,
        sleep: (t.sleepMin / 60).toFixed(1), diaper: t.diapers.length
      }
    });
  },

  // 正在计时的喂奶 / 睡眠（计时器存的是开始时间戳，离开页面也照样在走）
  tickRunning() {
    const f = store.get(store.K.feedTimer, null);
    const s = store.get(store.K.sleepTimer, null);
    let running = '';
    if (f && f.side) {
      const cur = (Date.now() - f.startedAt) / 1000;
      const sec = (f[f.side] || 0) + cur;
      running = '正在喂奶 · ' + (f.side === 'left' ? '左侧 ' : '右侧 ') + T.clock(sec);
    } else if (s && s.startedAt) {
      running = '宝宝睡觉中 · 已睡 ' + T.clock((Date.now() - s.startedAt) / 1000);
    }
    if (running !== this.data.running) this.setData({ running });
  },

  goRunning() {
    const f = store.get(store.K.feedTimer, null);
    wx.navigateTo({ url: f && f.side ? '/pages/feed/feed' : '/pages/sleep/sleep' });
  },
  goProfile() { wx.navigateTo({ url: '/pages/profile/profile' }); },
  goVaccine() { wx.navigateTo({ url: '/pages/vaccine/vaccine' }); },
  goArticle() { wx.navigateTo({ url: '/pages/article/article?id=' + this.data.article.id }); },

  onQuick(e) {
    const k = e.currentTarget.dataset.key;
    const nav = { feed: '/pages/feed/feed', sleep: '/pages/sleep/sleep', growth: '/pages/growth-add/growth-add', vaccine: '/pages/vaccine/vaccine', camera: '/pages/shoot/shoot' };
    if (nav[k]) return wx.navigateTo({ url: nav[k] });
    if (k === 'book') return wx.switchTab({ url: '/pages/knowledge/knowledge' });
    if (k === 'album') return wx.switchTab({ url: '/pages/album/album' });
    if (k === 'diaper') this.addDiaper();
  },

  addDiaper() {
    const kinds = ['wet', 'dirty', 'both'];
    wx.showActionSheet({
      itemList: ['嘘嘘', '便便', '嘘嘘+便便'],
      success: (res) => {
        store.add(store.K.diapers, { ts: Date.now(), kind: kinds[res.tapIndex] });
        wx.vibrateShort({ type: 'light' });
        wx.showToast({ title: '已记录换尿布', icon: 'success' });
        this.refresh();
      }
    });
  },

  onItem(e) {
    const { id, key, title } = e.currentTarget.dataset;
    wx.showActionSheet({
      itemList: ['删除这条记录'],
      itemColor: '#f0506e',
      success: () => {
        wx.showModal({
          title: '删除记录', content: '确定删除「' + title + '」吗？', confirmColor: '#f0506e',
          success: (r) => {
            if (!r.confirm) return;
            store.remove(key, id);
            wx.showToast({ title: '已删除', icon: 'none' });
            this.refresh();
          }
        });
      }
    });
  }
});
