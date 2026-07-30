            var __storage = {};
            
            wx.setStorageSync = function(key, data) {
                var value = typeof data === 'string' ? data : JSON.stringify(data);
                __storage[key] = value;
                if (typeof __native_storage_set === 'function') { __native_storage_set(key, value); }
            };
            
            wx.getStorageSync = function(key) {
                var value = __storage[key];
                if (value === undefined && typeof __native_storage_get === 'function') {
                    value = __native_storage_get(key);
                    if (value) { __storage[key] = value; }
                }
                if (value === undefined || value === '') { return ''; }
                try { return JSON.parse(value); } catch(e) { return value; }
            };
            
            wx.removeStorageSync = function(key) {
                delete __storage[key];
                if (typeof __native_storage_remove === 'function') { __native_storage_remove(key); }
            };
            
            wx.clearStorageSync = function() {
                __storage = {};
                if (typeof __native_storage_clear === 'function') { __native_storage_clear(); }
            };
            
            wx.getStorageInfoSync = function() {
                var keys = Object.keys(__storage);
                var currentSize = 0;
                for (var i = 0; i < keys.length; i++) { currentSize += (__storage[keys[i]] || '').length; }
                return { keys: keys, currentSize: Math.ceil(currentSize / 1024), limitSize: 10240 };
            };
            
            wx.setStorage = function(options) {
                options = options || {};
                try { wx.setStorageSync(options.key, options.data); options.success && options.success(); }
                catch(e) { options.fail && options.fail({ errMsg: 'setStorage:fail' }); }
                options.complete && options.complete();
            };
            
            wx.getStorage = function(options) {
                options = options || {};
                try { var data = wx.getStorageSync(options.key); options.success && options.success({ data: data }); }
                catch(e) { options.fail && options.fail({ errMsg: 'getStorage:fail' }); }
                options.complete && options.complete();
            };
            
            wx.removeStorage = function(options) {
                options = options || {};
                try { wx.removeStorageSync(options.key); options.success && options.success(); }
                catch(e) { options.fail && options.fail({ errMsg: 'removeStorage:fail' }); }
                options.complete && options.complete();
            };
            
            wx.clearStorage = function(options) {
                options = options || {};
                try { wx.clearStorageSync(); options.success && options.success(); }
                catch(e) { options.fail && options.fail({ errMsg: 'clearStorage:fail' }); }
                options.complete && options.complete();
            };
