Page({
  data: {
    buttonTip: '按下有 button-hover 态；disabled 的点不动',
    inputLog: '在上面任一输入框里打字（e.detail.value 一律是字符串）',
    taLen: 0,
    fruits: [
      { name: '苹果', value: 'apple', checked: true },
      { name: '香蕉', value: 'banana', checked: false },
      { name: '橙子', value: 'orange', checked: false }
    ],
    sizes: ['S', 'M', 'L', 'XL'],
    volume: 60,
    choiceLog: '拨开关、勾选、拖滑块看 change 事件',
    formResult: '提交后在这里显示 e.detail.value（各字段带类型）'
  },
  taps: 0,
  onButton() {
    this.taps++;
    this.setData({ buttonTip: 'bindtap 第 ' + this.taps + ' 次' });
  },
  onInput(e) {
    const f = e.currentTarget.dataset.field;
    this.setData({ inputLog: 'bindinput [' + f + '] → "' + e.detail.value + '"（' + typeof e.detail.value + '）' });
  },
  onFocus(e) { this.setData({ inputLog: 'bindfocus：聚焦，宿主弹出软键盘' }); },
  onBlur(e) { this.setData({ inputLog: 'bindblur："' + e.detail.value + '"' }); },
  onConfirm(e) { this.setData({ inputLog: 'bindconfirm（点了键盘上的「搜索」）："' + e.detail.value + '"' }); },
  onTextarea(e) { this.setData({ taLen: e.detail.value.length }); },
  onSwitch(e) { this.setData({ choiceLog: 'switch change → ' + e.detail.value + '（' + typeof e.detail.value + '）' }); },
  onCheck(e) { this.setData({ choiceLog: 'checkbox-group change → ' + JSON.stringify(e.detail.value) }); },
  onRadio(e) { this.setData({ choiceLog: 'radio-group change → "' + e.detail.value + '"' }); },
  onSlider(e) { this.setData({ volume: e.detail.value, choiceLog: 'slider change → ' + e.detail.value + '（' + typeof e.detail.value + '）' }); },
  onSubmit(e) {
    this.setData({ formResult: 'submit：' + JSON.stringify(e.detail.value) });
  },
  onReset() {
    this.setData({ formResult: 'reset：输入框清空、开关与勾选取消、滑块回到 min' });
  }
});
