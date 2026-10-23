package dev.minirender

import org.json.JSONObject

/**
 * 小程序调了一个**宿主实现**的 `wx.*`（[MiniProgramView.hostApis] 里登记的名字，或 SDK 自带的选图 / 选文件那几个）。
 *
 * 参数已由引擎按官方默认值补齐（`host-media.js` 那一组还先问过「拍照 / 从手机相册选择」）。
 * 回包用 [MiniHostReply]：成功给 `res`（交给页面的 success 回调，`errMsg` 不用写，引擎补 `<api>:ok`），
 * 失败给 `errMsg`（`<api>:fail 原因`；用户取消照微信写 `<api>:fail cancel`）。
 *
 * ## 文件
 * - 交给小程序的文件：写进 [tempDir]（引擎只换名字、不拷贝），回包里写 `"file://" + 绝对路径`，
 *   引擎换成 `wxfile://tmp/…`；别处的文件也可以，引擎会拷一份进沙盒。
 * - 小程序交过来的文件（`saveImageToPhotosAlbum({filePath})` 这类）：真实路径查 [files]。
 */
class MiniHostCall internal constructor(
    /** 回包时引擎认这个 */
    val id: Int,
    /** `wx` 上的名字，如 `login` */
    val api: String,
    /** 页面传的参数（回调函数已去掉） */
    val args: JSONObject,
    /** 宿主可以直接写的临时目录（`<dataDir>/wxfile/<小程序目录名>/tmp`） */
    val tempDir: String,
    /** [args] 里出现的 `wxfile://…` → 真实路径（文件不存在的不列） */
    val files: Map<String, String>,
) {
    internal companion object {
        fun fromJson(o: JSONObject): MiniHostCall {
            val f = o.optJSONObject("files")
            val files = f?.keys()?.asSequence()?.associateWith { f.optString(it) }.orEmpty()
            return MiniHostCall(
                o.optInt("id"),
                o.optString("api"),
                o.optJSONObject("args") ?: JSONObject(),
                o.optString("tempDir"),
                files,
            )
        }
    }
}

/** hostCall 的回包。**只认第一次**（之后的调用忽略），哪条线程都可以调 */
interface MiniHostReply {
    fun success(result: JSONObject = JSONObject())

    /** `errMsg` 写全：`<api>:fail 原因`。[extra] 合进回包（如 `startSoterAuthentication` 的 `errCode`） */
    fun fail(errMsg: String, extra: JSONObject? = null)
}

/** 宿主实现的一个 `wx.*` API（主线程上调；耗时的活自己挪到后台，做完再 [MiniHostReply] 回） */
fun interface MiniHostApi {
    fun handle(call: MiniHostCall, reply: MiniHostReply)
}
