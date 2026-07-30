            if (typeof Proxy === 'function' && typeof wx === 'object' && !wx.__probed) {
                var __wxImpl = wx;
                var __warned = {};
                __wxImpl.__probed = true;
                wx = new Proxy(__wxImpl, {
                    get: function (target, key) {
                        if (!(key in target) && typeof key === 'string' && key.indexOf('__') !== 0) {
                            if (!__warned[key]) {
                                __warned[key] = true;
                                if (typeof console !== 'undefined' && console.warn) {
                                    console.warn('[未实现的 API] wx.' + key);
                                }
                            }
                            return undefined;
                        }
                        return target[key];
                    }
                });
                // uni-app / Taro 会把 uni 指向 wx，保持同一层探针
                if (typeof uni !== 'undefined' && uni === __wxImpl) { uni = wx; }
            }
