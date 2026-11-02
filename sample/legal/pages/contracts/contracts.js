const C = require('../../utils/contracts.js');
const S = require('../../utils/store.js');

Page({
  data: { cats: C.cats, cat: '全部', kw: '', list: C.templates, drafts: [] },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 2 });
    this.setData({ drafts: S.drafts() });
  },
  filter() {
    const { cat, kw } = this.data;
    const k = kw.trim();
    this.setData({ list: C.templates.filter(t => (cat === '全部' || t.cat === cat) && (!k || t.name.indexOf(k) >= 0 || t.desc.indexOf(k) >= 0)) });
  },
  pickCat(e) { this.setData({ cat: e.currentTarget.dataset.c }); this.filter(); },
  onKw(e) { this.setData({ kw: e.detail.value }); this.filter(); },
  clearKw() { this.setData({ kw: '' }); this.filter(); },
  open(e) { wx.navigateTo({ url: '/pages/editor/editor?tpl=' + e.currentTarget.dataset.id }); },
  openDraft(e) { wx.navigateTo({ url: '/pages/editor/editor?draft=' + e.currentTarget.dataset.id }); },
  draftMenu(e) {
    const id = e.currentTarget.dataset.id;
    wx.showActionSheet({
      itemList: ['继续编辑', '删除草稿'],
      itemColor: '#1f2633',
      success: r => {
        if (r.tapIndex === 0) return this.openDraft(e);
        wx.showModal({
          title: '删除草稿', content: '删除后无法恢复，确定删除吗？', confirmColor: '#d64545',
          success: m => { if (m.confirm) { S.removeDraft(id); this.setData({ drafts: S.drafts() }); } }
        });
      }
    });
  }
});
