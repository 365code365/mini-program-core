// 合同编辑：editor + EditorContext。内容存 getContents 的 html，再进来 setContents 还原。
const C = require('../../utils/contracts.js');
const S = require('../../utils/store.js');
const H = require('../../utils/html.js');

Page({
  data: { name: '', words: 0, formats: {}, canUndo: false, preview: false, paras: [], saved: true },
  onLoad(q) {
    if (q.draft) {
      const d = S.getDraft(q.draft);
      if (d) { this.draft = d; this.tpl = C.templates.find(t => t.id === d.tplId); }
    }
    if (!this.tpl) this.tpl = C.templates.find(t => t.id === q.tpl) || C.templates[0];
    this.draftId = this.draft ? this.draft.id : 'D' + Date.now().toString().slice(-8);
    this.setData({ name: this.draft ? this.draft.name : this.tpl.name });
  },
  onEditorReady() {
    wx.createSelectorQuery().select('#editor').context(res => {
      this.ctx = res && res.context;
      if (!this.ctx) return wx.showToast({ title: '编辑器初始化失败', icon: 'none' });
      this.ctx.setContents({
        html: this.draft ? this.draft.html : this.tpl.html,
        success: () => this.ctx.getContents({ success: r => this.setData({ words: this.count(r.text), saved: true }) })
      });
    }).exec();
  },
  count(text) { return String(text || '').replace(/\s/g, '').length; },
  onInput(e) { this.setData({ words: this.count(e.detail.text), saved: false, canUndo: true }); },
  onStatus(e) { this.setData({ formats: e.detail || {} }); },
  onName(e) { this.setData({ name: e.detail.value, saved: false }); },
  bold() { this.ctx && this.ctx.format('bold'); },
  // undo / redo 不一定触发 bindinput，做完重新取一次内容刷新字数
  undo() { this.ctx && this.ctx.undo({ success: () => this.syncCount() }); },
  redo() { this.ctx && this.ctx.redo({ success: () => this.syncCount() }); },
  syncCount() { this.ctx.getContents({ success: r => this.setData({ words: this.count(r.text), saved: false }) }); },
  insertClause() {
    wx.showActionSheet({
      itemList: C.clauses.map(c => '插入「' + c.name + '」条款'),
      success: r => {
        const c = C.clauses[r.tapIndex];
        this.ctx.insertText({ text: '\n第__条 ' + c.text, success: () => this.syncCount() });
      }
    });
  },
  insertDate() {
    const d = new Date();
    this.ctx.insertText({ text: `\n签订日期：${d.getFullYear()} 年 ${d.getMonth() + 1} 月 ${d.getDate()} 日` });
  },
  paste() {
    wx.getClipboardData({
      success: res => {
        if (!res.data) return wx.showToast({ title: '剪贴板是空的', icon: 'none' });
        this.ctx.insertText({ text: res.data });
      }
    });
  },
  copyAll() {
    this.ctx.getContents({ success: r => wx.setClipboardData({ data: r.text.trim() }) });
  },
  resetTpl() {
    wx.showModal({
      title: '恢复模板', content: '当前修改会被模板原文覆盖（可撤销），继续吗？',
      success: m => { if (m.confirm) this.ctx.setContents({ html: this.tpl.html }); }
    });
  },
  togglePreview() {
    if (this.data.preview) return this.setData({ preview: false });
    this.ctx.getContents({ success: r => this.setData({ preview: true, paras: H.paras(r.html) }) });
  },
  save() {
    const name = this.data.name.trim();
    if (!name) return wx.showToast({ title: '请填写合同名称', icon: 'none' });
    this.ctx.getContents({
      success: r => {
        S.saveDraft({ id: this.draftId, name, tplId: this.tpl.id, html: r.html, words: this.count(r.text) });
        this.setData({ saved: true });
        wx.showToast({ title: '已保存到草稿', icon: 'success' });
      }
    });
  },
  review() {
    // 写完可以直接找律师审查：带上合同名，跳到文书审查服务
    this.save();
    wx.navigateTo({ url: '/pages/order/order?lawyer=l3&service=doc' });
  }
});
