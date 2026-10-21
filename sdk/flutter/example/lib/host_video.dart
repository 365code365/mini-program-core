import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:mini_render/mini_render.dart';
import 'package:video_player/video_player.dart';

/// 宿主承载的 `<video>`：引擎只报位置与属性（`nativeComponents` 点名了 video，引擎不解码），
/// 画面由这里的 `video_player`（Android ExoPlayer / iOS AVPlayer，系统硬解）出。
///
/// 支持的属性：`src`（网络地址 / 包内路径）、`autoplay`、`loop`、`muted`、`controls`、
/// `object-fit`（contain / fill / cover）、`poster`。
///
/// 回传的事件与微信 `<video>` 同名同形（经 `sendNativeViewEvent` 派发到节点的 `bind*`）：
/// `loadedmetadata {width, height, duration}`、`play {}`、`pause {}`、`waiting {}`、
/// `timeupdate {currentTime, duration}`（每 250ms，单位秒）、`ended {}`、`error {errMsg}`。
///
/// 注意：网络地址要 https —— Android 9+ 默认禁止明文 HTTP，iOS 的 ATS 也一样，
/// http 地址在这里会走 `binderror`（errMsg 里是播放器给的原因）。
class HostVideo extends StatefulWidget {
  const HostVideo({super.key, required this.view, required this.controller});

  final MiniNativeView view;
  final MiniProgramController controller;

  @override
  State<HostVideo> createState() => _HostVideoState();
}

class _HostVideoState extends State<HostVideo> {
  VideoPlayerController? _player;
  String _src = '';
  String? _error;
  Timer? _ticker;
  bool _wasPlaying = false;
  bool _wasBuffering = false;
  bool _ended = false;
  bool _started = false;

  Map<String, String> get _attrs => widget.view.attrs;
  bool _flag(String name) => _attrs[name] == 'true';

  void _send(String name, [Map<String, Object?> detail = const {}]) =>
      widget.controller.sendNativeViewEvent(widget.view.key, name, detail);

  static double _secs(Duration d) => d.inMilliseconds / 1000.0;

  @override
  void initState() {
    super.initState();
    _open(_attrs['src'] ?? '');
  }

  @override
  void didUpdateWidget(HostVideo old) {
    super.didUpdateWidget(old);
    // 滚动时每帧都会带着新位置重建；只有 src 变了才换片源
    final src = _attrs['src'] ?? '';
    if (src != _src) {
      _close();
      _open(src);
    }
    final p = _player;
    if (p != null && p.value.isInitialized) {
      p.setLooping(_flag('loop'));
      p.setVolume(_flag('muted') ? 0 : 1);
    }
  }

  /// 网络地址直接给播放器；包内路径按小程序根目录（`/` 开头）或当前页目录解析
  VideoPlayerController? _controllerFor(String src) {
    if (src.startsWith('https://') || src.startsWith('http://')) {
      return VideoPlayerController.networkUrl(Uri.parse(src));
    }
    final root = widget.controller.appDir;
    final route = widget.controller.page.value?.route ?? '';
    final dir = route.contains('/') ? route.substring(0, route.lastIndexOf('/')) : '';
    final path = src.startsWith('/') ? '$root$src' : '$root/$dir/$src';
    final file = File(path);
    return file.existsSync() ? VideoPlayerController.file(file) : null;
  }

  Future<void> _open(String src) async {
    _src = src;
    _error = null;
    _ended = false;
    _started = false;
    _wasPlaying = false;
    _wasBuffering = false;
    if (src.isEmpty) return;
    final p = _controllerFor(src);
    if (p == null) {
      _fail('src 找不到：$src');
      return;
    }
    _player = p;
    p.addListener(_onValue);
    try {
      await p.initialize();
    } catch (e) {
      if (_player == p) _fail('$e');
      return;
    }
    if (!mounted || _player != p) return;
    await p.setLooping(_flag('loop'));
    await p.setVolume(_flag('muted') ? 0 : 1);
    final v = p.value;
    _send('loadedmetadata', {
      'width': v.size.width.round(),
      'height': v.size.height.round(),
      'duration': _secs(v.duration),
    });
    setState(() {});
    if (_flag('autoplay')) await p.play();
  }

  void _close() {
    _ticker?.cancel();
    _ticker = null;
    final p = _player;
    _player = null;
    p?.removeListener(_onValue);
    p?.dispose();
  }

  void _fail(String msg) {
    _error = msg;
    _send('error', {'errMsg': msg});
    if (mounted) setState(() {});
  }

  /// 播放器状态的跳变 → 小程序事件（点按、播完、缓冲都从这里出，不在各处分别发）
  void _onValue() {
    final p = _player;
    if (p == null) return;
    final v = p.value;
    if (v.hasError && _error == null) {
      _fail(v.errorDescription ?? 'unknown');
      return;
    }
    if (v.isPlaying != _wasPlaying) {
      _wasPlaying = v.isPlaying;
      if (v.isPlaying) {
        _started = true;
        _ended = false;
        _send('play');
        _ticker ??= Timer.periodic(const Duration(milliseconds: 250), (_) => _tick());
      } else {
        _ticker?.cancel();
        _ticker = null;
        // 播到头时播放器也会报一次「停了」：那是 ended，不是 pause
        if (!v.isCompleted) _send('pause');
      }
      if (mounted) setState(() {});
    }
    if (v.isCompleted && !_ended && !v.isPlaying) {
      _ended = true;
      _send('timeupdate', {'currentTime': _secs(v.duration), 'duration': _secs(v.duration)});
      _send('ended');
      if (mounted) setState(() {});
    }
    if (v.isBuffering != _wasBuffering) {
      _wasBuffering = v.isBuffering;
      if (v.isBuffering && _started) _send('waiting');
    }
  }

  /// `value.position` 是插件每 500ms 刷一次的缓存，按 250ms 发 timeupdate 要现取
  Future<void> _tick() async {
    final p = _player;
    if (p == null || !p.value.isPlaying) return;
    final pos = await p.position ?? p.value.position;
    if (_player != p) return;
    _send('timeupdate', {'currentTime': _secs(pos), 'duration': _secs(p.value.duration)});
  }

  void _toggle() {
    final p = _player;
    if (p == null || !p.value.isInitialized) return;
    if (p.value.isPlaying) {
      p.pause();
    } else {
      // 播完了再点：从头放（与微信一致）
      if (_ended) p.seekTo(Duration.zero);
      p.play();
    }
  }

  @override
  void dispose() {
    _close();
    super.dispose();
  }

  BoxFit get _fit => switch (_attrs['object-fit']) {
        'fill' => BoxFit.fill,
        'cover' => BoxFit.cover,
        _ => BoxFit.contain,
      };

  @override
  Widget build(BuildContext context) {
    final p = _player;
    final ready = p != null && p.value.isInitialized;
    final playing = ready && p.value.isPlaying;
    final poster = _attrs['poster'] ?? '';
    final controls = _attrs['controls'] != 'false';
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: _toggle,
      child: ColoredBox(
        color: Colors.black,
        child: Stack(fit: StackFit.expand, children: [
          if (ready)
            ClipRect(
              child: FittedBox(
                fit: _fit,
                child: SizedBox(width: p.value.size.width, height: p.value.size.height, child: VideoPlayer(p)),
              ),
            ),
          if (!_started && poster.startsWith('http'))
            Image.network(poster, fit: _fit, errorBuilder: (_, _, _) => const SizedBox.shrink()),
          if (_error != null)
            Center(
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Text('视频加载失败\n$_error',
                    textAlign: TextAlign.center,
                    maxLines: 4,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(color: Colors.white70, fontSize: 12)),
              ),
            )
          else if (!ready)
            const Center(
              child: SizedBox.square(
                  dimension: 28, child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white70)),
            )
          else if (!playing && controls)
            const Center(child: Icon(Icons.play_circle, color: Colors.white70, size: 52)),
          if (ready && controls)
            Positioned(
              left: 0,
              right: 0,
              bottom: 0,
              child: VideoProgressIndicator(
                p,
                allowScrubbing: true,
                padding: const EdgeInsets.only(top: 8),
                colors: const VideoProgressColors(playedColor: Color(0xFF07C160)),
              ),
            ),
        ]),
      ),
    );
  }
}
