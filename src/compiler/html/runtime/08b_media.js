  // ───────── <video>：组件事件与 VideoContext（与原生端 host/video.rs 同一套事件名与 detail） ─────────
  // 媒体事件不冒泡，所以在捕获阶段统一接；绑定名就是 applyEvents 写上的 data-<事件名>。
  // 浏览器的 timeupdate 频率由内核定（约 4Hz，与官方的 250ms 一致）。
  (function () {
    function fire(el, name, detail) {
      var h = el.getAttribute && el.getAttribute("data-" + name);
      if (h) call(h, evObj(el, detail));
    }
    function isVideo(el) { return el && el.tagName === "VIDEO"; }
    var map = {
      play: function () { return {}; },
      pause: function (v) { return v.ended ? null : {}; },
      ended: function () { return {}; },
      timeupdate: function (v) { return { currentTime: v.currentTime, duration: v.duration || 0 }; },
      loadedmetadata: function (v) { return { width: v.videoWidth, height: v.videoHeight, duration: v.duration || 0 }; },
      seeked: function (v) { return { position: v.currentTime }; }
    };
    Object.keys(map).forEach(function (type) {
      document.addEventListener(type, function (e) {
        var v = e.target; if (!isVideo(v)) return;
        var d = map[type](v); if (!d) return; // 播到末尾浏览器先发 pause：原生端只发 ended
        fire(v, type === "seeked" ? "seekcomplete" : type, d);
      }, true);
    });
    var fsEl = null;
    document.addEventListener("fullscreenchange", function () {
      var cur = document.fullscreenElement;
      var v = isVideo(cur) ? cur : fsEl; if (!v) return;
      fire(v, "fullscreenchange", { fullScreen: !!cur, direction: (v.videoWidth > v.videoHeight) ? "horizontal" : "vertical" });
      fsEl = isVideo(cur) ? cur : null;
    });
    function byId(id) { var el = document.getElementById(id); return isVideo(el) ? el : null; }
    var noop = function () {};
    wx.createVideoContext = function (id) {
      function with_(f) { return function (a) { var v = byId(id); if (v) f(v, a); }; }
      return {
        play: with_(function (v) { var p = v.play(); if (p && p.catch) p.catch(noop); }),
        pause: with_(function (v) { v.pause(); }),
        stop: with_(function (v) { v.pause(); v.currentTime = 0; }),
        seek: with_(function (v, pos) { v.currentTime = Number(pos) || 0; }),
        playbackRate: with_(function (v, r) { v.playbackRate = Math.min(Math.max(Number(r) || 1, 0.5), 2); }),
        requestFullScreen: with_(function (v) { if (v.requestFullscreen) { fsEl = v; v.requestFullscreen().catch(noop); } }),
        exitFullScreen: with_(function () { if (document.fullscreenElement && document.exitFullscreen) document.exitFullscreen().catch(noop); }),
        sendDanmu: noop, showStatusBar: noop, hideStatusBar: noop,
        requestBackgroundPlayback: noop, exitBackgroundPlayback: noop, exitPictureInPicture: noop
      };
    };
  })();
