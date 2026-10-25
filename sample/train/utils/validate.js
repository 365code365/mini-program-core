// 乘车人表单校验：返回错误文案，合法时返回空串
const W = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2]
const CHECK = '10X98765432'
function name(s) {
  s = (s || '').trim()
  if (!s) return '请填写姓名'
  if (!/^[\u4e00-\u9fa5·]{2,15}$/.test(s)) return '姓名需为 2~15 个汉字'
  return ''
}
// 18 位：17 位数字 + 校验位（数字或 X），出生日期合法，校验位按 GB 11643 加权
function idcard(s) {
  s = (s || '').trim().toUpperCase()
  if (!s) return '请填写身份证号'
  if (s.length !== 18) return '身份证号应为 18 位'
  if (!/^\d{17}[\dX]$/.test(s)) return '身份证号格式不正确'
  const y = Number(s.slice(6, 10))
  const m = Number(s.slice(10, 12))
  const d = Number(s.slice(12, 14))
  const dt = new Date(y, m - 1, d)
  if (y < 1900 || dt.getFullYear() !== y || dt.getMonth() !== m - 1 || dt.getDate() !== d) return '身份证号里的出生日期不正确'
  let sum = 0
  for (let i = 0; i < 17; i++) sum += Number(s.charAt(i)) * W[i]
  if (CHECK.charAt(sum % 11) !== s.charAt(17)) return '身份证号校验位不正确'
  return ''
}
function phone(s) {
  s = (s || '').trim()
  if (!s) return '请填写手机号'
  if (!/^1[3-9]\d{9}$/.test(s)) return '手机号格式不正确'
  return ''
}
module.exports = { name, idcard, phone }
