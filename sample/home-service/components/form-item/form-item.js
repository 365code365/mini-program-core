// 表单项：左侧标签 + 默认 slot 放控件 + 错误提示。校验逻辑来自 behaviors/validate.js。
const validate = require('../../behaviors/validate.js');

Component({
  behaviors: [validate],
  relations: {
    '../form-group/form-group': {
      type: 'ancestor',
      linked(group) { this.group = group; }
    }
  },
  properties: {
    label: { type: String, value: '' },
    required: { type: Boolean, value: false },
    column: { type: Boolean, value: false }
  }
});
