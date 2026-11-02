const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const SPEEDS = [1, 1.25, 1.5, 2];
function mmss(s) { s = Math.max(0, Math.round(s)); return store.pad(Math.floor(s / 60)) + ':' + store.pad(s % 60); }
// 课时学习：示例里没有视频素材，播放器按「每 0.5 秒走 15 秒 × 倍速」模拟进度，走完自动记为学完；
// 也可以直接点「标记学完」。没买的课只能看试看课时。
Page({
  data: { c: null, list: [], cur: null, idx: 0, owned: false, playing: false, pos: 0, posTx: '00:00', durTx: '00:00', pct: 0,
    speeds: SPEEDS, speed: 1, notes: [], noteText: '', formSeq: 0, progressTx: '' },
  onLoad(q) {
    const c = data.courseOf(q.cid) || data.courses[0];
    this.c = c;
    this.setData({ c: { id: c.id, title: c.title, cover: c.cover, mark: c.mark } });
    this.open(q.lid || store.progress(c.id).last);
  },
  onHide() { this.pause(); },
  onUnload() { this.pause(); },
  refreshList() {
    const p = store.progress(this.c.id);
    const owned = store.owned(this.c.id);
    const cur = this.data.cur;
    const list = data.flatLessons(this.c).map(l => ({ id: l.id, title: l.title, mins: l.mins, chapter: l.chapter, free: l.free,
      done: p.doneIds.indexOf(l.id) >= 0, locked: !owned && !l.free, on: cur && cur.id === l.id }));
    this.setData({ list, owned, progressTx: owned ? '已学 ' + p.done + '/' + p.total + ' 节' : '试看中 · 购买后解锁全部 ' + p.total + ' 节' });
    return list;
  },
  open(lid) {
    const flat = data.flatLessons(this.c);
    let idx = flat.findIndex(l => l.id === lid);
    if (idx < 0) idx = 0;
    const l = flat[idx];
    if (!store.owned(this.c.id) && !l.free) {
      // 直接打开了没解锁的课时：先停在第一节试看课上
      if (!this.data.cur) { const f = flat.find(x => x.free); if (f) this.open(f.id); }
      this.askBuy('「' + l.title + '」需要购买后学习');
      return;
    }
    this.pause();
    this.setData({ cur: { id: l.id, title: l.title, chapter: l.chapter, mins: l.mins, free: l.free }, idx, pos: 0, pct: 0, posTx: '00:00', durTx: mmss(l.mins * 60), noteText: '', formSeq: this.data.formSeq + 1 });
    wx.setNavigationBarTitle({ title: l.title });
    this.refreshList();
    this.refreshNotes();
  },
  askBuy(content) {
    wx.showModal({ title: '购买后解锁', content: content + '，试看课时可以免费学习', confirmText: '去购买', success: r => {
      if (r.confirm) wx.redirectTo({ url: '/pages/order/order?id=' + this.c.id });
    } });
  },
  togglePlay() { if (this.data.playing) this.pause(); else this.play(); },
  play() {
    if (this.timer) return;
    this.setData({ playing: true });
    this.timer = setInterval(() => this.tick(), 500);
  },
  pause() {
    if (this.timer) { clearInterval(this.timer); this.timer = null; }
    if (this.data.playing) this.setData({ playing: false });
  },
  tick() {
    const dur = this.data.cur.mins * 60;
    const pos = Math.min(dur, this.data.pos + 15 * this.data.speed);
    this.setData({ pos, posTx: mmss(pos), pct: Math.round(pos / dur * 100) });
    if (pos >= dur) this.finish();
  },
  onSpeed() {
    const i = (SPEEDS.indexOf(this.data.speed) + 1) % SPEEDS.length;
    this.setData({ speed: SPEEDS[i] });
  },
  // 学完本节（播完或手动标记）
  finish() {
    this.pause();
    const dur = this.data.cur.mins * 60;
    this.setData({ pos: dur, posTx: mmss(dur), pct: 100 });
    if (!store.owned(this.c.id)) { this.askBuy('试看结束'); return; }
    const r = store.completeLesson(this.c.id, this.data.cur.id);
    this.refreshList();
    if (r.finished && r.first) {
      wx.showModal({ title: '恭喜学完全部课程', content: '「' + this.c.title + '」的结业证书已发放，可在「我的 - 我的证书」查看', confirmText: '看证书', success: x => {
        if (x.confirm) wx.navigateTo({ url: '/pages/cert/cert?id=' + this.c.id });
      } });
      return;
    }
    wx.showToast({ title: r.first ? '本节已学完 +' + this.data.cur.mins + ' 分钟' : '这一节之前学过了', icon: 'none' });
  },
  prev() { if (this.data.idx > 0) this.open(this.data.list[this.data.idx - 1].id); else wx.showToast({ title: '已经是第一节', icon: 'none' }); },
  next() {
    const n = this.data.list[this.data.idx + 1];
    if (!n) { wx.showToast({ title: '已经是最后一节', icon: 'none' }); return; }
    this.open(n.id);
  },
  onPick(e) { this.open(String(e.currentTarget.dataset.id)); },
  // ---------- 笔记 ----------
  refreshNotes() {
    const notes = store.getNotes().filter(n => n.cid === this.c.id && n.lid === this.data.cur.id);
    this.setData({ notes });
  },
  onNote(e) { this.setData({ noteText: e.detail.value }); },
  addNote() {
    const text = (this.data.noteText || '').trim();
    if (!text) { wx.showToast({ title: '先写点内容', icon: 'none' }); return; }
    const all = store.getNotes();
    all.unshift({ id: 'n' + Date.now(), cid: this.c.id, ctitle: this.c.title, lid: this.data.cur.id, ltitle: this.data.cur.title, text, pos: this.data.posTx, time: store.stamp(new Date()), day: store.today() });
    store.saveNotes(all);
    this.setData({ noteText: '', formSeq: this.data.formSeq + 1 });
    this.refreshNotes();
    wx.showToast({ title: '笔记已保存', icon: 'none' });
  },
  delNote(e) {
    const id = String(e.currentTarget.dataset.id);
    wx.showModal({ title: '删除这条笔记？', confirmText: '删除', confirmColor: '#f0577a', success: r => {
      if (!r.confirm) return;
      store.saveNotes(store.getNotes().filter(n => n.id !== id));
      this.refreshNotes();
    } });
  }
});
