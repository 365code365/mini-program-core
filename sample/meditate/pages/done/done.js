const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const theme = require('../../utils/theme.js');

// 练习完成页：由播放页 / 呼吸页 redirectTo 过来，所以返回直接回到课程详情（或首页）
Page({
  data: { theme: 'light', title: '', subtitle: '', durText: '', streak: 0, count: 0, nextTitle: '', feel: '', feels: ['更平静了', '有点困了', '还是很乱', '身体放松了'], backText: '返回' },

  onLoad(q) {
    const sec = +q.sec || 0;
    const info = store.streakInfo();
    const st = store.stats();
    const durText = sec >= 60 ? Math.round(sec / 60) + ' 分钟' : sec + ' 秒';
    let title = '完成一次呼吸练习', subtitle = '完成 ' + (q.rounds || 4) + ' 轮 4-7-8 呼吸，心率正在慢下来';
    let nextTitle = '';
    if (q.type === 'course') {
      const c = data.course(q.id);
      const idx = +q.idx || 0;
      title = '完成「' + c.chapters[idx].title + '」';
      subtitle = c.title + ' · 第 ' + (idx + 1) + '/' + c.chapters.length + ' 节';
      if (idx + 1 < c.chapters.length) { nextTitle = c.chapters[idx + 1].title; this.next = { id: c.id, idx: idx + 1 }; }
    }
    this.type = q.type;
    this.setData({ title, subtitle, durText, streak: info.streak, count: st.count, nextTitle, backText: q.type === 'course' ? '返回课程' : '返回' });
  },

  onShow() { theme.apply(this, -1); },
  applyTheme() { theme.apply(this, -1); },

  onFeel(e) {
    this.setData({ feel: e.currentTarget.dataset.v });
    wx.showToast({ title: '已记下你的感受', icon: 'none' });
  },

  goNext() {
    wx.redirectTo({ url: '/pages/player/player?id=' + this.next.id + '&idx=' + this.next.idx });
  },

  writeMood() {
    wx.navigateTo({ url: '/pages/mood/mood?from=done' + (this.data.feel ? '&note=' + encodeURIComponent(this.data.feel) : '') });
  },

  goBack() {
    if (getCurrentPages().length > 1) wx.navigateBack();
    else wx.switchTab({ url: '/pages/index/index' });
  }
});
