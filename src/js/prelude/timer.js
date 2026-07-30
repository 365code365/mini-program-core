            var __timers = {};
            var __timer_id = 0;
            
            function setTimeout(callback, delay) {
                delay = delay || 0;
                var id = ++__timer_id;
                __timers[id] = { callback: callback, delay: delay, type: 'timeout' };
                if (typeof __native_set_timer === 'function') {
                    __native_set_timer(String(id), String(delay), 'false');
                }
                return id;
            }
            
            function setInterval(callback, delay) {
                delay = delay || 0;
                var id = ++__timer_id;
                __timers[id] = { callback: callback, delay: delay, type: 'interval' };
                if (typeof __native_set_timer === 'function') {
                    __native_set_timer(String(id), String(delay), 'true');
                }
                return id;
            }
            
            function clearTimeout(id) {
                if (__timers[id]) {
                    delete __timers[id];
                    if (typeof __native_clear_timer === 'function') {
                        __native_clear_timer(String(id));
                    }
                }
            }
            
            function clearInterval(id) { clearTimeout(id); }
            
            function __trigger_timer(id) {
                var timer = __timers[id];
                if (timer && typeof timer.callback === 'function') {
                    try { timer.callback(); } catch (e) { console.error('Timer error:', e); }
                    if (timer.type === 'timeout') { delete __timers[id]; }
                }
            }
