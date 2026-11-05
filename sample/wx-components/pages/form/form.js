const app = getApp();
Page({
  data: {
    focus: false, comment: '', loading: false, percent: 20, size: 'S', color: 'red',
    fruits: Array.from({ length: 30 }, (_, i) => ({ v: `f${i}`, name: `水果 ${i + 1}`, checked: i === 0 })),
    log: []
  },
  toggleFocus() { this.setData({ focus: !this.data.focus }); },
  onFocus(e) { app.note(this, `focus height=${e.detail.height} (${typeof e.detail.height})`); },
  onBlur(e) { app.note(this, `blur value=${e.detail.value}`); },
  onChange(e) { app.note(this, `input bindchange（失焦且内容变了）value=${e.detail.value}`); },
  onInput(e) { app.note(this, `input cursor=${e.detail.cursor} keyCode=${e.detail.keyCode}`); },
  onConfirm(e) { app.note(this, `confirm ${e.detail.value}`); },
  onLine(e) { app.note(this, `linechange lineCount=${e.detail.lineCount} height=${e.detail.height}`); },
  onSize(e) { app.note(this, `规格 change ${e.detail.value}`); },
  onColor(e) { app.note(this, `颜色 change ${e.detail.value}（与规格组互不影响）`); },
  onFruits(e) { app.note(this, `checkbox-group 选中 ${e.detail.value.length} 项（滚出视口的也算）`); },
  submit() {
    this.setData({ loading: true });
    setTimeout(() => { this.setData({ loading: false }); app.note(this, 'loading 结束'); }, 1500);
  },
  onDisabledTap() { app.note(this, '不该出现：disabled 按钮不派 tap'); },
  grow() { this.setData({ percent: this.data.percent >= 100 ? 10 : this.data.percent + 30 }); },
  onActiveEnd(e) { app.note(this, `progress activeend curPercent=${e.detail.curPercent}`); },
  onSubmit(e) { app.note(this, `form submit ${JSON.stringify(e.detail.value)}`); }
});
