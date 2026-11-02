const store = require('../../utils/store.js');
const data = require('../../utils/data.js');
const theme = require('../../utils/theme.js');

// 按时段问候 + 推荐：深夜 / 晚上推助眠，白天推减压与专注
function greeting(h) {
  if (h >= 5 && h < 9) return { hi: '早上好', tip: '新的一天，从一次深呼吸开始', recTitle: '晨间唤醒', cats: ['intro', 'focus', 'body'] };
  if (h >= 9 && h < 12) return { hi: '上午好', tip: '忙碌间隙，给自己三分钟安静', recTitle: '为你推荐', cats: ['focus', 'relax', 'intro'] };
  if (h >= 12 && h < 14) return { hi: '午安', tip: '午休十分钟，下午更有精神', recTitle: '午间充电', cats: ['relax', 'body', 'focus'] };
  if (h >= 14 && h < 18) return { hi: '下午好', tip: '肩膀有点紧？试试身体扫描', recTitle: '为你推荐', cats: ['relax', 'emotion', 'body'] };
  if (h >= 18 && h < 22) return { hi: '晚上好', tip: '放下手机前，和今天好好告别', recTitle: '睡前放松', cats: ['emotion', 'sleep', 'relax'] };
  return { hi: '夜深了', tip: '睡不着也没关系，跟着引导慢慢放松', recTitle: '助眠推荐', cats: ['sleep', 'body', 'relax'] };
}

const WEEK = ['日', '一', '二', '三', '四', '五', '六'];

Page({
  data: {
    theme: 'light',
    dateText: '',
    greet: {},
    streak: { streak: 0, best: 0, totalDays: 0, week: [], todayDone: false },
    badgeAnim: null,
    recs: [],
    lastCourse: null,
    lastChapter: '',
    quick: [
      { key: 'breath', name: '呼吸练习', mark: '息', bg: 'linear-gradient(135deg, #7fd3c3, #3fa58f)' },
      { key: 'noise', name: '白噪音', mark: '雨', bg: 'linear-gradient(135deg, #8db6ff, #5b7fe0)' },
      { key: 'sleep', name: '睡眠记录', mark: '眠', bg: 'linear-gradient(135deg, #9a8ff0, #5f55c9)' },
      { key: 'mood', name: '心情日记', mark: '心', bg: 'linear-gradient(135deg, #ffb59a, #f07e6a)' }
    ]
  },

  onLoad() {
    const now = new Date();
    const g = greeting(now.getHours());
    const recs = g.cats.map(c => data.COURSES.find(x => x.cat === c)).filter(Boolean);
    this.setData({ greet: g, recs, dateText: (now.getMonth() + 1) + '月' + now.getDate() + '日 星期' + WEEK[now.getDay()] });
    this._lastStreak = -1;
  },

  onShow() {
    this.applyTheme();
    const streak = store.streakInfo();
    const last = store.last();
    const c = last ? data.course(last.courseId) : null;
    this.setData({
      streak,
      lastCourse: c,
      lastChapter: c ? '第 ' + (last.idx + 1) + ' 节 · ' + c.chapters[last.idx].title : ''
    });
    // 打卡数变了（刚练完回来）：徽章弹一下
    if (this._lastStreak >= 0 && streak.streak > this._lastStreak) this.popBadge();
    this._lastStreak = streak.streak;
  },

  applyTheme() { theme.apply(this, 0); },

  popBadge() {
    const a = wx.createAnimation({ duration: 260, timingFunction: 'ease-out' });
    a.scale(1.35).rotate(-12).step();
    a.scale(1).rotate(0).step({ duration: 200 });
    this.setData({ badgeAnim: a.export() });
  },

  goPractice() {
    wx.showModal({
      title: '今天还没有练习',
      content: '完成任意一节课程或一次呼吸练习即可打卡。先做一次 4-7-8 呼吸？',
      confirmText: '去呼吸',
      cancelText: '选课程',
      success: res => {
        if (res.confirm) wx.navigateTo({ url: '/pages/breath/breath' });
        else if (res.cancel) wx.switchTab({ url: '/pages/category/category' });
      }
    });
  },

  onQuick(e) {
    const k = e.currentTarget.dataset.key;
    if (k === 'breath') wx.navigateTo({ url: '/pages/breath/breath' });
    else if (k === 'noise') wx.navigateTo({ url: '/pages/player/player?id=c3&idx=0&noise=1' });
    else if (k === 'sleep') wx.switchTab({ url: '/pages/sleep/sleep' });
    else if (k === 'mood') wx.navigateTo({ url: '/pages/mood/mood' });
  },

  onContinue() {
    const last = store.last();
    if (last) wx.navigateTo({ url: '/pages/player/player?id=' + last.courseId + '&idx=' + last.idx });
  },

  openCourse(e) {
    wx.navigateTo({ url: '/pages/course/course?id=' + e.currentTarget.dataset.id });
  },

  goAll() { wx.switchTab({ url: '/pages/category/category' }); }
});
