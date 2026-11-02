// 申报向导每一步的校验：返回 { 字段: 错误文案 }，空对象表示通过。
const { checkIdcard } = require('./store');

function step0(f) {
  const e = {};
  if (!/^[\u4e00-\u9fa5·]{2,20}$/.test(f.name || '')) e.name = '请填写与身份证一致的中文姓名';
  const idErr = checkIdcard(f.idcard);
  if (idErr) e.idcard = idErr;
  if (!/^1[3-9]\d{9}$/.test(f.phone || '')) e.phone = '请填写 11 位手机号码';
  if (!f.region || f.region.length !== 3) e.region = '请选择户籍所在地';
  if (!f.address || f.address.length < 5) e.address = '请填写详细到门牌号的居住地址';
  return e;
}

function step1(f) {
  const e = {};
  if (f.reasonIdx < 0) e.reason = '请选择申请类型';
  if (!f.way) e.way = '请选择结果领取方式';
  if (f.way === '邮寄送达' && (!f.address || f.address.length < 5)) e.way = '邮寄送达需在第 1 步填写详细地址';
  if ((f.remark || '').length > 200) e.remark = '补充说明不超过 200 字';
  return e;
}

function step2(f, materials) {
  const e = {};
  const miss = materials.filter((m, i) => m.required && !(f.files && f.files[i]));
  if (miss.length) e.files = `还有 ${miss.length} 份必交材料未上传：${miss.map(m => m.name).join('、')}`;
  return e;
}

function step3(f) {
  return f.agree ? {} : { agree: '请阅读并勾选承诺书' };
}

function check(step, form, materials) {
  return [step0, step1, step2, step3][step](form, materials);
}

module.exports = { check };
