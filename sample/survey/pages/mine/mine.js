const data = require('../../utils/data.js');

Page({
  data: { tabs: ['我发布的', '我参与的'], tIdx: 0, pub: [], joined: [], sum: { pub: 0, votes: 0, joined: 0 } },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    this.load();
  },
  load() {
    const t = data.today();
    const pub = data.getMine().map(m => {
      const total = m.counts.reduce((a, b) => a + b, 0);
      const order = m.counts.map((c, i) => ({ t: m.options[i], c })).sort((a, b) => b.c - a.c).slice(0, 2);
      return {
        id: m.id, title: m.title, created: m.created, deadline: m.deadline, anonymous: m.anonymous, multi: m.multi,
        total, ended: !!m.closed || m.deadline < t, deletable: m.id.charAt(0) === 'u',
        top: order.map(o => ({ t: o.t, pct: total ? Math.round(o.c / total * 100) : 0 }))
      };
    });
    const ans = data.getAnswers();
    const joined = Object.keys(ans).map(id => {
      const s = data.getSurvey(id);
      return { id, title: s.title, kind: s.kind, glyph: s.glyph, bg: s.bg, author: s.author, qCount: s.questions.length };
    });
    this.setData({ pub, joined, sum: { pub: pub.length, votes: pub.reduce((a, p) => a + p.total, 0), joined: joined.length } });
  },
  onTab(e) { this.setData({ tIdx: e.currentTarget.dataset.idx }); },
  onResult(e) { wx.navigateTo({ url: '/pages/result/result?id=' + e.currentTarget.dataset.id }); },
  onMore(e) {
    const p = this.data.pub[e.currentTarget.dataset.idx];
    const items = p.ended ? ['查看结果', '再发一次'] : ['查看结果', '提前结束投票', '邀请好友投票'];
    if (p.deletable) items.push('删除');
    wx.showActionSheet({ itemList: items, success: (r) => {
      const act = items[r.tapIndex];
      if (act === '查看结果') wx.navigateTo({ url: '/pages/result/result?id=' + p.id });
      else if (act === '再发一次') wx.switchTab({ url: '/pages/create/create' });
      else if (act === '邀请好友投票') wx.showToast({ title: '已生成邀请卡片', icon: 'none' });
      else if (act === '提前结束投票') this.update(p.id, m => Object.assign(m, { closed: true }), '投票已结束');
      else if (act === '删除') {
        wx.showModal({ title: '删除投票', content: '删除后不可恢复，确定删除吗？', confirmColor: '#F5455C', success: (m) => {
          if (!m.confirm) return;
          wx.setStorageSync('sv_mine', (wx.getStorageSync('sv_mine') || []).filter(x => x.id !== p.id));
          this.load();
          wx.showToast({ title: '已删除', icon: 'none' });
        } });
      }
    } });
  },
  update(id, fn, msg) {
    const list = wx.getStorageSync('sv_mine') || [];
    const hit = list.find(x => x.id === id);
    if (hit) { fn(hit); wx.setStorageSync('sv_mine', list); }
    else {
      // 内置示例投票：把「已结束」状态单独记一份
      const closed = wx.getStorageSync('sv_closed') || [];
      wx.setStorageSync('sv_closed', closed.concat([id]));
    }
    this.load();
    wx.showToast({ title: msg, icon: 'none' });
  },
  onJoined(e) { wx.navigateTo({ url: '/pages/result/result?id=' + e.currentTarget.dataset.id }); },
  onGo() { wx.switchTab({ url: '/pages/index/index' }); },
  onCreate() { wx.switchTab({ url: '/pages/create/create' }); }
});
