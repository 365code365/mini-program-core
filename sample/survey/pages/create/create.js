let seq = 3;

function today() {
  const d = new Date();
  const p = n => (n < 10 ? '0' + n : '' + n);
  return d.getFullYear() + '-' + p(d.getMonth() + 1) + '-' + p(d.getDate());
}
function addDays(s, n) {
  const d = new Date(s.replace(/-/g, '/'));
  d.setDate(d.getDate() + n);
  const p = x => (x < 10 ? '0' + x : '' + x);
  return d.getFullYear() + '-' + p(d.getMonth() + 1) + '-' + p(d.getDate());
}

Page({
  data: {
    title: '', desc: '',
    options: [{ key: 1, text: '' }, { key: 2, text: '' }],
    deadline: '', today: '', multi: false, anonymous: true,
    validCount: 0, ready: false
  },
  onLoad() {
    const t = today();
    this.setData({ today: t, deadline: addDays(t, 7) });
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
  },
  check() {
    const validCount = this.data.options.filter(o => o.text.trim()).length;
    this.setData({ validCount, ready: !!this.data.title.trim() && validCount >= 2 });
  },
  onTitle(e) { this.setData({ title: e.detail.value }); this.check(); },
  onDesc(e) { this.setData({ desc: e.detail.value }); },
  onOpt(e) { this.setData({ ['options[' + e.currentTarget.dataset.idx + '].text']: e.detail.value }); this.check(); },
  onAdd() {
    if (this.data.options.length >= 8) { wx.showToast({ title: '最多 8 个选项', icon: 'none' }); return; }
    this.setData({ options: this.data.options.concat([{ key: ++seq, text: '' }]) });
  },
  onDel(e) {
    if (this.data.options.length <= 2) { wx.showToast({ title: '至少保留 2 个选项', icon: 'none' }); return; }
    const i = e.currentTarget.dataset.idx;
    this.setData({ options: this.data.options.filter((_, k) => k !== i) });
    this.check();
  },
  onDate(e) { this.setData({ deadline: e.detail.value }); },
  onMultiSw(e) { this.setData({ multi: e.detail.value }); },
  onAnon(e) { this.setData({ anonymous: e.detail.value }); },
  onPublish() {
    const title = this.data.title.trim();
    const opts = this.data.options.map(o => o.text.trim()).filter(Boolean);
    if (!title) { wx.showToast({ title: '请填写投票标题', icon: 'none' }); return; }
    if (opts.length < 2) { wx.showToast({ title: '至少填写 2 个选项', icon: 'none' }); return; }
    if (new Set(opts).size !== opts.length) { wx.showToast({ title: '选项不能重复', icon: 'none' }); return; }
    wx.showModal({
      title: '发布投票',
      content: '「' + title + '」共 ' + opts.length + ' 个选项，' + this.data.deadline + ' 截止' + (this.data.anonymous ? '，匿名' : ''),
      confirmText: '发布',
      confirmColor: '#6C5CE7',
      success: (r) => {
        if (!r.confirm) return;
        const t = today();
        const item = { id: 'u' + Date.now(), kind: '投票', title, options: opts, counts: opts.map(() => 0), deadline: this.data.deadline, anonymous: this.data.anonymous, multi: this.data.multi, created: t.slice(5) };
        wx.setStorageSync('sv_mine', [item].concat(wx.getStorageSync('sv_mine') || []));
        this.setData({ title: '', desc: '', options: [{ key: ++seq, text: '' }, { key: ++seq, text: '' }], validCount: 0, ready: false });
        wx.showToast({ title: '发布成功', icon: 'success' });
        setTimeout(() => wx.switchTab({ url: '/pages/mine/mine' }), 700);
      }
    });
  }
});
