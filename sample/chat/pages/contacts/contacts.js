const store = require('../../utils/store.js');
Page({
  data: { groups: [], letters: [], intoView: '', bigLetter: '', listH: 600, idxTop: 0, letterH: 20, kw: '', count: 0, newFriends: 2 },
  onLoad() {
    const sys = wx.getSystemInfoSync();
    this.rpx = sys.windowWidth / 750;
    this.listH = Math.floor(sys.windowHeight - 100 * this.rpx);
    this.letterH = Math.round(40 * this.rpx);
  },
  onShow() {
    store.syncTabBadge(this, 1);
    this.build();
  },
  build() {
    const kw = this.data.kw.trim();
    const stars = store.stars();
    const map = {};
    const groups = [];
    const starList = [];
    store.CONTACTS.forEach((c) => {
      if (kw && c.name.indexOf(kw) < 0 && c.remark.indexOf(kw) < 0 && c.wxid.indexOf(kw) < 0) return;
      const row = { id: c.id, name: c.name, remark: c.remark, initial: c.initial, color: c.color, sign: c.sign };
      if (stars.indexOf(c.id) >= 0 && !kw) starList.push(Object.assign({ key: 's' + c.id }, row));
      if (!map[c.py]) { map[c.py] = { id: 'g-' + c.py, title: c.py, items: [] }; groups.push(map[c.py]); }
      map[c.py].items.push(Object.assign({ key: c.id }, row));
    });
    groups.sort((a, b) => (a.title < b.title ? -1 : 1));
    if (starList.length) groups.unshift({ id: 'g-star', title: '星标朋友', items: starList });
    const letters = groups.map((g) => ({ id: g.id, text: g.id === 'g-star' ? '星' : g.title }));
    const count = groups.reduce((a, g) => a + (g.id === 'g-star' ? 0 : g.items.length), 0);
    const idxTop = Math.max(0, Math.round((this.listH - letters.length * this.letterH) / 2));
    this.letters = letters;
    this.idxTop = idxTop;
    this.setData({ groups, letters, count, listH: this.listH, idxTop, letterH: this.letterH });
  },
  onSearch(e) { this.setData({ kw: e.detail.value }); this.build(); },
  jump(e) { this.go(Number(e.currentTarget.dataset.i)); },
  // 手指在索引条上滑：按纵坐标换算到第几个字母
  onIdxMove(e) {
    const t = e.touches && e.touches[0];
    if (!t) return;
    const i = Math.floor((t.clientY - this.idxTop) / this.letterH);
    if (i >= 0 && i < this.letters.length && i !== this.lastIdx) this.go(i);
  },
  onIdxEnd() { this.lastIdx = -1; },
  go(i) {
    const l = this.letters[i];
    if (!l) return;
    this.lastIdx = i;
    this.setData({ intoView: l.id, bigLetter: l.text, curLetter: l.id });
    clearTimeout(this.bt);
    this.bt = setTimeout(() => this.setData({ bigLetter: '' }), 700);
  },
  open(e) { wx.navigateTo({ url: '/pages/contact/contact?id=' + e.currentTarget.dataset.id }); },
  newFriend() {
    wx.showModal({
      title: '新的朋友', content: '「冯晓晓」和「邓超越」请求添加你为朋友，是否全部通过？', confirmText: '全部通过',
      success: (r) => { if (r.confirm) { this.setData({ newFriends: 0 }); wx.showToast({ title: '已通过', icon: 'success' }); } }
    });
  },
  groupChat() { wx.showToast({ title: '暂无群聊', icon: 'none' }); }
});
