// 错题本：测验答错的词，按错的次数排；重做错题时答对即移出
const store = require('../../utils/store.js');
const books = require('../../utils/books.js');

Page({
  data: { list: [] },
  onShow() { this.load(); },
  load() {
    const list = store.wrongs().map((x) => Object.assign({}, books.find(x.w) || { w: x.w, mean: '' }, { times: x.times, last: x.last.slice(5) }))
      .sort((a, b) => b.times - a.times);
    this.setData({ list });
  },
  goWord(e) { wx.navigateTo({ url: '/pages/word/word?w=' + e.currentTarget.dataset.w }); },
  onRemove(e) {
    store.removeWrong(e.currentTarget.dataset.w);
    this.load();
    wx.showToast({ title: '已移出错题本', icon: 'none' });
  },
  onClear() {
    wx.showModal({ title: '清空错题本？', content: '共 ' + this.data.list.length + ' 个错题，清空后不能恢复。', confirmColor: '#f0566a', success: (r) => {
      if (r.confirm) { store.clearWrong(); this.load(); }
    } });
  },
  onRedo() { wx.navigateTo({ url: '/pages/quiz/quiz?src=wrong' }); },
  goQuiz() { wx.redirectTo({ url: '/pages/quiz/quiz?src=book' }); }
});
