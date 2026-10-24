// 每个处理函数只做一件事：把事件对象的关键字段打到 console（EV 行格式不要改，排查脚本按它 grep）。
// console.log 之后另有一次 setData，只更新页面上的「读数」，不影响事件派发与冒泡。
var KINDS = { touchstart: 'start', touchmove: 'move', touchend: 'end', touchcancel: 'cancel', tap: 'tap', longpress: 'longpress' };

function who(o) {
  return o && o.dataset ? o.dataset.who : '?';
}

function log(name) {
  return function (e) {
    var t = (e.touches && e.touches[0]) || (e.changedTouches && e.changedTouches[0]) || {};
    console.log('EV ' + e.type + ' -> ' + name
      + ' target=' + (e.target && e.target.dataset ? e.target.dataset.who : '?')
      + ' current=' + (e.currentTarget && e.currentTarget.dataset ? e.currentTarget.dataset.who : '?')
      + ' touches=' + (e.touches ? e.touches.length : 'x')
      + ' changed=' + (e.changedTouches ? e.changedTouches.length : 'x')
      + ' pt=' + t.clientX + ',' + t.clientY
      + ' ts=' + (typeof e.timeStamp));
    // 仅用于显示的读数
    if (!this || typeof this.setData !== 'function') return;
    var x = typeof t.clientX === 'number' ? Math.round(t.clientX) : '-';
    var y = typeof t.clientY === 'number' ? Math.round(t.clientY) : '-';
    var ev = {
      type: e.type,
      handler: name,
      target: String(who(e.target)),
      current: String(who(e.currentTarget)),
      pt: x + ', ' + y
    };
    var patch = { last: ev, total: (this.data.total || 0) + 1 };
    var kind = KINDS[e.type];
    if (kind) patch['counts.' + kind] = (this.data.counts[kind] || 0) + 1;
    if (name === 'lockMove') {
      patch.maskLast = ev;
      patch.maskMoves = (this.data.maskMoves || 0) + 1;
    } else if (ev.target === 'inner') {
      // 外层处理函数也会收到遮罩/滚动区的事件，内层卡片只显示落在内层上的
      patch.innerLast = ev;
    }
    this.setData(patch);
  };
}

var EMPTY = { type: '—', handler: '—', target: '—', current: '—', pt: '—' };

Page({
  data: {
    last: EMPTY,
    innerLast: EMPTY,
    maskLast: EMPTY,
    maskMoves: 0,
    total: 0,
    counts: { start: 0, move: 0, end: 0, cancel: 0, tap: 0, longpress: 0 }
  },
  capOuterTS: log('capOuterTS'),
  tsOuter: log('tsOuter'),
  tmOuter: log('tmOuter'),
  teOuter: log('teOuter'),
  tapOuter: log('tapOuter'),
  lpOuter: log('lpOuter'),
  tsInner: log('tsInner'),
  tmInner: log('tmInner'),
  teInner: log('teInner'),
  tapInner: log('tapInner'),
  lpInner: log('lpInner'),
  lockMove: log('lockMove'),
  tcOuter: log('tcOuter'),
  tcInner: log('tcInner'),
});
