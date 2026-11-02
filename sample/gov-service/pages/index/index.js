const { THEMES, DEPTS, ITEMS, themeOf, getItem } = require('../../utils/items');
const store = require('../../utils/store');
const draft = require('../../utils/draft');
const guide = require('../../utils/guide');

function decorate(list) {
  return list.map(i => ({
    id: i.id, name: i.name, dept: i.dept, level: i.level, color: themeOf(i.theme).color,
    promise: i.promise ? `${i.promise} 个工作日` : '即时办结', online: i.online
  }));
}

Page({
  data: {
    themes: THEMES,
    depts: DEPTS,
    mode: 'theme',          // theme 按主题 / dept 按部门
    cat: THEMES[0].id,
    list: [],
    hot: decorate(ITEMS.filter(i => i.hot)),
    keyword: '',
    results: [],
    draft: null,
    stat: { total: ITEMS.length, online: ITEMS.filter(i => i.online).length }
  },
  onLoad() { this.refreshList(); },
  onShow() {
    store.syncTab(this, 0);
    draft.load((d) => {
      if (!d) { this.setData({ draft: null }); return; }
      const item = getItem(d.form.itemId);
      this.setData({ draft: { name: item ? item.name : '申报事项', step: d.step + 1, time: store.fmtTime(d.savedAt) } });
    });
  },
  refreshList() {
    const { mode, cat } = this.data;
    const src = ITEMS.filter(i => (mode === 'theme' ? i.theme === cat : i.dept === cat));
    this.setData({ list: decorate(src) });
  },
  switchMode(e) {
    const mode = e.currentTarget.dataset.mode;
    if (mode === this.data.mode) return;
    this.setData({ mode, cat: mode === 'theme' ? THEMES[0].id : DEPTS[0] }, () => this.refreshList());
  },
  pickCat(e) { this.setData({ cat: e.currentTarget.dataset.id }, () => this.refreshList()); },
  onSearch(e) {
    const kw = (e.detail.value || '').trim();
    const results = kw ? decorate(ITEMS.filter(i => i.name.indexOf(kw) >= 0 || i.dept.indexOf(kw) >= 0)) : [];
    this.setData({ keyword: kw, results });
  },
  clearSearch() { this.setData({ keyword: '', results: [] }); },
  openItem(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  continueDraft() { wx.navigateTo({ url: '/pages/apply/apply?draft=1' }); },
  dropDraft() {
    wx.showModal({
      title: '删除草稿', content: '删除后已填写的内容和已选材料都无法恢复，确定删除吗？', confirmColor: '#dc2626',
      success: (r) => { if (r.confirm) draft.remove(() => { this.setData({ draft: null }); wx.showToast({ title: '草稿已删除', icon: 'none' }); }); }
    });
  },
  goAppoint() { wx.navigateTo({ url: '/pages/appoint/appoint' }); },
  goProgress() { wx.navigateTo({ url: '/pages/progress/progress' }); },
  goLicense() { wx.switchTab({ url: '/pages/license/license' }); },
  downloadGuide() { guide.open(''); }
});
