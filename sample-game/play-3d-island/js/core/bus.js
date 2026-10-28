// 事件总线：经济、任务、界面之间只通过它说话，谁也不直接调谁的界面
function make() {
  var L = {};
  return {
    on: function (name, f) { (L[name] = L[name] || []).push(f); return function () { L[name] = (L[name] || []).filter(function (x) { return x !== f; }); }; },
    emit: function (name, data) {
      (L[name] || []).slice().forEach(function (f) { try { f(data); } catch (e) { console.error('[bus] ' + name, e); } });
      (L['*'] || []).slice().forEach(function (f) { try { f(name, data); } catch (e) { console.error('[bus] *', e); } });
    }
  };
}
module.exports = { make: make };
