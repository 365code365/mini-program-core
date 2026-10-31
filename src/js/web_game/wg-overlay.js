// ==================== 高性能模式：系统覆盖层（Toast / Loading / Modal / ActionSheet） ====================
//
// QuickJS 模式由引擎画在帧里（game/overlay.rs）；这里画成页面上盖在画布之上的 DOM。逻辑（参数校验、单槽位、
// 计时、回调形状）全在基础库 ui.js / system.js 里，这里只实现它们调的原生入口：
// __native_show_toast / hide_toast / show_loading / hide_loading / show_modal → __handleModalResult(confirm)，
// __native_host_request('actionSheet') → __resolveActionSheet(id, tapIndex, err)。
// 覆盖层的元素带 .__wg_overlay：落在上面的触摸不进游戏（wg-input.js），带 mask 的 Toast / Modal / ActionSheet 拦全屏。
(function() {
    var G = globalThis;
    var mk = __wgNative.createElement;
    var css = mk('style');
    css.textContent = [
        '.__wg_overlay{position:fixed;z-index:1000;font-family:-apple-system,"PingFang SC","Helvetica Neue",sans-serif;-webkit-user-select:none;user-select:none}',
        '.__wg_mask{left:0;top:0;right:0;bottom:0}',
        '.__wg_dim{background:rgba(0,0,0,.6)}',
        '.__wg_tip{left:50%;top:50%;transform:translate(-50%,-50%);background:rgba(17,17,17,.7);color:#fff;border-radius:12px;text-align:center;box-sizing:border-box}',
        '.__wg_tip.icon{width:136px;min-height:136px;padding:28px 12px 18px}',
        '.__wg_tip.text{max-width:70vw;padding:12px 20px;font-size:15px;line-height:1.4}',
        '.__wg_tip .t{font-size:15px;line-height:1.4;margin-top:14px;word-break:break-all}',
        '.__wg_icon{width:44px;height:44px;margin:0 auto;display:block}',
        '.__wg_spin{animation:__wg_rot 1s steps(12) infinite}',
        '@keyframes __wg_rot{to{transform:rotate(360deg)}}',
        '.__wg_modal{left:50%;top:50%;transform:translate(-50%,-50%);width:min(320px,80vw);background:#fff;border-radius:12px;overflow:hidden;text-align:center}',
        '.__wg_modal .h{padding:32px 24px 0;font-size:17px;font-weight:600;color:#000}',
        '.__wg_modal .c{padding:16px 24px 32px;font-size:15px;line-height:1.5;color:rgba(0,0,0,.5);word-break:break-all;white-space:pre-wrap}',
        '.__wg_modal .h+.c{padding-top:12px}',
        '.__wg_modal .b{display:flex;border-top:1px solid rgba(0,0,0,.1)}',
        '.__wg_modal .b div{flex:1;height:56px;line-height:56px;font-size:17px;font-weight:600}',
        '.__wg_modal .b div+div{border-left:1px solid rgba(0,0,0,.1)}',
        '.__wg_sheet{left:0;right:0;bottom:0;background:#f7f7f7;border-radius:12px 12px 0 0;overflow:hidden;padding-bottom:env(safe-area-inset-bottom)}',
        '.__wg_sheet div{background:#fff;height:56px;line-height:56px;text-align:center;font-size:17px;border-bottom:1px solid rgba(0,0,0,.08)}',
        '.__wg_sheet .a{height:auto;line-height:1.4;padding:16px;font-size:14px;color:rgba(0,0,0,.5)}',
        '.__wg_sheet .x{margin-top:8px;border:none}',
        '.__wg_press{background:#ececec !important}'
    ].join('\n');
    document.head.appendChild(css);

    function el(cls, html) { var d = mk('div'); d.className = '__wg_overlay ' + cls; if (html) d.innerHTML = html; return d; }
    function esc(s) { return String(s).replace(/[&<>"]/g, function(c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]; }); }
    // 按下态 + 点击（触摸抬起在同一个元素上才算点到）
    function tap(node, fn) {
        node.addEventListener('touchstart', function(e) { e.preventDefault(); e.stopPropagation(); node.classList.add('__wg_press'); }, { passive: false });
        node.addEventListener('touchend', function(e) {
            e.preventDefault(); e.stopPropagation(); node.classList.remove('__wg_press');
            var t = e.changedTouches[0], hit = t && document.elementFromPoint(t.clientX, t.clientY);
            if (hit && (hit === node || node.contains(hit))) fn();
        }, { passive: false });
        node.addEventListener('touchcancel', function() { node.classList.remove('__wg_press'); });
        node.addEventListener('click', function(e) { if (!('ontouchend' in G)) { e.stopPropagation(); fn(); } });
    }
    function swallow(node) {
        ['touchstart', 'touchmove', 'touchend', 'touchcancel'].forEach(function(k) {
            node.addEventListener(k, function(e) { if (e.cancelable) e.preventDefault(); e.stopPropagation(); }, { passive: false });
        });
    }

    var ICONS = {
        success: '<svg class="__wg_icon" viewBox="0 0 44 44"><path d="M10 23l8 8 16-17" fill="none" stroke="#fff" stroke-width="3.5" stroke-linecap="round" stroke-linejoin="round"/></svg>',
        error: '<svg class="__wg_icon" viewBox="0 0 44 44"><circle cx="22" cy="22" r="18" fill="none" stroke="#fff" stroke-width="3"/><path d="M22 12v13" stroke="#fff" stroke-width="3.5" stroke-linecap="round"/><circle cx="22" cy="31" r="2.2" fill="#fff"/></svg>',
        loading: (function() {
            var s = '<svg class="__wg_icon __wg_spin" viewBox="0 0 44 44">';
            for (var i = 0; i < 12; i++) s += '<rect x="20.5" y="4" width="3" height="10" rx="1.5" fill="#fff" opacity="' + (0.2 + i * 0.065).toFixed(2) + '" transform="rotate(' + i * 30 + ' 22 22)"/>';
            return s + '</svg>';
        })()
    };
    var tip = null, tipMask = null;
    function hideTip() {
        if (tip) { tip.remove(); tip = null; }
        if (tipMask) { tipMask.remove(); tipMask = null; }
    }
    function showTip(title, icon, image, mask) {
        hideTip();
        var glyph = image ? '<img class="__wg_icon" src="' + esc(__wgUrl(image)) + '">' : ICONS[icon] || '';
        tip = el(glyph ? '__wg_tip icon' : '__wg_tip text', glyph ? glyph + '<div class="t">' + esc(title) + '</div>' : esc(title));
        // 不带 mask：提示本身也不拦触摸（微信：toast 期间可以继续玩）
        if (mask) { tipMask = el('__wg_mask'); swallow(tipMask); document.body.appendChild(tipMask); }
        else tip.style.pointerEvents = 'none';
        document.body.appendChild(tip);
    }
    G.__native_show_toast = function(title, icon, duration, mask, image) { showTip(title, icon === 'none' ? '' : icon, image, mask === 'true'); return 'undefined'; };
    G.__native_show_loading = function(title, mask) { showTip(title, 'loading', '', mask === 'true'); return 'undefined'; };
    G.__native_hide_toast = G.__native_hide_loading = function() { hideTip(); return 'undefined'; };

    G.__native_show_modal = function(title, content, showCancel, cancelText, confirmText, cancelColor, confirmColor) {
        var mask = el('__wg_mask __wg_dim'); swallow(mask);
        var box = el('__wg_modal');
        box.innerHTML = (title ? '<div class="h">' + esc(title) + '</div>' : '') + (content ? '<div class="c">' + esc(content) + '</div>' : '<div class="c"></div>') + '<div class="b"></div>';
        var row = box.querySelector('.b');
        function btn(text, color, confirm) {
            var b = mk('div');
            b.textContent = text; b.style.color = color;
            tap(b, function() { mask.remove(); box.remove(); __handleModalResult(confirm); });
            row.appendChild(b);
        }
        if (showCancel === 'true') btn(cancelText, cancelColor, false);
        btn(confirmText, confirmColor, true);
        swallow(box);
        document.body.appendChild(mask);
        document.body.appendChild(box);
        return 'undefined';
    };

    function actionSheet(o) {
        var mask = el('__wg_mask __wg_dim'), sheet = el('__wg_sheet');
        function close(i, err) { mask.remove(); sheet.remove(); __resolveActionSheet(o.id, i, err); }
        if (o.alertText) { var a = mk('div'); a.className = 'a'; a.textContent = o.alertText; sheet.appendChild(a); }
        (o.itemList || []).forEach(function(t, i) {
            var d = mk('div'); d.textContent = t; d.style.color = o.itemColor || '#000';
            tap(d, function() { close(i); });
            sheet.appendChild(d);
        });
        var x = mk('div'); x.className = 'x'; x.textContent = '取消';
        tap(x, function() { close(-1, 'cancel'); });
        sheet.appendChild(x);
        swallow(mask); swallow(sheet);
        tap(mask, function() { close(-1, 'cancel'); });
        document.body.appendChild(mask);
        document.body.appendChild(sheet);
    }
    // 页面自己画得了的宿主请求（wg-env.js 的 __native_host_request 先问这里）
    G.__wgHostRequest = function(kind, json) {
        if (kind === 'actionSheet') { try { actionSheet(JSON.parse(json)); } catch (e) { console.error(e); } return true; }
        return false;
    };
})();
