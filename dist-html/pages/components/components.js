Page({
  data: {
    pickerRange: ['选项一', '选项二', '选项三', '选项四'],
    pickerValue: 0,
    richTextNodes: [
      { type: 'text', text: '这是一段' },
      { name: 'span', attrs: { style: 'color: #ff0000;' }, children: [{ type: 'text', text: '红色' }] },
      { type: 'text', text: '的富文本内容。' }
    ],
    checkboxValues: ['apple'],
    radioValue: 'male'
  },

  onLoad: function() {
    console.log('Components page loaded');
  },

  onPickerChange: function(e) {
    console.log('Picker changed:', e.detail.value);
    this.setData({
      pickerValue: e.detail.value
    });
  },

  onCheckboxChange: function(e) {
    console.log('Checkbox changed:', e.detail.value);
    this.setData({
      checkboxValues: e.detail.value
    });
  },

  onRadioChange: function(e) {
    console.log('Radio changed:', e.detail.value);
    this.setData({
      radioValue: e.detail.value
    });
  },

  onBack: function() {
    wx.navigateBack();
  }
});
