// ==================== 高性能模式：触摸、鼠标、滚轮、键盘（DOM 事件 → 与 QuickJS 模式同形的 wx 事件） ====================
//
// 形状与 game/touch.rs 一致：
// - `{touches, changedTouches, timeStamp}`，Touch = {identifier, screenX/Y, clientX/Y, pageX/Y, force}（三组坐标都是视图内逻辑坐标）；
// - identifier：一次按下到抬起不变，分配当前最小的空闲值（从 0 起），不透传 DOM 的 identifier；最多 10 根；
// - timeStamp：与 performance.now() 同一时间轴（毫秒）。
// 派发走 game-input.js 的 __gameInput（监听表、回调抛错不打断其余），键盘走 __hostDispatch（keyDown / keyUp）。
// 系统覆盖层（Toast 带 mask、Modal、ActionSheet，wg-overlay.js）弹着时触摸不进游戏：覆盖层在画布之上，DOM 自己拦住了。
(function() {
    var G = globalThis, MAX = 10;
    var slots = [];   // [{dom, ident, x, y, force}]，按按下先后
    function free() { var id = 0; while (slots.some(function(s) { return s.ident === id; })) id++; return id; }
    function touchObj(s) { return { identifier: s.ident, screenX: s.x, screenY: s.y, clientX: s.x, clientY: s.y, pageX: s.x, pageY: s.y, force: s.force }; }
    function dispatch(kind, changed, ts) {
        var ev = { touches: slots.map(touchObj), changedTouches: changed.map(touchObj), timeStamp: ts };
        G.__gameInput(kind, JSON.stringify(ev));
    }
    function find(dom) { for (var i = 0; i < slots.length; i++) if (slots[i].dom === dom) return i; return -1; }
    function upd(s, t) { s.x = t.clientX; s.y = t.clientY; s.force = typeof t.force === 'number' ? Math.max(0, Math.min(1, t.force)) : 0; }
    // 只收落在游戏画面上的触摸（覆盖层上的不收：它们在 DOM 里先被覆盖层的元素接走）
    function onGame(t) { var el = t.target; return !(el && el.closest && el.closest('.__wg_overlay')); }
    function handler(kind) {
        return function(e) {
            var ts = performance.now(), changed = [], list = e.changedTouches;
            for (var i = 0; i < list.length; i++) {
                var t = list[i], at = find(t.identifier);
                if (kind === 'touchstart') {
                    if (at >= 0 || slots.length >= MAX || !onGame(t)) continue;
                    var s = { dom: t.identifier, ident: free(), x: 0, y: 0, force: 0 };
                    upd(s, t); slots.push(s); changed.push(s);
                } else if (at >= 0) {
                    var s2 = slots[at];
                    upd(s2, t);
                    if (kind === 'touchmove') changed.push(s2);
                    else { slots.splice(at, 1); changed.push(s2); }
                }
            }
            if (changed.length) dispatch(kind, changed, ts);
        };
    }
    // 捕获阶段派发 wx.onTouch*（游戏先拿到），冒泡阶段最后才 preventDefault（拦掉系统的滚动 / 缩放 / 模拟鼠标）：
    // 捕获阶段就 preventDefault 的话，直接监听画布的引擎（Phaser 的 TouchManager）看到 defaultPrevented 就不处理了
    ['touchstart', 'touchmove', 'touchend', 'touchcancel'].forEach(function(k) {
        document.addEventListener(k, handler(k), { passive: true, capture: true });
        G.addEventListener(k, function(e) { if (e.cancelable) e.preventDefault(); }, { passive: false });
    });
    // 进后台：按着的手指全部 touchcancel（changedTouches = 全部、touches = 空），与 QuickJS 模式一致
    G.__wgCancelTouches = function() {
        if (!slots.length) return;
        var all = slots; slots = [];
        dispatch('touchcancel', all, performance.now());
    };

    // 鼠标 / 滚轮（PC 端 wx.onMouseDown 等；手机上没有）。触摸模拟出来的鼠标事件不发：iOS 点一下会补一串 mouse*
    var last = null, touchAt = 0;
    document.addEventListener('touchend', function() { touchAt = performance.now(); }, true);
    function mouse(kind) {
        return function(e) {
            if (performance.now() - touchAt < 800) return;
            var x = e.clientX, y = e.clientY, ts = performance.now(), ev;
            if (kind === 'mousemove') { ev = { x: x, y: y, movementX: last ? x - last[0] : 0, movementY: last ? y - last[1] : 0, timeStamp: ts }; }
            else ev = { button: Math.max(0, Math.min(4, e.button)), x: x, y: y, timeStamp: ts };
            last = [x, y];
            G.__gameInput(kind, JSON.stringify(ev));
        };
    }
    ['mousedown', 'mouseup', 'mousemove'].forEach(function(k) { document.addEventListener(k, mouse(k), true); });
    document.addEventListener('wheel', function(e) {
        if (e.cancelable) e.preventDefault();
        G.__gameInput('wheel', JSON.stringify({ deltaX: e.deltaX, deltaY: e.deltaY, deltaZ: e.deltaZ, x: e.clientX, y: e.clientY, timeStamp: performance.now() }));
    }, { passive: false, capture: true });
    function key(kind) {
        return function(e) {
            // 输入框（wx.showKeyboard 的原生输入框不在页面里；页面里只有覆盖层，没有可编辑元素）不拦
            if (e.target && /^(INPUT|TEXTAREA)$/.test(e.target.tagName)) return;
            G.__hostDispatch(kind, JSON.stringify({ key: e.key, code: e.code, timeStamp: performance.now() }));
        };
    }
    document.addEventListener('keydown', key('keyDown'), true);
    document.addEventListener('keyup', key('keyUp'), true);
    // 长按菜单、双指缩放、选中文字：游戏页面里都不要
    document.addEventListener('contextmenu', function(e) { e.preventDefault(); }, true);
    document.addEventListener('gesturestart', function(e) { e.preventDefault(); }, true);
})();
