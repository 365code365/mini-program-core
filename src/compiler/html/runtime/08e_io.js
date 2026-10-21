  // ───────── 第 10 批的 wx.*（与原生端 prelude/socket.js、fs.js、audio.js、app-info.js、codec.js 同语义） ─────────
  // WebSocket（浏览器 WebSocket）、FileSystemManager（内存文件表，USER_DATA_PATH 落 localStorage）、
  // InnerAudioContext（<audio> 元素）、canIUse / 胶囊 / 网络状态 / 分享菜单 / 屏幕 / 背景色 / 键盘 / 图片信息与压缩。
  (function () {
    function done(o, ok, res) {
      o = o || {};
      if (ok) { o.success && o.success(res); } else { o.fail && o.fail(res); }
      o.complete && o.complete(res);
    }
    function syncOk(o, name, res) {
      res = res || {}; res.errMsg = name + ":ok";
      if (o && (o.success || o.fail || o.complete)) { done(o, true, res); return; }
      return Promise.resolve(res);
    }
    function later(o, ok, res) { setTimeout(function () { done(o, ok, res); }, 0); }
    function listeners() {
      var ls = [];
      return { on: function (cb) { if (typeof cb === "function") ls.push(cb); },
               off: function (cb) { ls = cb ? ls.filter(function (f) { return f !== cb; }) : []; },
               fire: function (r) { ls.slice().forEach(function (cb) { try { cb(r); } catch (e) { console.error(e); } }); } };
    }

    // ---------- base64 ----------
    function b64enc(buf) {
      var b = new Uint8Array(buf instanceof ArrayBuffer ? buf : (buf && buf.buffer) || new ArrayBuffer(0)), s = "";
      for (var i = 0; i < b.length; i++) s += String.fromCharCode(b[i]);
      return btoa(s);
    }
    function b64dec(s) {
      var bin = atob(String(s || "").replace(/[\s]/g, "")), out = new Uint8Array(bin.length);
      for (var i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
      return out.buffer;
    }
    wx.arrayBufferToBase64 = b64enc;
    wx.base64ToArrayBuffer = b64dec;

    // ---------- WebSocket ----------
    var latest = null, glob = { open: listeners(), message: listeners(), error: listeners(), close: listeners() };
    wx.connectSocket = function (o) {
      o = o || {};
      var url = String(o.url || ""), task = { readyState: 0, CONNECTING: 0, OPEN: 1, CLOSING: 2, CLOSED: 3 };
      var ls = { open: listeners(), message: listeners(), error: listeners(), close: listeners() };
      ["Open", "Message", "Error", "Close"].forEach(function (N) { var k = N.toLowerCase(); task["on" + N] = ls[k].on; task["off" + N] = ls[k].off; });
      if (!/^wss?:\/\//.test(url)) { later(o, false, { errMsg: 'connectSocket:fail invalid url "' + url + '"' }); task.readyState = 3; return task; }
      var ws;
      try { ws = new WebSocket(url, o.protocols || []); } catch (e) { later(o, false, { errMsg: "connectSocket:fail " + e.message }); return task; }
      ws.binaryType = "arraybuffer";
      latest = task;
      var fire = function (k, r) { ls[k].fire(r); if (latest === task) glob[k].fire(r); };
      ws.onopen = function () { task.readyState = 1; fire("open", { header: {}, protocol: ws.protocol || "" }); };
      ws.onmessage = function (ev) { fire("message", { data: ev.data }); };
      ws.onerror = function () { fire("error", { errMsg: "connect fail" }); };
      ws.onclose = function (ev) { task.readyState = 3; fire("close", { code: ev.code, reason: ev.reason }); };
      task.send = function (so) {
        so = so || {};
        if (ws.readyState !== 1) return done(so, false, { errMsg: "sendSocketMessage:fail WebSocket is not connected" });
        ws.send(so.data); done(so, true, { errMsg: "sendSocketMessage:ok" });
      };
      task.close = function (co) {
        co = co || {}; task.readyState = 2;
        try { ws.close(co.code === undefined ? 1000 : co.code, co.reason || ""); done(co, true, { errMsg: "closeSocket:ok" }); }
        catch (e) { done(co, false, { errMsg: "closeSocket:fail " + e.message }); }
      };
      later(o, true, { errMsg: "connectSocket:ok" });
      return task;
    };
    ["Open", "Message", "Error", "Close"].forEach(function (N) { var k = N.toLowerCase(); wx["onSocket" + N] = glob[k].on; wx["offSocket" + N] = glob[k].off; });
    wx.sendSocketMessage = function (o) { if (!latest) return done(o, false, { errMsg: "sendSocketMessage:fail WebSocket is not connected" }); latest.send(o); };
    wx.closeSocket = function (o) { if (!latest) return done(o, false, { errMsg: "closeSocket:fail WebSocket is not connected" }); latest.close(o); };

    // ---------- FileSystemManager：内存文件表；wxfile://usr 下的持久化到 localStorage ----------
    var USR = "wxfile://usr", files = {}, dirs = {};
    dirs[USR] = true;
    try { var saved = JSON.parse(localStorage.getItem("__wxfs") || "{}"); for (var k in saved) files[k] = new Uint8Array(b64dec(saved[k])); } catch (e) {}
    function persist() {
      try { var o = {}; for (var k in files) if (k.indexOf(USR + "/") === 0) o[k] = b64enc(files[k].buffer); localStorage.setItem("__wxfs", JSON.stringify(o)); } catch (e) {}
    }
    function norm(p) { p = String(p || ""); return p.replace(/\/+$/, ""); }
    function writable(p) { return p === USR || p.indexOf(USR + "/") === 0; }
    function enc(str, e) {
      e = String(e || "utf8").toLowerCase().replace("-", "");
      if (e === "base64") return new Uint8Array(b64dec(str));
      if (e === "hex") { var o = new Uint8Array(str.length / 2); for (var i = 0; i < o.length; i++) o[i] = parseInt(str.substr(i * 2, 2), 16); return o; }
      if (e === "binary" || e === "latin1" || e === "ascii") { var b = new Uint8Array(str.length); for (var j = 0; j < str.length; j++) b[j] = str.charCodeAt(j) & 255; return b; }
      return new TextEncoder().encode(str);
    }
    function dec(bytes, e) {
      e = String(e).toLowerCase().replace("-", "");
      if (e === "base64") return b64enc(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength));
      if (e === "hex") return Array.prototype.map.call(bytes, function (x) { return ("0" + x.toString(16)).slice(-2); }).join("");
      if (e === "binary" || e === "latin1" || e === "ascii") return String.fromCharCode.apply(null, bytes);
      return new TextDecoder().decode(bytes);
    }
    function err(name, msg) { var e = new Error(name + ":fail " + msg); e.errMsg = name + ":fail " + msg; throw e; }
    function parentOk(p) { var d = p.slice(0, p.lastIndexOf("/")); return dirs[d] || d === USR; }
    var ops = {
      access: function (n, a) { var p = norm(a.path); if (!(p in files) && !dirs[p]) err(n, "no such file or directory, access " + p); },
      writeFile: function (n, a, append) {
        var p = norm(a.filePath);
        if (!writable(p)) err(n, "permission denied, open " + p);
        if (!parentOk(p)) err(n, "no such file or directory, open " + p);
        var data = typeof a.data === "string" ? enc(a.data, a.encoding) : new Uint8Array(a.data || new ArrayBuffer(0));
        if (append) { if (!(p in files)) err(n, "no such file or directory, open " + p); var old = files[p], m = new Uint8Array(old.length + data.length); m.set(old); m.set(data, old.length); data = m; }
        files[p] = data; persist();
      },
      readFile: function (n, a) {
        var p = norm(a.filePath), b = files[p];
        if (!b) err(n, "no such file or directory, open " + p);
        if (a.position !== undefined || a.length !== undefined) b = b.subarray(a.position || 0, a.length === undefined ? b.length : (a.position || 0) + a.length);
        return { data: a.encoding ? dec(b, a.encoding) : b.slice().buffer };
      },
      unlink: function (n, a) { var p = norm(a.filePath); if (!writable(p)) err(n, "permission denied, unlink " + p); if (!(p in files)) err(n, "no such file or directory, unlink " + p); delete files[p]; persist(); },
      mkdir: function (n, a) {
        var p = norm(a.dirPath);
        if (!writable(p)) err(n, "permission denied, mkdir " + p);
        if (dirs[p]) err(n, "file already exists " + p);
        if (!a.recursive && !parentOk(p)) err(n, "no such file or directory, mkdir " + p);
        var parts = p.slice(USR.length + 1).split("/"), cur = USR;
        parts.forEach(function (s) { cur += "/" + s; dirs[cur] = true; });
      },
      readdir: function (n, a) {
        var p = norm(a.dirPath); if (!dirs[p]) err(n, "no such file or directory, scandir " + p);
        var out = {};
        Object.keys(files).concat(Object.keys(dirs)).forEach(function (k) { if (k.indexOf(p + "/") === 0) out[k.slice(p.length + 1).split("/")[0]] = 1; });
        return { files: Object.keys(out).sort() };
      },
      rmdir: function (n, a) {
        var p = norm(a.dirPath); if (!writable(p)) err(n, "permission denied, rmdir " + p);
        if (!dirs[p]) err(n, "no such file or directory, rmdir " + p);
        var kids = ops.readdir(n, { dirPath: p }).files;
        if (kids.length && !a.recursive) err(n, "directory not empty " + p);
        Object.keys(files).forEach(function (k) { if (k.indexOf(p + "/") === 0) delete files[k]; });
        Object.keys(dirs).forEach(function (k) { if (k === p || k.indexOf(p + "/") === 0) delete dirs[k]; });
        persist();
      },
      stat: function (n, a) {
        var p = norm(a.path), isDir = !!dirs[p];
        if (!isDir && !(p in files)) err(n, "no such file or directory, stat " + p);
        var mk = function (dir, size) { return { mode: dir ? 16877 : 33188, size: size, lastAccessedTime: 0, lastModifiedTime: 0,
          isDirectory: function () { return dir; }, isFile: function () { return !dir; } }; };
        if (a.recursive && isDir) {
          var list = [{ path: "/", stats: mk(true, 0) }];
          Object.keys(dirs).concat(Object.keys(files)).sort().forEach(function (k) {
            if (k.indexOf(p + "/") === 0) list.push({ path: k.slice(p.length), stats: mk(!!dirs[k], files[k] ? files[k].length : 0) });
          });
          return { stats: list };
        }
        return { stats: mk(isDir, isDir ? 0 : files[p].length) };
      },
      rename: function (n, a) { var o = norm(a.oldPath), q = norm(a.newPath); if (!(o in files)) err(n, "no such file or directory, rename " + o); files[q] = files[o]; delete files[o]; persist(); },
      copyFile: function (n, a) { var s = norm(a.srcPath), d = norm(a.destPath); if (!writable(d)) err(n, "permission denied, copyFile " + d); if (!(s in files)) err(n, "no such file or directory, copyFile " + s); files[d] = files[s].slice(); persist(); },
      getFileInfo: function (n, a) { var p = norm(a.filePath); if (!(p in files)) err(n, "no such file or directory, open " + p); return { size: files[p].length, digest: "" }; }
    };
    var pos = { access: ["path"], writeFile: ["filePath", "data", "encoding"], appendFile: ["filePath", "data", "encoding"], readFile: ["filePath", "encoding", "position", "length"],
      unlink: ["filePath"], mkdir: ["dirPath", "recursive"], readdir: ["dirPath"], rmdir: ["dirPath", "recursive"], stat: ["path", "recursive"],
      rename: ["oldPath", "newPath"], copyFile: ["srcPath", "destPath"] };
    var pick = { readFile: "data", readdir: "files", stat: "stats" };
    var fsm = null;
    wx.getFileSystemManager = function () {
      if (fsm) return fsm;
      fsm = {};
      Object.keys(pos).concat(["getFileInfo"]).forEach(function (op) {
        var impl = op === "appendFile" ? function (n, a) { return ops.writeFile(n, a, true); } : ops[op];
        fsm[op] = function (o) {
          o = o || {};
          try { var r = impl(op, o) || {}; r.errMsg = op + ":ok"; later(o, true, r); }
          catch (e) { later(o, false, { errMsg: e.errMsg || String(e) }); }
        };
        if (pos[op]) fsm[op + "Sync"] = function () {
          var a = arguments, o = {};
          if (typeof a[0] === "object" && a[0] !== null) o = a[0]; else pos[op].forEach(function (k, i) { if (a[i] !== undefined) o[k] = a[i]; });
          var r = impl(op + "Sync", o);
          return r && pick[op] ? r[pick[op]] : r;
        };
      });
      return fsm;
    };

    // ---------- InnerAudioContext：<audio> 元素 ----------
    function makeAudio() {
      var el = new Audio(), ctx = {}, ls = {}, start = 0;
      ["Canplay", "Play", "Pause", "Stop", "Ended", "TimeUpdate", "Error", "Waiting", "Seeking", "Seeked"].forEach(function (N) {
        var k = N.charAt(0).toLowerCase() + N.slice(1); ls[k] = listeners(); ctx["on" + N] = ls[k].on; ctx["off" + N] = ls[k].off;
      });
      var map = { canplay: "canplay", play: "play", pause: "pause", ended: "ended", timeupdate: "timeUpdate", waiting: "waiting", seeking: "seeking", seeked: "seeked" };
      Object.keys(map).forEach(function (ev) { el.addEventListener(ev, function () { ls[map[ev]].fire({}); }); });
      el.addEventListener("error", function () { var c = el.error ? el.error.code : 0; ls.error.fire({ errCode: c === 2 ? 10002 : c === 4 ? 10004 : 10001, errMsg: "audio error" }); });
      el.addEventListener("loadedmetadata", function () { if (start) el.currentTime = start; });
      Object.defineProperty(ctx, "src", { get: function () { return el.getAttribute("src") || ""; }, set: function (v) { el.src = String(v || ""); } });
      Object.defineProperty(ctx, "autoplay", { get: function () { return el.autoplay; }, set: function (v) { el.autoplay = !!v; } });
      Object.defineProperty(ctx, "loop", { get: function () { return el.loop; }, set: function (v) { el.loop = !!v; } });
      Object.defineProperty(ctx, "volume", { get: function () { return el.volume; }, set: function (v) { el.volume = Math.max(0, Math.min(1, v)); } });
      Object.defineProperty(ctx, "playbackRate", { get: function () { return el.playbackRate; }, set: function (v) { el.playbackRate = Math.max(0.5, Math.min(2, v)); } });
      Object.defineProperty(ctx, "startTime", { get: function () { return start; }, set: function (v) { start = Number(v) || 0; } });
      ["currentTime", "duration", "paused"].forEach(function (k) { Object.defineProperty(ctx, k, { get: function () { var v = el[k]; return k === "duration" && !isFinite(v) ? 0 : v; } }); });
      Object.defineProperty(ctx, "buffered", { get: function () { return el.buffered.length ? el.buffered.end(el.buffered.length - 1) : 0; } });
      ctx.obeyMuteSwitch = true;
      ctx.play = function () { var p = el.play(); if (p && p.catch) p.catch(function () {}); };
      ctx.pause = function () { el.pause(); };
      ctx.stop = function () { el.pause(); el.currentTime = 0; ls.stop.fire({}); };
      ctx.seek = function (t) { el.currentTime = Number(t) || 0; };
      ctx.destroy = function () { el.pause(); el.removeAttribute("src"); el.load(); };
      return ctx;
    }
    wx.createInnerAudioContext = makeAudio;
    wx.setInnerAudioOption = function (o) { return syncOk(o, "setInnerAudioOption"); };
    var bg = null;
    wx.getBackgroundAudioManager = function () {
      if (bg) return bg;
      bg = makeAudio();
      var d = Object.getOwnPropertyDescriptor(bg, "src");
      Object.defineProperty(bg, "src", { get: d.get, set: function (v) { d.set(v); if (v) bg.play(); } });
      ["title", "epname", "singer", "coverImgUrl", "webUrl", "protocol"].forEach(function (k) { bg[k] = ""; });
      bg.onPrev = bg.onNext = function () {};
      return bg;
    };
    wx.createAudioContext = function () { var a = makeAudio(); return { setSrc: function (s) { a.src = s; }, play: a.play, pause: a.pause, seek: a.seek }; };
    wx.getRecorderManager = function () {
      var err = listeners(), r = { onError: err.on };
      ["onStart", "onStop", "onPause", "onResume", "onFrameRecorded", "onInterruptionBegin", "onInterruptionEnd"].forEach(function (k) { r[k] = function () {}; });
      ["start", "pause", "resume", "stop"].forEach(function (k) { r[k] = function () { setTimeout(function () { err.fire({ errMsg: "operateRecorder:fail " + k + " 当前宿主未实现录音" }); }, 0); }; });
      return r;
    };

    // ---------- 网络状态 ----------
    var netLs = listeners();
    function netType() { if (!navigator.onLine) return "none"; var c = navigator.connection; return c && c.type === "cellular" ? (c.effectiveType || "4g") : "wifi"; }
    wx.getNetworkType = function (o) { return syncOk(o, "getNetworkType", { networkType: netType(), signalStrength: 0, hasSystemProxy: false }); };
    wx.onNetworkStatusChange = netLs.on; wx.offNetworkStatusChange = netLs.off;
    wx.onNetworkWeakChange = function () {}; wx.offNetworkWeakChange = function () {};
    window.addEventListener("online", function () { netLs.fire({ isConnected: true, networkType: netType() }); });
    window.addEventListener("offline", function () { netLs.fire({ isConnected: false, networkType: "none" }); });

    // ---------- 胶囊、能力探测、更新、账号、授权、分享 ----------
    wx.getMenuButtonBoundingClientRect = function () {
      var w = (wx.getSystemInfoSync().windowWidth || 375), top = 20 + 4, right = w - 7;
      return { width: 87, height: 32, top: top, bottom: top + 32, left: right - 87, right: right };
    };
    var comps = ["view", "block", "scroll-view", "swiper", "swiper-item", "text", "rich-text", "icon", "image", "video", "canvas", "button", "input",
      "textarea", "switch", "slider", "progress", "checkbox", "checkbox-group", "radio", "radio-group", "picker", "picker-view", "picker-view-column", "form", "navigator", "label", "map"];
    wx.canIUse = function (schema) {
      var parts = String(schema || "").split("."), head = parts[0] === "wx" ? parts[1] : parts[0];
      if (!head) return false;
      if (comps.indexOf(head) >= 0) return true;
      var f = wx[head];
      return typeof f === "function" ? !f.__default : (f !== undefined && f !== null);
    };
    wx.getUpdateManager = function () {
      return { applyUpdate: function () {}, onCheckForUpdate: function (cb) { setTimeout(function () { cb && cb({ hasUpdate: false }); }, 0); },
               onUpdateReady: function () {}, onUpdateFailed: function () {} };
    };
    wx.getAccountInfoSync = function () { return { miniProgram: { appId: "", envVersion: "develop", version: "" }, plugin: {} }; };
    wx.getSetting = function (o) { return syncOk(o, "getSetting", { authSetting: {}, subscriptionsSetting: { mainSwitch: true } }); };
    ["showShareMenu", "hideShareMenu", "updateShareMenu", "showShareImageMenu"].forEach(function (n) { wx[n] = function (o) { return syncOk(o, n); }; });

    // ---------- 屏幕、背景色、电量、键盘 ----------
    var lock = null;
    wx.setKeepScreenOn = function (o) {
      o = o || {};
      if (o.keepScreenOn && navigator.wakeLock) navigator.wakeLock.request("screen").then(function (l) { lock = l; }).catch(function () {});
      else if (lock) { lock.release(); lock = null; }
      return syncOk(o, "setKeepScreenOn");
    };
    var bright = 0.5;
    wx.setScreenBrightness = function (o) { bright = Math.max(0, Math.min(1, Number(o && o.value))); return syncOk(o, "setScreenBrightness"); };
    wx.getScreenBrightness = function (o) { return syncOk(o, "getScreenBrightness", { value: bright }); };
    wx.onUserCaptureScreen = function () {}; wx.offUserCaptureScreen = function () {};
    // 与原生端同一个优先级：页面写了 page 背景就是它，没写才用 setBackgroundColor 的颜色
    wx.setBackgroundColor = function (o) {
      o = o || {};
      var c = o.backgroundColor || o.backgroundColorTop;
      if (c) document.documentElement.style.backgroundColor = c;
      return syncOk(o, "setBackgroundColor");
    };
    wx.setBackgroundTextStyle = function (o) { return syncOk(o, "setBackgroundTextStyle"); };
    wx.getBatteryInfoSync = function () { return { level: 100, isCharging: true }; };
    wx.getBatteryInfo = function (o) { return syncOk(o, "getBatteryInfo", wx.getBatteryInfoSync()); };
    var kb = listeners();
    wx.hideKeyboard = function (o) { if (document.activeElement && document.activeElement.blur) document.activeElement.blur(); return syncOk(o, "hideKeyboard"); };
    wx.onKeyboardHeightChange = kb.on; wx.offKeyboardHeightChange = kb.off;

    // ---------- 日志 / 上报 / 性能 / 杂项 ----------
    wx.getLogManager = function () { return { log: console.log.bind(console), info: console.info.bind(console), warn: console.warn.bind(console), debug: console.debug.bind(console) }; };
    wx.getRealtimeLogManager = function () { var m = { info: console.info.bind(console), warn: console.warn.bind(console), error: console.error.bind(console), setFilterMsg: function () {}, addFilterMsg: function () {} }; m.tag = function () { return m; }; return m; };
    wx.reportAnalytics = wx.reportEvent = wx.reportMonitor = wx.reportPerformance = function () {};
    wx.getPerformance = function () { return { now: function () { return performance.now(); }, getEntries: function () { return []; }, getEntriesByName: function () { return []; },
      getEntriesByType: function () { return []; }, createObserver: function () { return { observe: function () {}, disconnect: function () {} }; }, setBufferSize: function () {} }; };
    wx.onMemoryWarning = function () {}; wx.offMemoryWarning = function () {};
    wx.getExtConfigSync = function () { return {}; };
    wx.getExtConfig = function (o) { return syncOk(o, "getExtConfig", { extConfig: {} }); };

    // ---------- 图片信息与压缩：<img> 量尺寸，<canvas> 重新编码 ----------
    function loadImg(src, cb) { var im = new Image(); im.onload = function () { cb(null, im); }; im.onerror = function () { cb("load fail"); }; im.src = src; }
    wx.getImageInfo = function (o) {
      o = o || {};
      var run = function (oo) {
        loadImg(o.src, function (e, im) {
          if (e) return done(oo, false, { errMsg: "getImageInfo:fail " + e });
          var m = /\.(png|jpe?g|gif|webp|bmp)(\?|$)/i.exec(o.src) || /^data:image\/(\w+)/.exec(o.src);
          done(oo, true, { width: im.naturalWidth, height: im.naturalHeight, type: m ? m[1].toLowerCase().replace("jpg", "jpeg") : "unknown",
                           orientation: "up", path: o.src, errMsg: "getImageInfo:ok" });
        });
      };
      if (o.success || o.fail || o.complete) return run(o);
      return new Promise(function (resolve, reject) { run({ success: resolve, fail: reject }); });
    };
    wx.compressImage = function (o) {
      o = o || {};
      var run = function (oo) {
        loadImg(o.src, function (e, im) {
          if (e) return done(oo, false, { errMsg: "compressImage:fail " + e });
          var w = o.compressedWidth || 0, h = o.compressedHeight || 0, iw = im.naturalWidth, ih = im.naturalHeight;
          if (w && !h) h = Math.round(w * ih / iw); else if (h && !w) w = Math.round(h * iw / ih); else if (!w) { w = iw; h = ih; }
          var c = document.createElement("canvas"); c.width = w; c.height = h;
          c.getContext("2d").drawImage(im, 0, 0, w, h);
          done(oo, true, { tempFilePath: c.toDataURL("image/jpeg", (o.quality === undefined ? 80 : o.quality) / 100), errMsg: "compressImage:ok" });
        });
      };
      if (o.success || o.fail || o.complete) return run(o);
      return new Promise(function (resolve, reject) { run({ success: resolve, fail: reject }); });
    };
  })();
