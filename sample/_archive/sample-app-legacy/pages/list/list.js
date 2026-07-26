Page({
  data: {
    // 输入框数据
    inputList: [
      { id: 1, label: '用户名', placeholder: '请输入用户名', value: '', type: 'text', disabled: false },
      { id: 2, label: '密码', placeholder: '请输入密码', value: '', type: 'password', disabled: false },
      { id: 3, label: '手机号', placeholder: '请输入手机号', value: '', type: 'number', disabled: false },
      { id: 4, label: '禁用', placeholder: '禁用状态', value: '', type: 'text', disabled: true }
    ],
    
    // 复选框数据
    checkboxList: [
      { id: 1, label: '苹果 Apple', checked: true, disabled: false },
      { id: 2, label: '香蕉 Banana', checked: true, disabled: false },
      { id: 3, label: '橙子 Orange', checked: false, disabled: false },
      { id: 4, label: '葡萄 Grape (禁用)', checked: false, disabled: true }
    ],
    
    // 单选框数据
    radioList: [
      { id: 1, label: '男 Male', checked: true, disabled: false },
      { id: 2, label: '女 Female', checked: false, disabled: false },
      { id: 3, label: '保密 Secret (禁用)', checked: false, disabled: true }
    ],
    
    // 滑块数据
    sliderList: [
      { id: 1, label: '音量', value: 50, color: '#07C160', showValue: true, blockSize: 28, blockColor: '#FFFFFF' },
      { id: 2, label: '亮度', value: 70, color: '#FF9500', showValue: true, blockSize: 28, blockColor: '#FFFFFF' },
      { id: 3, label: '进度', value: 30, color: '#007AFF', showValue: true, blockSize: 28, blockColor: '#FFFFFF' },
      { id: 4, label: '自定义滑块', value: 60, color: '#FF6B6B', showValue: false, blockSize: 24, blockColor: '#FF6B6B' }
    ]
  },
  
  onLoad: function() {
    console.log('📝 表单页加载完成');
  },
  
  onShow: function() {
    console.log('📝 表单页显示');
  },
  
  // 复选框点击
  onCheckboxTap: function(e) {
    var id = e.currentTarget.dataset.id;
    console.log('☑️ 复选框点击:', id);
  },
  
  // 单选框点击
  onRadioTap: function(e) {
    var id = e.currentTarget.dataset.id;
    console.log('🔘 单选框点击:', id);
  }
});
