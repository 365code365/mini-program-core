const store = require('../../utils/store.js');
const social = require('../../utils/social.js');
const PAGE = 6;
const FRESH = [
  ['photo', '今天的晚霞像被打翻的橘子汽水', '下班路上抬头看到的，手机直出没修图。'],
  ['cook', '复刻了一家网红店的葱油拌面', '关键是葱油要小火慢熬，葱变成焦褐色就关火。'],
  ['code', '终于把项目的构建时间从 40 分钟降到 2 分钟', '关掉了不必要的 LTO，开了增量编译，效果立竿见影。'],
  ['pet', '猫主子今天又把杯子推下桌了', '我已经习惯了，杯子都换成塑料的了。'],
  ['travel', '错峰出游的快乐谁懂', '工作日去的景区，几乎包场。'],
  ['run', '雨后的空气太适合跑步了', '配速比平时快了 15 秒。']
];

Page({
  data: { tabs: [], cur: 'all', tabInto: '', list: [], hasMore: true, loading: false, banner: '', empty: false },
  onLoad() {
    const tabs = [{ id: 'all', name: '推荐' }, { id: 'follow', name: '已加入' }].concat(store.CIRCLES.map((c) => ({ id: c.id, name: c.name })));
    this.setData({ tabs });
    this.shown = 0;
    this.reload();
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    // 圈子页「去看看」通过 storage 指定要切到哪个圈子（switchTab 不能带参数）
    const go = wx.getStorageSync('forum_go_circle');
    if (go) {
      wx.removeStorageSync('forum_go_circle');
      this.switchTo(go);
      return;
    }
    // 从详情页 / 发帖页回来：点赞、评论数、新帖都可能变了，按已加载的条数重算一遍
    if (this.loaded) this.rebuild(Math.max(this.shown, PAGE));
    this.loaded = true;
  },
  source() {
    const cur = this.data.cur;
    const all = store.posts();
    if (cur === 'all') return all;
    if (cur === 'follow') { const j = store.joined(); return all.filter((p) => j.indexOf(p.circle) >= 0); }
    return all.filter((p) => p.circle === cur);
  },
  rebuild(n) {
    const src = this.source();
    const likes = store.likes(); const favs = store.favs();
    const list = src.slice(0, n).map((p) => store.card(p, likes, favs));
    this.shown = list.length;
    this.setData({ list, hasMore: src.length > list.length, empty: list.length === 0 });
  },
  reload() { this.rebuild(PAGE); },
  onTab(e) { this.switchTo(e.currentTarget.dataset.id); },
  switchTo(id) {
    if (!this.data.tabs.find((t) => t.id === id)) return;
    const i = this.data.tabs.findIndex((t) => t.id === id);
    // 让选中项左边留一个，避免贴边
    this.setData({ cur: id, tabInto: 't-' + this.data.tabs[Math.max(0, i - 1)].id, loading: false });
    this.reload();
    wx.pageScrollTo({ scrollTop: 0, duration: 0 });
  },
  onPullDownRefresh() {
    setTimeout(() => {
      const all = store.posts();
      const n = this.freshIdx || 0;
      const add = [];
      for (let k = 0; k < 2; k++) {
        const f = FRESH[(n + k) % FRESH.length];
        const cur = this.data.cur;
        const circle = (cur === 'all' || cur === 'follow') ? f[0] : cur;
        const u = store.USERS[(n + k * 5) % store.USERS.length];
        add.push({ id: 'n' + Date.now() + k, circle, author: u, color: store.COLORS[(n + k) % 8], mine: false, title: f[1], body: f[2], tags: ['晒图'], pics: k === 0 ? 1 : 0, likes: 0, comments: 0, time: Date.now() - k * 30000 });
      }
      this.freshIdx = n + 2;
      store.savePosts(add.concat(all));
      this.rebuild(this.shown + add.length);
      wx.stopPullDownRefresh();
      this.setData({ banner: '为你更新了 ' + add.length + ' 条内容' });
      clearTimeout(this.bt);
      this.bt = setTimeout(() => this.setData({ banner: '' }), 2000);
    }, 800);
  },
  onReachBottom() {
    if (this.data.loading || !this.data.hasMore) return;
    this.setData({ loading: true });
    setTimeout(() => {
      this.setData({ loading: false });
      this.rebuild(this.shown + PAGE);
    }, 700);
  },
  open(e) { wx.navigateTo({ url: '/pages/post/post?id=' + e.detail.id }); },
  like(e) { social.toggle(this, 'list', 'like', e.detail.id); },
  fav(e) { social.toggle(this, 'list', 'fav', e.detail.id); },
  goCircle(e) {
    const p = store.posts().find((x) => x.id === e.detail.id);
    if (p) wx.navigateTo({ url: '/pages/circle/circle?id=' + p.circle });
  },
  compose() { wx.navigateTo({ url: '/pages/compose/compose' }); }
});
