// 写影评 / 改影评：打星（必填）、标签（多选）、正文（至少 6 个字）。?mid=影片 [&tid=电影票] [&id=已有影评]
const D = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { m: null, starList: [1, 2, 3, 4, 5], star: 0, starText: '点击星星打分', tags: [], text: '', editing: false, tid: '' },
  onLoad(q) {
    const old = q.id ? store.reviewById(q.id) : null;
    const m = D.getMovie(old ? old.mid : q.mid);
    const picked = old ? old.tags || [] : [];
    this.id = old ? old.id : '';
    this.setData({
      m, editing: !!old, tid: old ? old.tid || '' : q.tid || '',
      star: old ? old.star : 0, starText: old ? D.STAR_TEXT[old.star] : '点击星星打分', text: old ? old.text : '',
      tags: D.REVIEW_TAGS.map(name => ({ name, on: picked.indexOf(name) >= 0 }))
    });
    wx.setNavigationBarTitle({ title: old ? '修改影评' : '写影评' });
  },
  onStar(e) {
    const star = Number(e.currentTarget.dataset.n);
    this.setData({ star, starText: D.STAR_TEXT[star] });
  },
  onTag(e) {
    const i = Number(e.currentTarget.dataset.i);
    const on = !this.data.tags[i].on;
    if (on && this.data.tags.filter(t => t.on).length >= 3) { wx.showToast({ title: '最多选 3 个标签', icon: 'none' }); return; }
    this.setData({ [`tags[${i}].on`]: on });
  },
  onText(e) { this.setData({ text: e.detail.value }); },
  onSubmit() {
    const text = this.data.text.trim();
    if (!this.data.star) { wx.showToast({ title: '请先打分', icon: 'none' }); return; }
    if (text.length < 6) { wx.showToast({ title: '再多写几个字吧（至少 6 个字）', icon: 'none' }); return; }
    const r = { mid: this.data.m.id, tid: this.data.tid, star: this.data.star, text, tags: this.data.tags.filter(t => t.on).map(t => t.name) };
    if (this.id) r.id = this.id;
    store.saveReview(r);
    wx.showToast({ title: this.id ? '已修改' : '发布成功', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 600);
  },
  onDelete() {
    wx.showModal({
      title: '删除影评', content: '删除后不能恢复', confirmText: '删除', confirmColor: '#E8243F',
      success: r => {
        if (!r.confirm) return;
        store.removeReview(this.id);
        wx.showToast({ title: '已删除', icon: 'none' });
        setTimeout(() => wx.navigateBack(), 600);
      }
    });
  }
});
