// <editor>（官方 component/editor）：本引擎按**纯文本**富文本编辑器实现 —— 一个多行输入框，内容按段落存。
// EditorContext（wx.createSelectorQuery().select('#ed').context(…)）的方法都在：setContents / getContents 收发
// `{html, text, delta}`（html 是 <p> 段落、delta 是 Quill 的 ops），insertText / clear / undo / redo / blur 真生效；
// format / removeFormat / insertDivider / insertImage / insertCustomBlock 只改文本（分割线、图片按占位文字插入）。
// 取舍：没有行内样式（加粗、颜色、列表……），format 之后 statuschange 照常回当前格式状态，画面不变（写明）。
function __edEscape(s) { return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;'); }
function __edFromHtml(html) {
  return String(html || '')
    .replace(/<br\s*\/?>/gi, '\n').replace(/<\/(p|div|li|h[1-6])>/gi, '\n')
    .replace(/<img[^>]*>/gi, '[图片]').replace(/<hr[^>]*>/gi, '———\n').replace(/<[^>]+>/g, '')
    .replace(/&nbsp;/g, ' ').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&amp;/g, '&')
    .replace(/\n+$/, '');
}
Component({
  properties: {
    readOnly: { type: Boolean, value: false },
    placeholder: { type: String, value: '' },
    showImgSize: { type: Boolean, value: false },
    showImgToolbar: { type: Boolean, value: false },
    showImgResize: { type: Boolean, value: false }
  },
  data: { text: '' },
  lifetimes: {
    attached: function () {
      this._undo = [];
      this._redo = [];
      this._formats = {};
      var self = this;
      setTimeout(function () { self.triggerEvent('ready', {}); }, 0);
    }
  },
  methods: {
    _detail: function () {
      var t = this.data.text;
      var paras = t.split('\n');
      return {
        html: paras.map(function (p) { return '<p>' + (p ? __edEscape(p) : '<br>') + '</p>'; }).join(''),
        text: t + '\n',
        delta: { ops: [{ insert: t + '\n' }] }
      };
    },
    _set: function (t, record) {
      if (record !== false && t !== this.data.text) { this._undo.push(this.data.text); this._redo = []; }
      this.setData({ text: t });
    },
    _in: function (e) {
      var v = e.detail.value;
      if (v !== this.data.text) { this._undo.push(this.data.text); this._redo = []; }
      this.data.text = v;
      this.triggerEvent('input', this._detail());
    },
    _focus: function () { this.triggerEvent('focus', this._detail()); },
    _blur: function () { this.triggerEvent('blur', this._detail()); },
    __editorContext: function () {
      var self = this;
      var done = function (o, name, res) {
        o = o || {};
        res = res || {};
        res.errMsg = name + ':ok';
        setTimeout(function () { o.success && o.success(res); o.complete && o.complete(res); }, 0);
      };
      var insert = function (s) { self._set(self.data.text + s); self.triggerEvent('input', self._detail()); };
      return {
        setContents: function (o) { o = o || {}; var t = o.delta && o.delta.ops ? o.delta.ops.map(function (op) { return typeof op.insert === 'string' ? op.insert : '[图片]'; }).join('').replace(/\n$/, '') : __edFromHtml(o.html); self._set(t); done(o, 'setContents'); },
        getContents: function (o) { done(o, 'getContents', self._detail()); },
        getSelectionText: function (o) { done(o, 'getSelectionText', { text: '' }); },
        clear: function (o) { self._set(''); self.triggerEvent('input', self._detail()); done(o, 'clear'); },
        insertText: function (o) { o = o || {}; insert(String(o.text || '')); done(o, 'insertText'); },
        insertDivider: function (o) { insert('\n———\n'); done(o, 'insertDivider'); },
        insertImage: function (o) { o = o || {}; insert('[' + (o.alt || '图片') + ']'); done(o, 'insertImage'); },
        insertCustomBlock: function (o) { done(o, 'insertCustomBlock'); },
        deleteText: function (o) {
          o = o || {};
          var t = self.data.text, i = Math.max(0, o.index || 0), n = Math.max(0, o.length || 0);
          self._set(t.slice(0, i) + t.slice(i + n));
          done(o, 'deleteText');
        },
        undo: function (o) { if (self._undo.length) { self._redo.push(self.data.text); self._set(self._undo.pop(), false); } done(o, 'undo'); },
        redo: function (o) { if (self._redo.length) { self._undo.push(self.data.text); self._set(self._redo.pop(), false); } done(o, 'redo'); },
        format: function (name, value) { self._formats[name] = value === undefined ? true : value; self.triggerEvent('statuschange', JSON.parse(JSON.stringify(self._formats))); },
        removeFormat: function (o) { self._formats = {}; self.triggerEvent('statuschange', {}); done(o, 'removeFormat'); },
        getHistoryState: function (o) { done(o, 'getHistoryState', { undo: self._undo.length > 0, redo: self._redo.length > 0 }); },
        getSelection: function (o) { var n = self.data.text.length; done(o, 'getSelection', { start: n, end: n }); },
        setSelection: function (o) { done(o, 'setSelection'); },
        getBounds: function (o) { done(o, 'getBounds', { bounds: { left: 0, top: 0, width: 0, height: 0 } }); },
        blur: function (o) { wx.hideKeyboard(); done(o, 'blur'); },
        scrollIntoView: function (o) { done(o, 'scrollIntoView'); }
      };
    }
  }
});
