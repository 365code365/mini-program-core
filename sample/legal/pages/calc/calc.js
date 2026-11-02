const K = require('../../utils/calc.js');
const H = require('../../utils/html.js');

const LEVELS = K.INJURY_MONTHS.map((m, i) => ({ name: `${['一', '二', '三', '四', '五', '六', '七', '八', '九', '十'][i]}级伤残（${m} 个月）`, v: i + 1 }));
const KINDS = [{ id: 'property', name: '财产案件（合同、借贷、侵权赔偿）' }, { id: 'labor', name: '劳动争议案件' }, { id: 'divorce', name: '离婚案件' }];
const LAWS = {
  injury: '<p><b>《工伤保险条例》第三十五~三十七条</b>：职工因工致残被鉴定为一级至十级伤残的，从工伤保险基金按伤残等级支付一次性伤残补助金，标准为本人工资的 27 至 7 个月。</p><p>五至十级伤残与单位解除劳动关系的，另有一次性工伤医疗补助金和伤残就业补助金，具体标准由<b>各省</b>规定。</p>',
  severance: '<p><b>《劳动合同法》第四十七条</b>：经济补偿按劳动者在本单位工作的年限，每满一年支付一个月工资；六个月以上不满一年的按一年计算；不满六个月的支付半个月工资。</p><p>月工资高于当地上年度职工月平均工资三倍的，按三倍支付，年限最高不超过十二年。<b>第八十七条</b>：违法解除的，按经济补偿标准的二倍支付赔偿金。</p>',
  court: '<p><b>《诉讼费用交纳办法》第十三条</b>：财产案件按标的额分段累计交纳，不超过 1 万元的每件 50 元；劳动争议案件每件 10 元；离婚案件每件 50~300 元，财产总额超过 20 万元的部分按 0.5% 交纳。</p><p>适用简易程序审理的案件减半交纳。</p>'
};

function today() { const d = new Date(); return `${d.getFullYear()}-${('0' + (d.getMonth() + 1)).slice(-2)}-${('0' + d.getDate()).slice(-2)}`; }

Page({
  data: {
    tabs: ['工伤赔偿', 'N+1 补偿', '诉讼费'], tab: 0,
    levels: LEVELS, levelIdx: 9, wage: '', offMonths: '3',
    start: '2021-03-01', end: today(), avg: '9850', sWage: '', notified: 'no', type: 'legal',
    kinds: KINDS, kindIdx: 0, amount: '', simple: 'normal',
    res: null, err: '', law: H.paras(LAWS.injury)
  },
  onLoad(q) {
    const t = Number(q.tab || 0);
    if (t) this.switchTo(t);
  },
  switchTo(t) { this.setData({ tab: t, res: null, err: '', law: H.paras([LAWS.injury, LAWS.severance, LAWS.court][t]) }); },
  onTab(e) { this.switchTo(e.currentTarget.dataset.i); },
  onField(e) { this.setData({ [e.currentTarget.dataset.k]: e.detail.value, err: '' }); },
  onLevel(e) { this.setData({ levelIdx: Number(e.detail.value) }); },
  onKind(e) { this.setData({ kindIdx: Number(e.detail.value) }); },
  calc() {
    const d = this.data;
    let res = null;
    if (d.tab === 0) {
      const wage = Number(d.wage), off = Number(d.offMonths || 0);
      if (!(wage > 0)) return this.setData({ err: '请填写受伤前 12 个月的平均月工资' });
      if (off < 0 || off > 24) return this.setData({ err: '停工留薪期一般不超过 12 个月，最长 24 个月' });
      const r = K.injury(wage, d.levels[d.levelIdx].v, off);
      res = {
        total: r.total, rows: [
          { k: `一次性伤残补助金（${r.months} 个月）`, v: r.subsidy },
          { k: `停工留薪期工资（${off} 个月）`, v: r.off }
        ].concat(r.pension ? [{ k: '伤残津贴（按月领取）', v: r.pension }] : []),
        note: '五至十级解除劳动关系的一次性工伤医疗补助金、就业补助金按当地标准另计。'
      };
    } else if (d.tab === 1) {
      const wage = Number(d.sWage), avg = Number(d.avg);
      if (!(wage > 0)) return this.setData({ err: '请填写离职前 12 个月的平均月工资' });
      if (!(avg > 0)) return this.setData({ err: '请填写当地上年度职工月平均工资' });
      if (d.end <= d.start) return this.setData({ err: '离职日期要晚于入职日期' });
      const r = K.severance({ start: d.start, end: d.end, wage, avg, notified: d.notified === 'yes', type: d.type });
      res = {
        total: r.total, rows: [
          { k: `工作年限 ${r.y.years} 年 ${r.y.months} 个月 → N = ${r.n}`, v: '' },
          { k: `计算基数${r.capped ? '（超社平 3 倍，按封顶）' : ''}`, v: r.base },
          { k: r.illegal ? '违法解除赔偿金（2N）' : '经济补偿金（N）', v: r.illegal ? r.nPay * 2 : r.nPay }
        ].concat(r.notice ? [{ k: '代通知金（+1）', v: r.notice }] : []),
        note: r.illegal ? '违法解除的赔偿金不再另付经济补偿与代通知金。' : '协商一致解除、到期不续签等情形不适用 +1。'
      };
    } else {
      const amount = Number(d.amount || 0);
      const kind = d.kinds[d.kindIdx].id;
      if (kind === 'property' && !(amount > 0)) return this.setData({ err: '请填写诉讼请求的金额（标的额）' });
      const r = K.courtFee(kind, amount, d.simple === 'simple');
      res = {
        total: r.final, rows: [
          { k: '案件受理费', v: r.fee },
          { k: d.simple === 'simple' ? '简易程序减半' : '普通程序', v: d.simple === 'simple' ? -(r.fee - r.final) : '' }
        ],
        note: '受理费由原告预交，最终由败诉方负担；保全费、鉴定费另计。'
      };
    }
    res.rows = res.rows.map(x => ({ k: x.k, v: x.v === '' ? '' : (typeof x.v === 'number' ? (x.v < 0 ? '-¥' : '¥') + Math.abs(x.v).toFixed(2) : x.v) }));
    res.totalText = res.total.toFixed(2);
    this.setData({ res, err: '' });
    wx.pageScrollTo && wx.pageScrollTo({ scrollTop: 600, duration: 200 });
  },
  copy() {
    const r = this.data.res;
    const text = `【律安心·${this.data.tabs[this.data.tab]}测算】合计 ¥${r.totalText}\n` + r.rows.map(x => x.k + (x.v ? '：' + x.v : '')).join('\n') + '\n（仅供参考，不构成法律意见）';
    wx.setClipboardData({ data: text });
  },
  ask() {
    const cat = ['labor', 'labor', 'contract'][this.data.tab];
    getApp().globalData.lawyerCat = cat;
    wx.switchTab({ url: '/pages/lawyers/lawyers' });
  }
});
