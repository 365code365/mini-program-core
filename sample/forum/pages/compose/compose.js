const store = require('../../utils/store.js');
const TITLE_MIN = 5, TITLE_MAX = 30, BODY_MIN = 10, BODY_MAX = 1000, TAG_MAX = 3;
const DRAFT = 'forum_draft';

Page({
  data: {
    mode: 'new', title: '', body: '', titleLen: 0, bodyLen: 0, titleMax: TITLE_MAX, bodyMax: BODY_MAX,
    circles: store.CIRCLES.map((c) => c.name), circleIdx: -1, tags: [], tagMax: TAG_MAX, picked: 0,
    errors: {}, tried: false, draftTip: '', canPost: false
  },
  onLoad(q) {
    if (q.id) {
      // 编辑已发布的帖子：不读写草稿
      const p = store.posts().find((x) => x.id === q.id);
      if (!p) return;
      this.editId = q.id;
      wx.setNavigationBarTitle({ title: '编辑帖子' });
      this.fill({ title: p.title, body: p.body, circle: p.circle, tags: p.tags }, 'edit');
      return;
    }
    wx.setNavigationBarTitle({ title: '发帖' });
    const d = wx.getStorageSync(DRAFT);
    if (d && (d.title || d.body)) {
      this.fill(d, 'new');
      this.setData({ draftTip: '已恢复上次未发布的草稿' });
    } else this.fill({ title: '', body: '', circle: '', tags: [] }, 'new');
  },
  onUnload() { this.saveDraft(); },
  onHide() { this.saveDraft(); },
  fill(d, mode) {
    const ci = store.CIRCLES.findIndex((c) => c.id === d.circle);
    this.setData({
      mode, title: d.title, body: d.body, titleLen: d.title.length, bodyLen: d.body.length, circleIdx: ci,
      tags: store.TAGS.map((t) => ({ name: t, on: (d.tags || []).indexOf(t) >= 0 })), picked: (d.tags || []).length
    });
    this.check();
  },
  current() {
    return {
      title: this.data.title, body: this.data.body,
      circle: this.data.circleIdx >= 0 ? store.CIRCLES[this.data.circleIdx].id : '',
      tags: this.data.tags.filter((t) => t.on).map((t) => t.name)
    };
  },
  saveDraft() {
    if (this.editId || this.published) return;
    const d = this.current();
    if (d.title.trim() || d.body.trim()) wx.setStorageSync(DRAFT, d);
    else wx.removeStorageSync(DRAFT);
  },
  // 输入时 600ms 防抖自动存草稿
  autosave() {
    if (this.editId) return;
    clearTimeout(this.st);
    this.st = setTimeout(() => { this.saveDraft(); this.setData({ draftTip: '草稿已自动保存' }); }, 600);
  },
  check() {
    const t = this.data.title.trim(), b = this.data.body.trim();
    const e = {};
    if (t.length < TITLE_MIN) e.title = '标题至少 ' + TITLE_MIN + ' 个字';
    else if (t.length > TITLE_MAX) e.title = '标题最多 ' + TITLE_MAX + ' 个字';
    if (b.length < BODY_MIN) e.body = '正文至少 ' + BODY_MIN + ' 个字，还差 ' + (BODY_MIN - b.length) + ' 个';
    else if (b.length > BODY_MAX) e.body = '正文超出 ' + (b.length - BODY_MAX) + ' 个字';
    if (this.data.circleIdx < 0) e.circle = '请选择要发布到的圈子';
    const ci = this.data.circleIdx;
    this.setData({ errors: e, canPost: Object.keys(e).length === 0, pickerVal: ci < 0 ? 0 : ci, circleText: ci < 0 ? '' : store.CIRCLES[ci].name + '圈' });
    return e;
  },
  onTitle(e) { this.setData({ title: e.detail.value, titleLen: e.detail.value.length }); this.check(); this.autosave(); },
  onBody(e) { this.setData({ body: e.detail.value, bodyLen: e.detail.value.length }); this.check(); this.autosave(); },
  onCircle(e) { this.setData({ circleIdx: Number(e.detail.value) }); this.check(); this.autosave(); },
  toggleTag(e) {
    const i = Number(e.currentTarget.dataset.i);
    const t = this.data.tags[i];
    if (!t.on && this.data.picked >= TAG_MAX) return wx.showToast({ title: '最多选 ' + TAG_MAX + ' 个标签', icon: 'none' });
    const d = {};
    d['tags[' + i + '].on'] = !t.on;
    d.picked = this.data.picked + (t.on ? -1 : 1);
    this.setData(d);
    this.autosave();
  },
  clearDraft() {
    wx.showModal({
      title: '清空内容', content: '标题、正文、圈子与标签都会被清空',
      success: (r) => {
        if (!r.confirm) return;
        wx.removeStorageSync(DRAFT);
        this.fill({ title: '', body: '', circle: '', tags: [] }, this.data.mode);
        this.setData({ draftTip: '', tried: false });
      }
    });
  },
  submit() {
    this.setData({ tried: true });
    const e = this.check();
    const keys = Object.keys(e);
    if (keys.length) return wx.showToast({ title: e[keys[0]], icon: 'none' });
    const d = this.current();
    d.title = d.title.trim(); d.body = d.body.trim();
    wx.showLoading({ title: this.editId ? '保存中' : '发布中' });
    setTimeout(() => {
      wx.hideLoading();
      const all = store.posts();
      if (this.editId) {
        const p = all.find((x) => x.id === this.editId);
        Object.assign(p, d, { edited: true });
        store.savePosts(all);
        const ch = this.getOpenerEventChannel && this.getOpenerEventChannel();
        if (ch && ch.emit) ch.emit('updated', { id: p.id, title: p.title });
        this.published = true;
        wx.navigateBack();
        return;
      }
      const p = Object.assign({ id: 'my' + Date.now(), author: store.ME.name, color: store.ME.color, mine: true, pics: 0, likes: 0, comments: 0, time: Date.now() }, d);
      all.unshift(p);
      store.savePosts(all);
      wx.removeStorageSync(DRAFT);
      this.published = true;
      wx.showToast({ title: '发布成功', icon: 'success' });
      setTimeout(() => wx.navigateBack(), 600);
    }, 500);
  }
});
