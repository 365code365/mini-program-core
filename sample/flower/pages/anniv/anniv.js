const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const QUICK = ['妈妈生日', '爸爸生日', '恋爱纪念日', '结婚纪念日', '好友生日'];
Page({
  data: { id: '', name: '', nameSeq: 0, md: '', dateValue: '', dateText: '', left: 0, kinds: data.uses, kind: 'birthday', quick: QUICK },
  onLoad(q) {
    const d = q.id ? store.getDates().find(x => x.id === String(q.id)) : null;
    if (d) {
      this.setData({ id: d.id, name: d.name, kind: d.kind });
      wx.setNavigationBarTitle({ title: '编辑纪念日' });
      this.sync(d.md);
    } else {
      this.sync(store.dayAfter(7).slice(5));
    }
  },
  sync(md) {
    const p = md.split('-');
    this.setData({ md, dateValue: new Date().getFullYear() + '-' + md, dateText: Number(p[0]) + ' 月 ' + Number(p[1]) + ' 日', left: store.daysLeft(md) });
  },
  onName(e) { this.setData({ name: e.detail.value }); },
  // 快捷名称：顺便猜一个用途（生日推荐生日花束，纪念日推荐爱情花束）
  onQuick(e) {
    const t = e.currentTarget.dataset.t;
    this.setData({ name: t, nameSeq: this.data.nameSeq + 1, kind: t.indexOf('生日') >= 0 ? 'birthday' : 'love' });
  },
  onDate(e) { this.sync(e.detail.value.slice(5)); },
  onKind(e) { this.setData({ kind: e.currentTarget.dataset.k }); },
  onSave() {
    const name = this.data.name.trim();
    if (!name) { wx.showToast({ title: '给纪念日起个名字吧', icon: 'none' }); return; }
    if (name.length > 10) { wx.showToast({ title: '名称最多 10 个字', icon: 'none' }); return; }
    const dup = store.getDates().some(x => x.id !== this.data.id && x.name === name && x.md === this.data.md);
    if (dup) { wx.showToast({ title: '这个纪念日已经添加过了', icon: 'none' }); return; }
    store.upsertDate({ id: this.data.id, name, md: this.data.md, kind: this.data.kind });
    wx.showToast({ title: '已保存', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 400);
  },
  onDelete() {
    wx.showModal({
      title: '删除纪念日', content: '删除「' + this.data.name + '」后不会再提醒你订花', confirmText: '删除', confirmColor: '#ef4f76',
      success: r => {
        if (!r.confirm) return;
        store.removeDate(this.data.id);
        wx.showToast({ title: '已删除', icon: 'none' });
        setTimeout(() => wx.navigateBack(), 300);
      }
    });
  }
});
