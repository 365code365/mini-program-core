  // ───────── 第 8 批的 wx.*（与原生端 prelude/system.js、tabbar.js、network.js 同语义） ─────────
  // showActionSheet 面板、previewImage 浮层、原生 tabBar 运行时 API、剪贴板、振动、
  // 「要宿主才能做」的能力的默认实现、downloadFile / uploadFile（fetch）。
  (function () {
    function done(o, ok, res) {
      o = o || {};
      if (ok) { o.success && o.success(res); } else { o.fail && o.fail(res); }
      o.complete && o.complete(res);
    }
    function asPromise(o, run) {
      if (o && (o.success || o.fail || o.complete)) { run(o); return; }
      return new Promise(function (resolve, reject) { run({ success: resolve, fail: reject }); });
    }

    // ---------- showActionSheet ----------
    wx.showActionSheet = function (opt) {
      return asPromise(opt, function (o) {
        var items = (opt && opt.itemList || []).map(String);
        if (!items.length || items.length > 6) {
          return done(o, false, { errMsg: "showActionSheet:fail parameter error: itemList should not be " + (items.length ? "large than 6" : "empty") });
        }
        var mask = document.createElement("div");
        mask.style.cssText = "position:fixed;inset:0;background:rgba(0,0,0,.6);z-index:10002;";
        var sheet = document.createElement("div");
        sheet.style.cssText = "position:fixed;left:0;right:0;bottom:0;background:#f7f7f7;border-radius:12px 12px 0 0;overflow:hidden;z-index:10003;transform:translateY(100%);transition:transform .3s ease-out;";
        var html = "";
        // 标题：56 高、12px、FG-1、最多两行（WeUI 2.x weui-actionsheet__title，与原生端 action_sheet.rs 同值）
        if (opt.alertText) html += "<div style='background:#fff;color:rgba(0,0,0,.5);font-size:12px;line-height:1.4;height:56px;padding:0 24px;display:flex;flex-direction:column;justify-content:center;text-align:center;border-bottom:1px solid #e5e5e5;'><div style='display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;'>" + esc(opt.alertText) + "</div></div>";
        items.forEach(function (t, i) {
          html += "<div data-i='" + i + "' style='background:#fff;color:" + esc(opt.itemColor || "#000000") + ";font-size:17px;line-height:56px;text-align:center;" + (i + 1 < items.length ? "border-bottom:1px solid #e5e5e5;" : "") + "cursor:pointer;'>" + esc(t) + "</div>";
        });
        html += "<div data-i='-1' style='background:#fff;margin-top:8px;color:rgba(0,0,0,.9);font-size:17px;line-height:56px;text-align:center;cursor:pointer;'>取消</div>";
        sheet.innerHTML = html;
        document.body.appendChild(mask); document.body.appendChild(sheet);
        requestAnimationFrame(function () { sheet.style.transform = "translateY(0)"; });
        function close(i) {
          sheet.style.transform = "translateY(100%)";
          setTimeout(function () { mask.remove(); sheet.remove(); }, 310);
          if (i >= 0) done(o, true, { tapIndex: i, errMsg: "showActionSheet:ok" });
          else done(o, false, { errMsg: "showActionSheet:fail cancel" });
        }
        mask.addEventListener("click", function () { close(-1); });
        sheet.addEventListener("click", function (e) {
          var row = e.target.closest && e.target.closest("[data-i]");
          if (row) close(parseInt(row.getAttribute("data-i"), 10));
        });
      });
    };

    // ---------- previewImage ----------
    wx.previewImage = function (opt) {
      opt = opt || {};
      var urls = (opt.urls || []).map(String);
      if (!urls.length) return asPromise(opt, function (o) { done(o, false, { errMsg: "previewImage:fail parameter error: urls should not be empty" }); });
      var idx = typeof opt.current === "number" ? opt.current : Math.max(urls.indexOf(String(opt.current)), 0);
      var box = document.createElement("div");
      box.style.cssText = "position:fixed;inset:0;background:#000;z-index:10004;display:flex;align-items:center;justify-content:center;opacity:0;transition:opacity .2s;";
      var img = document.createElement("img");
      img.style.cssText = "max-width:100%;max-height:100%;object-fit:contain;transition:transform .2s;";
      var label = document.createElement("div");
      label.style.cssText = "position:absolute;left:0;right:0;bottom:40px;text-align:center;color:#fff;font-size:14px;";
      box.appendChild(img); box.appendChild(label); document.body.appendChild(box);
      function show() { img.src = assetUrl(urls[idx]); label.textContent = urls.length > 1 ? (idx + 1) + " / " + urls.length : ""; }
      show();
      requestAnimationFrame(function () { box.style.opacity = "1"; });
      var x0 = null, moved = 0;
      function down(x) { x0 = x; moved = 0; }
      function move(x) { if (x0 == null) return; moved = x - x0; img.style.transform = "translateX(" + moved + "px)"; }
      function up() {
        if (x0 == null) return;
        img.style.transform = "";
        if (Math.abs(moved) < 8) { box.style.opacity = "0"; setTimeout(function () { box.remove(); }, 200); }
        else if (moved <= -60 && idx + 1 < urls.length) { idx++; show(); }
        else if (moved >= 60 && idx > 0) { idx--; show(); }
        x0 = null;
      }
      box.addEventListener("mousedown", function (e) { down(e.clientX); });
      box.addEventListener("mousemove", function (e) { move(e.clientX); });
      box.addEventListener("mouseup", up);
      box.addEventListener("touchstart", function (e) { down(e.touches[0].clientX); }, { passive: true });
      box.addEventListener("touchmove", function (e) { move(e.touches[0].clientX); }, { passive: true });
      box.addEventListener("touchend", up);
      return asPromise(opt, function (o) { done(o, true, { errMsg: "previewImage:ok" }); });
    };

    // ---------- 剪贴板 / 振动 ----------
    var clip = "";
    wx.setClipboardData = function (opt) {
      return asPromise(opt, function (o) {
        clip = String(opt && opt.data != null ? opt.data : "");
        if (navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(clip).catch(function () {});
        wx.showToast({ title: "内容已复制", icon: "none", duration: 1500 });
        done(o, true, { errMsg: "setClipboardData:ok" });
      });
    };
    wx.getClipboardData = function (opt) { return asPromise(opt, function (o) { done(o, true, { data: clip, errMsg: "getClipboardData:ok" }); }); };
    wx.vibrateShort = function (opt) { if (navigator.vibrate) navigator.vibrate(15); return asPromise(opt, function (o) { done(o, true, { errMsg: "vibrateShort:ok" }); }); };
    wx.vibrateLong = function (opt) { if (navigator.vibrate) navigator.vibrate(400); return asPromise(opt, function (o) { done(o, true, { errMsg: "vibrateLong:ok" }); }); };

    // ---------- 原生 tabBar 运行时 API（导出页里的 nav.wx-tabbar） ----------
    function bar() { return document.querySelector(".wx-tabbar"); }
    function item(i) { var b = bar(); return b ? b.querySelectorAll(".wx-tabbar-item")[i] : null; }
    function tabOp(name, opt, fn) {
      return asPromise(opt, function (o) {
        var b = bar();
        if (!b) return done(o, false, { errMsg: name + ":fail not TabBar page" });
        var err = fn(b, opt || {});
        if (err) done(o, false, { errMsg: name + ":fail " + err }); else done(o, true, { errMsg: name + ":ok" });
      });
    }
    function mark(el, kind, text) {
      var old = el.querySelector(".wx-tabbar-mark"); if (old) old.remove();
      if (!kind) return;
      var m = document.createElement("span"); m.className = "wx-tabbar-mark";
      m.style.cssText = "position:absolute;top:2px;left:50%;background:#fa5151;color:#fff;font-size:12px;line-height:18px;border-radius:9px;" +
        (kind === "dot" ? "width:8px;height:8px;margin-left:10px;padding:0;" : "min-width:18px;height:18px;margin-left:4px;padding:0 5px;box-sizing:border-box;text-align:center;");
      if (kind === "badge") m.textContent = text.length > 4 ? text.slice(0, 3) + "…" : text;
      el.style.position = "relative"; el.appendChild(m);
    }
    function withItem(fn) { return function (b, o) { var el = item(Number(o.index)); if (!el) return "tabbar item not found: index " + o.index; fn(el, o); }; }
    wx.setTabBarBadge = function (o) { return tabOp("setTabBarBadge", o, withItem(function (el, o) { mark(el, "badge", String(o.text || "")); })); };
    wx.removeTabBarBadge = function (o) { return tabOp("removeTabBarBadge", o, withItem(function (el) { mark(el, null); })); };
    wx.showTabBarRedDot = function (o) { return tabOp("showTabBarRedDot", o, withItem(function (el) { mark(el, "dot"); })); };
    wx.hideTabBarRedDot = function (o) { return tabOp("hideTabBarRedDot", o, withItem(function (el) { mark(el, null); })); };
    wx.setTabBarItem = function (o) { return tabOp("setTabBarItem", o, withItem(function (el, o) { if (o.text != null) { var t = el.querySelector(".wx-tabbar-text"); if (t) t.textContent = String(o.text); } })); };
    wx.setTabBarStyle = function (o) {
      return tabOp("setTabBarStyle", o, function (b, o) {
        if (o.backgroundColor) b.style.background = o.backgroundColor;
        Array.prototype.forEach.call(b.querySelectorAll(".wx-tabbar-item"), function (el) {
          var active = el.getAttribute("href") && location.pathname.indexOf(el.getAttribute("href").replace(/^(\.\.\/)+/, "")) >= 0;
          var c = active ? o.selectedColor : o.color; if (c) el.style.color = c;
        });
      });
    };
    wx.showTabBar = function (o) { return tabOp("showTabBar", o, function (b) { b.style.display = ""; }); };
    wx.hideTabBar = function (o) { return tabOp("hideTabBar", o, function (b) { b.style.display = "none"; }); };

    // ---------- 要宿主才能做的能力：浏览器里一律立刻 fail ----------
    ["login", "getUserProfile", "getUserInfo", "requestPayment", "scanCode", "getLocation", "chooseLocation",
     "chooseImage", "chooseMedia", "chooseVideo", "chooseMessageFile", "makePhoneCall", "authorize"].forEach(function (name) {
      if (typeof wx[name] === "function") return;
      wx[name] = function (opt) { return asPromise(opt, function (o) { setTimeout(function () { done(o, false, { errMsg: name + ":fail 当前宿主未实现" }); }, 0); }); };
    });

    // ---------- loadFontFace（浏览器自己的 FontFace） ----------
    wx.loadFontFace = function (opt) {
      return asPromise(opt, function (o) {
        var src = String(opt && opt.source || "");
        var m = /url\(\s*['"]?([^'")]+)['"]?\s*\)/.exec(src);
        var url = assetUrl(m ? m[1] : src.trim());
        if (typeof FontFace !== "function") return done(o, false, { status: "error", errMsg: "loadFontFace:fail FontFace unsupported" });
        new FontFace(String(opt.family || ""), "url(" + JSON.stringify(url) + ")").load().then(function (f) {
          document.fonts.add(f);
          done(o, true, { status: "loaded", errMsg: "loadFontFace:ok" });
        }).catch(function (e) { done(o, false, { status: "error", errMsg: "loadFontFace:fail " + e }); });
      });
    };

    // ---------- downloadFile / uploadFile ----------
    wx.env = wx.env || { USER_DATA_PATH: "wxfile://usr" };
    wx.downloadFile = function (opt) {
      opt = opt || {};
      fetch(opt.url, { headers: opt.header }).then(function (r) {
        return r.blob().then(function (b) { done(opt, true, { tempFilePath: URL.createObjectURL(b), statusCode: r.status, errMsg: "downloadFile:ok" }); });
      }).catch(function (e) { done(opt, false, { errMsg: "downloadFile:fail " + e }); });
      return { abort: function () {}, onProgressUpdate: function () {}, offProgressUpdate: function () {} };
    };
    wx.uploadFile = function (opt) {
      opt = opt || {};
      fetch(assetUrl(String(opt.filePath || ""))).then(function (r) { return r.blob(); }).then(function (blob) {
        var fd = new FormData();
        var form = opt.formData || {};
        for (var k in form) fd.append(k, form[k]);
        fd.append(opt.name || "file", blob, String(opt.filePath || "file").split("/").pop());
        var h = {}; for (var hk in (opt.header || {})) if (hk.toLowerCase() !== "content-type") h[hk] = opt.header[hk];
        return fetch(opt.url, { method: "POST", body: fd, headers: h });
      }).then(function (r) {
        return r.text().then(function (t) { done(opt, true, { data: t, statusCode: r.status, errMsg: "uploadFile:ok" }); });
      }).catch(function (e) { done(opt, false, { errMsg: "uploadFile:fail " + e }); });
      return { abort: function () {}, onProgressUpdate: function () {}, offProgressUpdate: function () {} };
    };
  })();
