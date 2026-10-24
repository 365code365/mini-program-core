const data = require('../../utils/data.js');

// 演示用的历史投递（用户新投递的会插在前面）
const HISTORY = [
  { id: 'j3', time: '10月22日', status: 2 },
  { id: 'j4', time: '10月20日', status: 1 },
  { id: 'j5', time: '10月16日', status: 3 }
];

Page({
  data: {
    resume: {},
    avatarCh: '我',
    stats: { applied: 0, viewed: 0, interview: 0, fav: 0 },
    records: [],
    statusText: ['已投递', '被查看', '邀面试', '不合适'],
    menus: [
      { name: '在线简历', glyph: '历', bg: 'linear-gradient(135deg, #6FA0FF, #2E6BFF)', extra: '', url: '/pages/resume/resume' },
      { name: '关注的公司', glyph: '企', bg: 'linear-gradient(135deg, #5FD99A, #19BE6B)', extra: '' },
      { name: '求职偏好', glyph: '向', bg: 'linear-gradient(135deg, #FFB067, #FF7A1A)', extra: '' },
      { name: '隐私设置', glyph: '私', bg: 'linear-gradient(135deg, #B28CFF, #7B5CFF)', extra: '' },
      { name: '帮助与反馈', glyph: '助', bg: 'linear-gradient(135deg, #7ED6E8, #26A8C7)', extra: '' }
    ]
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    const mine = wx.getStorageSync('jobs_applied') || [];
    const seen = {};
    const all = mine.concat(HISTORY).filter(a => (seen[a.id] ? false : (seen[a.id] = true)));
    const records = all.map(a => { const j = data.getJob(a.id); return { id: a.id, title: j.title, co: j.co, time: a.time, status: a.status }; });
    const favs = wx.getStorageSync('jobs_fav') || [];
    const resume = wx.getStorageSync('jobs_resume') || {};
    const follows = wx.getStorageSync('jobs_follow') || [];
    this.setData({
      resume,
      avatarCh: (resume.name || '我').slice(0, 1),
      records,
      stats: {
        applied: records.length,
        viewed: records.filter(r => r.status >= 1).length,
        interview: records.filter(r => r.status === 2).length,
        fav: favs.length
      },
      'menus[1].extra': follows.length ? follows.length + ' 家' : ''
    });
  },
  onResume() { wx.navigateTo({ url: '/pages/resume/resume' }); },
  onJob(e) { wx.navigateTo({ url: '/pages/job/job?id=' + e.currentTarget.dataset.id }); },
  onStat() { wx.showToast({ title: '下拉查看投递记录', icon: 'none' }); },
  onMenu(e) {
    const m = this.data.menus[e.currentTarget.dataset.idx];
    if (m.url) wx.navigateTo({ url: m.url });
    else wx.showToast({ title: m.name + '即将开放', icon: 'none' });
  }
});
