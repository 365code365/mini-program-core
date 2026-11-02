// 发起审批的动态表单定义：apply 页按 fields 用 <block wx:for> + wx:if/elif 渲染。
// type：text / number / digit / textarea / selector / date / month / radio / switch / approver / computed
const types = [
  { id: 'leave', name: '请假', color: '#1677ff', ic: '假', desc: '年假、事假、病假、调休' },
  { id: 'expense', name: '报销', color: '#fa8c16', ic: '报', desc: '差旅、交通、招待、办公' },
  { id: 'seal', name: '用印', color: '#722ed1', ic: '印', desc: '公章、合同章、财务章' }
];

const fields = {
  leave: [
    { key: 'kind', label: '请假类型', type: 'selector', options: ['年假', '事假', '病假', '调休', '婚假', '陪产假'], required: true },
    { key: 'start', label: '开始日期', type: 'date', required: true },
    { key: 'startHalf', label: '开始时段', type: 'radio', options: ['上午', '下午'], value: '上午' },
    { key: 'end', label: '结束日期', type: 'date', required: true },
    { key: 'endHalf', label: '结束时段', type: 'radio', options: ['上午', '下午'], value: '下午' },
    { key: 'days', label: '请假时长', type: 'computed', unit: '天' },
    { key: 'reason', label: '请假事由', type: 'textarea', placeholder: '如：回老家参加亲属婚礼', required: true },
    { key: 'approver', label: '审批人', type: 'approver', required: true }
  ],
  expense: [
    { key: 'kind', label: '报销类型', type: 'selector', options: ['差旅费', '市内交通', '业务招待', '办公用品', '培训费'], required: true },
    { key: 'month', label: '费用所属月份', type: 'month', required: true },
    { key: 'amount', label: '报销金额', type: 'digit', placeholder: '0.00', unit: '元', required: true },
    { key: 'invoices', label: '发票张数', type: 'number', placeholder: '张', unit: '张', required: true },
    { key: 'reason', label: '费用说明', type: 'textarea', placeholder: '如：10 月 21-23 日赴上海拜访客户高铁往返及住宿', required: true },
    { key: 'approver', label: '审批人', type: 'approver', required: true }
  ],
  seal: [
    { key: 'kind', label: '印章类型', type: 'radio', options: ['公章', '合同专用章', '财务专用章', '法人章'], value: '合同专用章', required: true },
    { key: 'file', label: '文件名称', type: 'text', placeholder: '如：杭州某某科技服务合同', required: true },
    { key: 'copies', label: '用印份数', type: 'number', placeholder: '份', unit: '份', required: true },
    { key: 'date', label: '用印日期', type: 'date', required: true },
    { key: 'takeOut', label: '是否外带', type: 'switch', value: false },
    { key: 'reason', label: '用途说明', type: 'textarea', placeholder: '如：与客户签订年度服务合同', required: true },
    { key: 'approver', label: '审批人', type: 'approver', required: true }
  ]
};

// 半天为单位的请假时长
function leaveDays(v) {
  if (!v.start || !v.end) return '';
  const s = new Date(v.start.replace(/-/g, '/')), e = new Date(v.end.replace(/-/g, '/'));
  const halves = Math.round((e - s) / 86400000) * 2 + (v.endHalf === '下午' ? 2 : 1) - (v.startHalf === '下午' ? 1 : 0);
  return halves > 0 ? String(halves / 2) : '';
}

function summary(type, v) {
  if (type === 'leave') return `${v.kind} ${v.days} 天（${v.start} ${v.startHalf} 至 ${v.end} ${v.endHalf}）`;
  if (type === 'expense') return `${v.kind} ¥${Number(v.amount).toFixed(2)}（${v.month}，${v.invoices} 张发票）`;
  return `${v.kind} · ${v.file} ${v.copies} 份${v.takeOut ? '（外带）' : ''}`;
}

module.exports = { types, fields, leaveDays, summary };
