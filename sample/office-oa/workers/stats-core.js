// 月度考勤统计（纯函数）：worker 与主线程降级共用同一份。
// 规则：09:00 上班、18:00 下班；晚于 09:00 记迟到，早于 18:00 记早退，工作日无上班卡记缺卡；>18:30 的部分算加班。
function toMin(t) { if (!t) return -1; const p = t.split(':'); return Number(p[0]) * 60 + Number(p[1]); }
function pad(n) { return n < 10 ? '0' + n : '' + n; }

function monthStats(month, records, today) {
  const y = Number(month.slice(0, 4)), m = Number(month.slice(5, 7));
  const days = new Date(y, m, 0).getDate();
  const out = { month, workdays: 0, present: 0, late: 0, early: 0, missing: 0, lateMin: 0, hours: 0, overtime: 0, days: [] };
  const first = new Date(y, m - 1, 1).getDay();
  for (let d = 1; d <= days; d++) {
    const key = `${y}-${pad(m)}-${pad(d)}`;
    const w = (first + d - 1) % 7;
    const weekend = w === 0 || w === 6;
    const future = key > today;
    const r = records[key];
    let st = weekend ? 'rest' : (future ? 'future' : 'ok');
    // 今天还没打上班卡不算缺卡（一天没过完）
    if (!weekend && !future && key === today && (!r || !r.in)) st = 'today';
    else if (!weekend && !future) {
      out.workdays++;
      if (!r || !r.in) { out.missing++; st = 'miss'; }
      else {
        out.present++;
        const a = toMin(r.in), b = toMin(r.out);
        if (a > 540) { out.late++; out.lateMin += a - 540; st = 'late'; }
        if (b >= 0 && b < 1080) { out.early++; st = 'early'; }
        if (b < 0 && key !== today) { out.missing++; st = 'miss'; }
        if (b > a && a >= 0) { out.hours += (b - a - 60) / 60; if (b > 1110) out.overtime += (b - 1110) / 60; }
      }
    }
    out.days.push({ d, key, st, w });
  }
  out.avgHours = out.present ? Math.round(out.hours / out.present * 10) / 10 : 0;
  out.hours = Math.round(out.hours * 10) / 10;
  out.overtime = Math.round(out.overtime * 10) / 10;
  out.lead = first; // 月初前空几格（日历对齐）
  return out;
}

module.exports = { monthStats };
