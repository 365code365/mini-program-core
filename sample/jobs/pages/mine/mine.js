const S = require('../../utils/store.js');
const CLS = ['s0', 's1', 's2', 's3'];
Page({
  data: {
    resume: {},
    avatarCh: '我',
    intentText: '',
    stats: { applied: 0, viewed: 0, interview: 0, fav: 0 },
    records: [],
    total: 0,
    menus: []
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    const apps = S.getApps();
    const resume = S.getResume();
    const it = S.getIntent();
    const pending = S.pendingInterviews();
    const follows = S.getFollows();
    const blocked = S.getSettings().blocked;
    this.setData({
      resume,
      avatarCh: (resume.name || '我').slice(0, 1),
      intentText: S.JOB_STATUS[it.status],
      // 卡片里放最近 3 条，其余进投递记录
      records: apps.slice(0, 3).map(a => Object.assign({}, a, { cls: CLS[a.status] })),
      total: apps.length,
      stats: {
        applied: apps.length,
        viewed: apps.filter(r => r.status === 1).length,
        interview: S.getInterviews().length,
        fav: S.getFavs().length
      },
      menus: [
        { key: 'resume', name: '在线简历', glyph: '历', bg: 'linear-gradient(135deg, #6FA0FF, #2E6BFF)', extra: resume.open === false ? '未公开' : '已公开' },
        { key: 'interviews', name: '面试邀约', glyph: '面', bg: 'linear-gradient(135deg, #5FD99A, #19BE6B)', extra: pending ? pending + ' 个待确认' : '' },
        { key: 'follow', name: '关注的公司', glyph: '企', bg: 'linear-gradient(135deg, #7ED6E8, #26A8C7)', extra: follows.length ? follows.length + ' 家' : '' },
        { key: 'intent', name: '求职偏好', glyph: '向', bg: 'linear-gradient(135deg, #FFB067, #FF7A1A)', extra: it.intent },
        { key: 'settings', name: '隐私设置', glyph: '私', bg: 'linear-gradient(135deg, #B28CFF, #7B5CFF)', extra: blocked.length ? '已屏蔽 ' + blocked.length + ' 家' : '' },
        { key: 'help', name: '帮助与反馈', glyph: '助', bg: 'linear-gradient(135deg, #FF9AA8, #F5455C)', extra: '' }
      ]
    });
  },
  onResume() { wx.navigateTo({ url: '/pages/resume/resume' }); },
  onJob(e) { wx.navigateTo({ url: '/pages/job/job?id=' + e.currentTarget.dataset.id }); },
  // 已投递 / 被查看 → 投递记录对应分段；面试邀请 → 面试邀约；收藏职位 → 我的收藏
  onStat(e) {
    const k = Number(e.currentTarget.dataset.k);
    if (k === 0) wx.navigateTo({ url: '/pages/applications/applications' });
    else if (k === 1) wx.navigateTo({ url: '/pages/applications/applications?tab=2' });
    else if (k === 2) wx.navigateTo({ url: '/pages/interviews/interviews' });
    else wx.navigateTo({ url: '/pages/favs/favs' });
  },
  onAllRecords() { wx.navigateTo({ url: '/pages/applications/applications' }); },
  onMenu(e) {
    const key = this.data.menus[Number(e.currentTarget.dataset.idx)].key;
    const url = {
      resume: '/pages/resume/resume', interviews: '/pages/interviews/interviews', follow: '/pages/favs/favs?tab=1',
      intent: '/pages/intent/intent', settings: '/pages/settings/settings', help: '/pages/help/help'
    }[key];
    wx.navigateTo({ url });
  }
});
