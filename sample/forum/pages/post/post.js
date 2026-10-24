const store = require('../../utils/store.js');
const social = require('../../utils/social.js');
const PH = '说点什么，友善发言～';

Page({
  data: { one: [], floors: [], total: 0, order: 'asc', draft: '', canSend: false, placeholder: PH, replyTo: null, intoView: '', scrollH: 600, focus: false },
  onLoad(q) {
    this.pid = q.id || 'p100';
    const sys = wx.getSystemInfoSync();
    this.setData({ scrollH: sys.windowHeight - Math.round(116 * sys.windowWidth / 750) });
    const post = store.posts().find((x) => x.id === this.pid);
    if (!post) { wx.showToast({ title: '帖子已删除', icon: 'none' }); return; }
    wx.setNavigationBarTitle({ title: store.circle(post.circle).name + '圈' });
    this.post = post;
    this.floorLikes = {};
    this.raw = store.comments(this.pid);
    this.refreshPost();
    this.render();
  },
  refreshPost() {
    const c = store.card(this.post, store.likes(), store.favs());
    this.setData({ one: [c] });
  },
  render(into) {
    const floors = this.raw.map((f) => ({
      id: f.id, floor: f.floor, author: f.author, initial: f.author.slice(0, 1), color: f.color, text: f.text, time: store.ago(f.time),
      liked: !!this.floorLikes[f.id], likes: f.likes + (this.floorLikes[f.id] ? 1 : 0), isOp: f.author === this.post.author,
      replies: f.replies.map((r) => ({ id: r.id, author: r.author, to: r.to, mid: '\u00a0回复\u00a0', text: r.text, isMe: r.author === store.ME.name }))
    }));
    if (this.data.order === 'desc') floors.reverse();
    const total = floors.reduce((a, f) => a + 1 + f.replies.length, 0);
    const d = { floors, total };
    if (into) { d.intoView = ''; }
    this.setData(d);
    if (into) this.setData({ intoView: into });
  },
  toggleOrder() {
    this.setData({ order: this.data.order === 'asc' ? 'desc' : 'asc' });
    this.render();
  },
  like() { social.toggle(this, 'one', 'like', this.pid); },
  fav() { social.toggle(this, 'one', 'fav', this.pid); },
  likeFloor(e) {
    const id = e.currentTarget.dataset.id;
    this.floorLikes[id] = !this.floorLikes[id];
    this.render();
  },
  // 点楼层的「回复」：回复层主；点楼中楼的某条：回复那个人（同一楼里）
  replyFloor(e) {
    const f = this.raw.find((x) => x.id === e.currentTarget.dataset.id);
    this.setReply({ floorId: f.id, name: f.author, sub: false });
  },
  replySub(e) {
    const ds = e.currentTarget.dataset;
    if (ds.name === store.ME.name) return wx.showToast({ title: '不能回复自己', icon: 'none' });
    this.setReply({ floorId: ds.floor, name: ds.name, sub: true });
  },
  setReply(r) {
    this.setData({ replyTo: r, placeholder: '回复 @' + r.name, focus: true });
  },
  cancelReply() { this.setData({ replyTo: null, placeholder: PH }); },
  onInput(e) { const v = e.detail.value; this.setData({ draft: v, canSend: v.trim().length > 0 }); },
  onBlur() { this.setData({ focus: false }); },
  send() {
    const text = this.data.draft.trim();
    if (!text) return;
    if (text.length > 200) return wx.showToast({ title: '评论最多 200 字', icon: 'none' });
    const r = this.data.replyTo;
    let into;
    if (r) {
      const f = this.raw.find((x) => x.id === r.floorId);
      f.replies.push({ id: 'r' + Date.now(), author: store.ME.name, to: r.sub ? r.name : '', text, time: Date.now() });
      into = f.id;
    } else {
      const id = 'f' + Date.now();
      this.raw.push({ id, floor: this.raw.length ? this.raw[this.raw.length - 1].floor + 1 : 1, author: store.ME.name, color: store.ME.color, text, time: Date.now(), likes: 0, replies: [] });
      into = id;
      if (this.data.order === 'desc') this.setData({ order: 'asc' });
    }
    store.saveComments(this.pid, this.raw);
    this.post.comments = this.raw.reduce((a, f) => a + 1 + f.replies.length, 0);
    this.setData({ draft: '', canSend: false, replyTo: null, placeholder: PH, focus: false });
    this.refreshPost();
    this.render(into);
    wx.showToast({ title: r ? '回复成功' : '评论成功', icon: 'none' });
  }
});
