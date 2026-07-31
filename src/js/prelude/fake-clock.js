            // 可注入的固定时钟（只在设了 MINI_FAKE_NOW 时生效）。
            //
            // 为什么需要它：像素基线里有**跨天就失效**的页面 —— news-app 的签到日历用
            // `new Date()` 高亮当天，隔一天再比就在日历那一格上差 0.1%（实测 155×55 像素
            // 的方框），看着像渲染坏了。`--time` 只固定 CSS 动画时钟，管不到 JS 的 Date。
            //
            // 设了之后：`Date.now()` 与 `new Date()` 都停在那个时刻，带参数的构造照常透传
            // （`new Date(2024, 0, 1)` 这种算日历的用法不能被打断）。
            (function () {
                var fake = 0;
                if (typeof __native_fake_now === 'function') {
                    fake = Number(__native_fake_now()) || 0;
                }
                if (!fake) { return; }
                var RealDate = Date;
                function FakeDate() {
                    if (arguments.length === 0) { return new RealDate(fake); }
                    // 透传其余重载：Reflect.construct 保证 `new` 语义与原生一致
                    return Reflect.construct(RealDate, Array.prototype.slice.call(arguments));
                }
                FakeDate.prototype = RealDate.prototype;
                FakeDate.now = function () { return fake; };
                FakeDate.parse = RealDate.parse;
                FakeDate.UTC = RealDate.UTC;
                Date = FakeDate;
                if (typeof console !== 'undefined' && console.log) {
                    console.log('[固定时钟] Date 已锁在 ' + new RealDate(fake).toISOString());
                }
            })();
