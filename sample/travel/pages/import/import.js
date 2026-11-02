// 导入好友分享的行程：分享文字（store.shareText 生成）里带着城市、日期、每天的景点与预算，原样解析回来
const T = require('../../utils/store.js');
const { BUDGET_ITEMS } = require('../../utils/data.js');
const host = require('../../utils/host.js');
Page({
  data: { text: '', seq: 0, preview: null },
  onLoad(q) {
    // 从「转发」卡片进来：分享文字在 query 里
    if (q && q.text) this.setText(decodeURIComponent(q.text));
    else this.paste(true);
  },
  // 读剪贴板；quiet：一进页面自动读一次，读到的不是行程就不提示
  paste(quiet) {
    host.paste().then(text => {
      if (T.parseShare(text)) { this.setText(text); return; }
      if (quiet !== true) wx.showToast({ title: text ? '剪贴板里不是行程分享文字' : '剪贴板是空的，请手动粘贴', icon: 'none' });
    });
  },
  // 程序改 textarea 的内容：换个 id 让它重建（用户打过字的输入框，setData 改 value 画面不更新）
  setText(text) { this.setData({ text, seq: this.data.seq + 1 }); this.preview(); },
  onText(e) { this.setData({ text: e.detail.value }); this.preview(); },
  preview() {
    const p = T.parseShare(this.data.text);
    if (!p) { this.setData({ preview: null }); return; }
    this.parsed = p;
    const parts = BUDGET_ITEMS.filter(b => Number(p.budget.items[b.k] || 0) > 0).map(b => b.t + ' ¥' + p.budget.items[b.k]);
    this.setData({
      preview: {
        title: p.title, city: p.city, range: T.short(p.start) + ' - ' + T.short(p.end), days: p.plan.length, people: p.people, type: p.type,
        dayList: p.plan.map((d, i) => ({ i, label: '第' + (i + 1) + '天', names: d.length ? d.map(s => s.name).join('、') : '自由活动' })),
        budget: p.budget.total || parts.length ? '预算' + (p.budget.total ? ' ¥' + p.budget.total : '') + (parts.length ? '：' + parts.join('、') : '') : ''
      }
    });
  },
  doImport() {
    if (!this.data.preview) { wx.showToast({ title: '先粘贴好友分享的行程文字', icon: 'none' }); return; }
    const t = T.importTrip(this.parsed);
    wx.showToast({ title: '已导入「' + t.title + '」', icon: 'none' });
    wx.redirectTo({ url: '/pages/detail/detail?id=' + t.id });
  }
});
