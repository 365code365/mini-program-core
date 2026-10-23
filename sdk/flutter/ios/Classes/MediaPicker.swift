import AVFoundation
import Flutter
import MobileCoreServices
import Photos
import PhotosUI
import UIKit

/// 插件自带的宿主能力的 iOS 一半（Dart 那一半见 `lib/src/media.dart`；与 Android 的 MediaPicker.kt
/// 同一套通道方法与错误码）：
///
/// - `pickMedia` {dir, kind: image|video|mix, source: album|camera, count, compressed, maxDuration, camera}
/// - `pickFiles` {dir, count, type: all|image|video|file, extension}
/// - `saveToAlbum` {path, kind}
/// - `openDocument` {path, fileType}
///
/// 错误码：`cancel` / `denied` / `not_found` / `busy` / `io`。
///
/// 相册：iOS 14+ 用 `PHPickerViewController`（不要相册权限，用户只交出选中的那几张），iOS 13 退到
/// `UIImagePickerController`（只能选一张）。相机：`UIImagePickerController`。**Info.plist 缺用途说明时
/// 系统会直接杀进程**，所以先查：相机 `NSCameraUsageDescription`（录像另要 `NSMicrophoneUsageDescription`）、
/// 存相册 `NSPhotoLibraryAddUsageDescription`，缺了走 `denied`。
///
/// 照片一律按方向摆正后再交（iPhone 竖拍的 JPEG / HEIC 是「横着存 + EXIF 方向」，引擎画图不读 EXIF）；
/// HEIC 转 JPEG（引擎解不了）；GIF / PNG 原样。不用 UniformTypeIdentifiers（iOS 14 才有，插件最低 iOS 13），
/// 类型一律写字符串 UTI。
final class MediaPicker: NSObject {
    private var result: FlutterResult?
    private var dir = URL(fileURLWithPath: NSTemporaryDirectory())
    private var count = 1
    private var compressed = false
    private var fileType = "all"
    private var exts: [String] = []
    private var document: UIDocumentInteractionController?
    private static var seq = 0

    func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
        let a = call.arguments as? [String: Any] ?? [:]
        switch call.method {
        case "pickMedia": pickMedia(a, result)
        case "pickFiles": pickFiles(a, result)
        case "saveToAlbum": saveToAlbum(a, result)
        case "openDocument": openDocument(a, result)
        default: result(FlutterMethodNotImplemented)
        }
    }

    private static func err(_ code: String, _ msg: String? = nil) -> FlutterError {
        FlutterError(code: code, message: msg, details: nil)
    }

    /// 开一个选择器之前：同一时刻只开一个，记下 tempDir 与参数
    private func begin(_ a: [String: Any], _ result: @escaping FlutterResult) -> Bool {
        if self.result != nil {
            result(Self.err("busy", "上一次选择还没结束"))
            return false
        }
        guard let d = a["dir"] as? String, !d.isEmpty else {
            result(Self.err("io", "缺 dir"))
            return false
        }
        dir = URL(fileURLWithPath: d)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        count = max(1, (a["count"] as? NSNumber)?.intValue ?? 1)
        compressed = (a["compressed"] as? Bool) ?? false
        self.result = result
        return true
    }

    private func finish(_ value: Any?) {
        let r = result
        result = nil
        DispatchQueue.main.async { r?(value) }
    }

    private func present(_ vc: UIViewController) {
        guard let top = Self.topViewController() else {
            finish(Self.err("io", "找不到可以弹出选择器的界面"))
            return
        }
        top.present(vc, animated: true)
    }

    static func topViewController() -> UIViewController? {
        let windows = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.flatMap { $0.windows }
        var vc = (windows.first { $0.isKeyWindow } ?? windows.first)?.rootViewController
        while let p = vc?.presentedViewController { vc = p }
        return vc
    }

    private static func hasUsage(_ key: String) -> Bool {
        (Bundle.main.object(forInfoDictionaryKey: key) as? String)?.isEmpty == false
    }

    // MARK: - 选图 / 选视频

    private func pickMedia(_ a: [String: Any], _ result: @escaping FlutterResult) {
        let kind = a["kind"] as? String ?? "image"
        if a["source"] as? String == "camera" {
            camera(a, kind: kind, result)
            return
        }
        guard begin(a, result) else { return }
        if #available(iOS 14, *) {
            var conf = PHPickerConfiguration()
            conf.selectionLimit = count
            // .compatible：HEIC 照片给 JPEG、HEVC 视频给 H.264（引擎与大多数播放器都认）
            conf.preferredAssetRepresentationMode = .compatible
            switch kind {
            case "image": conf.filter = .images
            case "video": conf.filter = .videos
            default: conf.filter = .any(of: [.images, .videos])
            }
            let picker = PHPickerViewController(configuration: conf)
            picker.delegate = self
            present(picker)
        } else {
            let picker = UIImagePickerController()
            picker.sourceType = .photoLibrary
            picker.mediaTypes = Self.mediaTypes(kind)
            picker.delegate = self
            present(picker)
        }
    }

    private static func mediaTypes(_ kind: String) -> [String] {
        switch kind {
        case "image": return [kUTTypeImage as String]
        case "video": return [kUTTypeMovie as String]
        default: return [kUTTypeImage as String, kUTTypeMovie as String]
        }
    }

    private func camera(_ a: [String: Any], kind: String, _ result: @escaping FlutterResult) {
        let video = kind == "video"
        guard UIImagePickerController.isSourceTypeAvailable(.camera) else {
            result(Self.err("io", "这台设备没有相机"))
            return
        }
        guard Self.hasUsage("NSCameraUsageDescription"), !video || Self.hasUsage("NSMicrophoneUsageDescription") else {
            result(Self.err("denied", "Info.plist 缺 NSCameraUsageDescription\(video ? " / NSMicrophoneUsageDescription" : "")"))
            return
        }
        let open = { [weak self] in
            guard let self, self.begin(a, result) else { return }
            let picker = UIImagePickerController()
            picker.sourceType = .camera
            // mix 拍照片（与 Android 一致：一个系统相机界面没法「点拍照、按住录像」）
            picker.mediaTypes = [video ? kUTTypeMovie as String : kUTTypeImage as String]
            if video {
                picker.videoMaximumDuration = TimeInterval((a["maxDuration"] as? NSNumber)?.intValue ?? 60)
                picker.videoQuality = self.compressed ? .typeMedium : .typeHigh
            }
            if a["camera"] as? String == "front", UIImagePickerController.isCameraDeviceAvailable(.front) {
                picker.cameraDevice = .front
            }
            picker.delegate = self
            self.present(picker)
        }
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized: open()
        case .notDetermined:
            AVCaptureDevice.requestAccess(for: .video) { ok in
                DispatchQueue.main.async { ok ? open() : result(Self.err("denied", "camera")) }
            }
        default: result(Self.err("denied", "camera"))
        }
    }

    // MARK: - 选文件

    private func pickFiles(_ a: [String: Any], _ result: @escaping FlutterResult) {
        guard begin(a, result) else { return }
        fileType = a["type"] as? String ?? "all"
        exts = (a["extension"] as? [String] ?? []).map { $0.lowercased() }
        var types: [String]
        switch fileType {
        case "image": types = [kUTTypeImage as String]
        case "video": types = [kUTTypeMovie as String]
        default:
            types = fileType == "file" ? exts.compactMap(Self.uti(forExtension:)) : []
            if types.isEmpty { types = [kUTTypeItem as String] }
        }
        // .import：系统先把文件拷进本应用的沙盒再交过来（iOS 14 的 asCopy 要 UTType，见类注释）
        let picker = UIDocumentPickerViewController(documentTypes: types, in: .import)
        picker.allowsMultipleSelection = count > 1
        picker.delegate = self
        present(picker)
    }

    private static func uti(forExtension ext: String) -> String? {
        UTTypeCreatePreferredIdentifierForTag(kUTTagClassFilenameExtension, ext as CFString, nil)?.takeRetainedValue() as String?
    }

    // MARK: - 存相册 / 打开文档

    private func saveToAlbum(_ a: [String: Any], _ result: @escaping FlutterResult) {
        let path = a["path"] as? String ?? ""
        guard FileManager.default.fileExists(atPath: path) else {
            result(Self.err("not_found"))
            return
        }
        guard Self.hasUsage("NSPhotoLibraryAddUsageDescription") else {
            result(Self.err("denied", "Info.plist 缺 NSPhotoLibraryAddUsageDescription"))
            return
        }
        let url = URL(fileURLWithPath: path)
        let video = a["kind"] as? String == "video"
        let save = {
            PHPhotoLibrary.shared().performChanges({
                if video {
                    PHAssetChangeRequest.creationRequestForAssetFromVideo(atFileURL: url)
                } else {
                    PHAssetChangeRequest.creationRequestForAssetFromImage(atFileURL: url)
                }
            }) { ok, error in
                DispatchQueue.main.async { ok ? result(nil) : result(Self.err("io", error?.localizedDescription)) }
            }
        }
        let handle: (PHAuthorizationStatus) -> Void = { s in
            if s == .authorized || s.rawValue == 4 /* .limited（iOS 14） */ {
                save()
            } else {
                DispatchQueue.main.async { result(Self.err("denied", "photos")) }
            }
        }
        if #available(iOS 14, *) {
            PHPhotoLibrary.requestAuthorization(for: .addOnly, handler: handle)
        } else {
            PHPhotoLibrary.requestAuthorization(handle)
        }
    }

    private func openDocument(_ a: [String: Any], _ result: @escaping FlutterResult) {
        let path = a["path"] as? String ?? ""
        guard FileManager.default.fileExists(atPath: path) else {
            result(Self.err("not_found"))
            return
        }
        let doc = UIDocumentInteractionController(url: URL(fileURLWithPath: path))
        doc.delegate = self
        document = doc
        if !doc.presentPreview(animated: true) {
            // 系统预览不了的类型：给「用其它应用打开」的菜单
            guard let view = Self.topViewController()?.view,
                  doc.presentOpenInMenu(from: CGRect(x: view.bounds.midX, y: view.bounds.midY, width: 1, height: 1), in: view, animated: true)
            else {
                result(Self.err("io", "没有能打开这个文件的应用"))
                return
            }
        }
        result(nil)
    }

    // MARK: - 落盘

    static func newFile(in dir: URL, ext: String) -> URL {
        seq += 1
        let stamp = Int(Date().timeIntervalSince1970 * 1000)
        let e = ext.lowercased()
        let ok = !e.isEmpty && e.count <= 5 && e.allSatisfy { $0.isLetter || $0.isNumber }
        return dir.appendingPathComponent("host_\(stamp)_\(seq)" + (ok ? ".\(e)" : ""))
    }

    static func fileItem(_ url: URL, _ w: Int, _ h: Int, _ kind: String) -> [String: Any] {
        let size = (try? FileManager.default.attributesOfItem(atPath: url.path)[.size] as? NSNumber)??.intValue ?? 0
        return ["path": "file://" + url.path, "size": size, "width": w, "height": h, "fileType": kind]
    }

    /// 摆正（按 imageOrientation 重画）、按档位缩放后出 JPEG
    func writeImage(_ image: UIImage) throws -> [String: Any] {
        let maxSide: CGFloat = compressed ? 1920 : 4096
        var size = image.size
        let longest = max(size.width, size.height)
        if longest > maxSide { size = CGSize(width: size.width * maxSide / longest, height: size.height * maxSide / longest) }
        size = CGSize(width: size.width.rounded(), height: size.height.rounded())
        let format = UIGraphicsImageRendererFormat.default()
        format.scale = 1
        let upright = UIGraphicsImageRenderer(size: size, format: format).image { _ in image.draw(in: CGRect(origin: .zero, size: size)) }
        guard let data = upright.jpegData(compressionQuality: compressed ? 0.8 : 0.92) else { throw NSError(domain: "mini_render", code: 1) }
        let out = Self.newFile(in: dir, ext: "jpg")
        try data.write(to: out)
        return Self.fileItem(out, Int(size.width), Int(size.height), "image")
    }

    /// 图片数据：GIF / PNG / 本来就正的 JPEG 原样落盘，其余（HEIC、带方向的、要压缩的）重新编码
    func writeImageData(_ data: Data, uti: String) throws -> [String: Any] {
        guard let image = UIImage(data: data) else { throw NSError(domain: "mini_render", code: 2, userInfo: [NSLocalizedDescriptionKey: "解不开这张图"]) }
        let raw: String? = uti == "com.compuserve.gif" ? "gif"
            : (!compressed && image.imageOrientation == .up && uti == "public.png") ? "png"
            : (!compressed && image.imageOrientation == .up && uti == "public.jpeg") ? "jpg" : nil
        if let ext = raw {
            let out = Self.newFile(in: dir, ext: ext)
            try data.write(to: out)
            return Self.fileItem(out, Int(image.size.width * image.scale), Int(image.size.height * image.scale), "image")
        }
        return try writeImage(image)
    }

    /// 视频拷进 tempDir，读时长、显示尺寸（含旋转）与第一帧封面
    func writeVideo(from src: URL) throws -> [String: Any] {
        let out = Self.newFile(in: dir, ext: src.pathExtension.isEmpty ? "mp4" : src.pathExtension)
        try FileManager.default.copyItem(at: src, to: out)
        let asset = AVURLAsset(url: out)
        var w = 0, h = 0
        if let track = asset.tracks(withMediaType: .video).first {
            let s = track.naturalSize.applying(track.preferredTransform)
            w = Int(abs(s.width))
            h = Int(abs(s.height))
        }
        var item = Self.fileItem(out, w, h, "video")
        item["duration"] = CMTimeGetSeconds(asset.duration).isFinite ? CMTimeGetSeconds(asset.duration) : 0
        let gen = AVAssetImageGenerator(asset: asset)
        gen.appliesPreferredTrackTransform = true
        if let cg = try? gen.copyCGImage(at: .zero, actualTime: nil), let jpg = UIImage(cgImage: cg).jpegData(compressionQuality: 0.8) {
            let thumb = Self.newFile(in: dir, ext: "jpg")
            if (try? jpg.write(to: thumb)) != nil { item["thumb"] = "file://" + thumb.path }
        }
        return item
    }
}

// MARK: - PHPicker（iOS 14+）

@available(iOS 14, *)
extension MediaPicker: PHPickerViewControllerDelegate {
    func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
        picker.dismiss(animated: true)
        let picked = Array(results.prefix(count))
        if picked.isEmpty {
            finish(Self.err("cancel"))
            return
        }
        var items = [[String: Any]?](repeating: nil, count: picked.count)
        var failure: String?
        let group = DispatchGroup()
        let lock = NSLock()
        for (i, r) in picked.enumerated() {
            let p = r.itemProvider
            group.enter()
            let done: ([String: Any]?, Error?) -> Void = { item, e in
                lock.lock()
                items[i] = item
                if item == nil { failure = e?.localizedDescription ?? "读不出选中的文件" }
                lock.unlock()
                group.leave()
            }
            if p.hasItemConformingToTypeIdentifier(kUTTypeMovie as String) {
                // 临时文件只在回调里有效：回调里就拷走
                p.loadFileRepresentation(forTypeIdentifier: kUTTypeMovie as String) { url, e in
                    guard let url else { return done(nil, e) }
                    do { done(try self.writeVideo(from: url), nil) } catch { done(nil, error) }
                }
            } else {
                let uti = ["com.compuserve.gif", "public.png", "public.jpeg"].first { p.hasItemConformingToTypeIdentifier($0) } ?? (kUTTypeImage as String)
                p.loadDataRepresentation(forTypeIdentifier: uti) { data, e in
                    guard let data else { return done(nil, e) }
                    do { done(try self.writeImageData(data, uti: uti), nil) } catch { done(nil, error) }
                }
            }
        }
        group.notify(queue: .main) {
            let ok = items.compactMap { $0 }
            self.finish(ok.isEmpty ? Self.err("io", failure) : ok)
        }
    }
}

// MARK: - 相机 / iOS 13 相册

extension MediaPicker: UIImagePickerControllerDelegate, UINavigationControllerDelegate {
    func imagePickerControllerDidCancel(_ picker: UIImagePickerController) {
        picker.dismiss(animated: true)
        finish(Self.err("cancel"))
    }

    func imagePickerController(_ picker: UIImagePickerController, didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey: Any]) {
        picker.dismiss(animated: true)
        let movie = info[.mediaURL] as? URL
        let image = info[.originalImage] as? UIImage
        DispatchQueue.global(qos: .userInitiated).async {
            do {
                if let movie {
                    self.finish([try self.writeVideo(from: movie)])
                } else if let image {
                    self.finish([try self.writeImage(image)])
                } else {
                    self.finish(Self.err("cancel"))
                }
            } catch {
                self.finish(Self.err("io", error.localizedDescription))
            }
        }
    }
}

// MARK: - 文件选择器

extension MediaPicker: UIDocumentPickerDelegate {
    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
        finish(Self.err("cancel"))
    }

    func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
        let type = fileType, exts = self.exts
        DispatchQueue.global(qos: .userInitiated).async {
            var out: [[String: Any]] = []
            for url in urls {
                if out.count >= self.count { break }
                let ext = url.pathExtension.lowercased()
                let uti = Self.uti(forExtension: ext) ?? ""
                let kind = UTTypeConformsTo(uti as CFString, kUTTypeImage) ? "image" : UTTypeConformsTo(uti as CFString, kUTTypeMovie) ? "video" : "file"
                // file = 图片与视频以外的文件（官方语义）；extension 只在 type 为 file 时有效
                let keep = type == "all" || (type == "file" ? kind == "file" && (exts.isEmpty || exts.contains(ext)) : kind == type)
                guard keep else { continue }
                let dst = Self.newFile(in: self.dir, ext: ext)
                guard (try? FileManager.default.copyItem(at: url, to: dst)) != nil else { continue }
                var item = Self.fileItem(dst, 0, 0, kind)
                item["name"] = url.lastPathComponent
                item["time"] = Int(Date().timeIntervalSince1970)
                out.append(item)
            }
            self.finish(out.isEmpty ? Self.err("cancel") : out)
        }
    }
}

extension MediaPicker: UIDocumentInteractionControllerDelegate {
    func documentInteractionControllerViewControllerForPreview(_ controller: UIDocumentInteractionController) -> UIViewController {
        Self.topViewController() ?? UIViewController()
    }

    func documentInteractionControllerDidEndPreview(_ controller: UIDocumentInteractionController) {
        document = nil
    }
}
