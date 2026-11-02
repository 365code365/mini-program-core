const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const theme = require('../../utils/theme.js');

function decorate(list) {
  return list.map(e => {
    const m = data.MOODS.find(x => x.key === e.mood) || data.MOODS[0];
    return Object.assign({}, e, { moodName: m.name, color: m.color });
  });
}

Page({
  data: { theme: 'light', moods: data.MOODS, mood: '', tagOpts: [], text: '', err: '', list: [] },

  onLoad(q) {
    this.resetForm(q && q.note ? decodeURIComponent(q.note) + '。' : '');
    this.setData({ list: decorate(store.moods()) });
  },

  onShow() { theme.apply(this, -1); },
  applyTheme() { theme.apply(this, -1); },

  resetForm(text) {
    this.setData({ mood: '', text: text || '', err: '', tagOpts: data.MOOD_TAGS.map(n => ({ name: n, on: false })) });
  },

  pickMood(e) { this.setData({ mood: e.currentTarget.dataset.k, err: '' }); },

  onTags(e) {
    const on = e.detail.value;
    this.setData({ tagOpts: this.data.tagOpts.map(t => ({ name: t.name, on: on.indexOf(t.name) >= 0 })) });
  },

  onInput(e) { this.setData({ text: e.detail.value }); },

  save() {
    if (!this.data.mood) return this.setData({ err: '先选一个最接近的心情' });
    const text = this.data.text.trim();
    if (text.length > 300) return this.setData({ err: '最多 300 字' });
    const now = new Date();
    const entry = {
      id: 'm' + now.getTime(), ts: now.getTime(), date: store.dateStr(now), time: store.pad(now.getHours()) + ':' + store.pad(now.getMinutes()),
      mood: this.data.mood, tags: this.data.tagOpts.filter(t => t.on).map(t => t.name), text
    };
    const list = [entry].concat(store.moods());
    store.saveMoods(list);
    this.setData({ list: decorate(list) });
    this.resetForm('');
    wx.showToast({ title: '日记已保存', icon: 'success' });
  },

  remove(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '删除这篇日记？',
      content: '删除后无法恢复',
      confirmColor: '#e0604f',
      success: res => {
        if (!res.confirm) return;
        const list = store.moods().filter(x => x.id !== id);
        store.saveMoods(list);
        this.setData({ list: decorate(list) });
      }
    });
  }
});
