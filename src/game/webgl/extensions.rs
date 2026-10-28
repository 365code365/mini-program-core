//! WebGL 扩展名 → 底层 GL 扩展（与 Chromium `WebGLRenderingContextBase` 的对应一致）。
//!
//! `gl` 里**任一**存在即可用（同一能力在不同驱动上名字不同，比如深度纹理）；`core_in_es3` 的在 ES3 上下文里是核心功能，
//! 不看扩展串（系统 GLES 驱动常不再列 ES2 时代的扩展名）。

pub struct Ext {
    pub name: &'static str,
    /// 0 = WebGL1/2 都有，1 = 只 WebGL1，2 = 只 WebGL2
    pub version: u8,
    pub gl: &'static [&'static str],
    /// 不看 GL 扩展、永远可用（由引擎实现，或 ES3 起是核心）
    pub always: bool,
    pub core_in_es3: bool,
}

const fn e(name: &'static str, version: u8, gl: &'static [&'static str]) -> Ext {
    Ext { name, version, gl, always: false, core_in_es3: false }
}
const fn core(name: &'static str, version: u8, gl: &'static [&'static str]) -> Ext {
    Ext { name, version, gl, always: false, core_in_es3: true }
}
const fn own(name: &'static str, version: u8) -> Ext {
    Ext { name, version, gl: &[], always: true, core_in_es3: false }
}

pub static TABLE: &[Ext] = &[
    core("ANGLE_instanced_arrays", 1, &["GL_ANGLE_instanced_arrays", "GL_EXT_instanced_arrays"]),
    core("EXT_blend_minmax", 1, &["GL_EXT_blend_minmax"]),
    e("EXT_color_buffer_half_float", 0, &["GL_EXT_color_buffer_half_float"]),
    e("EXT_color_buffer_float", 2, &["GL_EXT_color_buffer_float"]),
    e("EXT_float_blend", 0, &["GL_EXT_float_blend"]),
    core("EXT_frag_depth", 1, &["GL_EXT_frag_depth"]),
    core("EXT_shader_texture_lod", 1, &["GL_EXT_shader_texture_lod"]),
    core("EXT_sRGB", 1, &["GL_EXT_sRGB"]),
    e("EXT_texture_compression_bptc", 0, &["GL_EXT_texture_compression_bptc"]),
    e("EXT_texture_compression_rgtc", 0, &["GL_EXT_texture_compression_rgtc"]),
    e("EXT_texture_filter_anisotropic", 0, &["GL_EXT_texture_filter_anisotropic"]),
    e("EXT_disjoint_timer_query", 1, &["GL_EXT_disjoint_timer_query"]),
    e("EXT_disjoint_timer_query_webgl2", 2, &["GL_EXT_disjoint_timer_query"]),
    e("KHR_parallel_shader_compile", 0, &["GL_KHR_parallel_shader_compile"]),
    core("OES_element_index_uint", 1, &["GL_OES_element_index_uint"]),
    e("OES_fbo_render_mipmap", 1, &["GL_OES_fbo_render_mipmap"]),
    core("OES_standard_derivatives", 1, &["GL_OES_standard_derivatives"]),
    core("OES_texture_float", 1, &["GL_OES_texture_float"]),
    e("OES_texture_float_linear", 0, &["GL_OES_texture_float_linear"]),
    core("OES_texture_half_float", 1, &["GL_OES_texture_half_float"]),
    core("OES_texture_half_float_linear", 1, &["GL_OES_texture_half_float_linear"]),
    core("OES_vertex_array_object", 1, &["GL_OES_vertex_array_object"]),
    e("WEBGL_color_buffer_float", 1, &["GL_CHROMIUM_color_buffer_float_rgba", "GL_EXT_color_buffer_float"]),
    e("WEBGL_compressed_texture_astc", 0, &["GL_KHR_texture_compression_astc_ldr"]),
    e("WEBGL_compressed_texture_etc", 0, &["GL_ANGLE_compressed_texture_etc"]),
    e("WEBGL_compressed_texture_etc1", 0, &["GL_OES_compressed_ETC1_RGB8_texture"]),
    e("WEBGL_compressed_texture_pvrtc", 0, &["GL_IMG_texture_compression_pvrtc"]),
    e("WEBGL_compressed_texture_s3tc", 0, &["GL_EXT_texture_compression_dxt1", "GL_ANGLE_texture_compression_dxt3", "GL_ANGLE_texture_compression_dxt5", "GL_EXT_texture_compression_s3tc"]),
    e("WEBGL_compressed_texture_s3tc_srgb", 0, &["GL_EXT_texture_compression_s3tc_srgb"]),
    own("WEBGL_debug_renderer_info", 0),
    own("WEBGL_debug_shaders", 0),
    core("WEBGL_depth_texture", 1, &["GL_ANGLE_depth_texture", "GL_OES_depth_texture"]),
    core("WEBGL_draw_buffers", 1, &["GL_EXT_draw_buffers"]),
    own("WEBGL_lose_context", 0),
    e("WEBGL_multi_draw", 0, &["GL_ANGLE_multi_draw"]),
    e("OES_draw_buffers_indexed", 2, &["GL_OES_draw_buffers_indexed", "GL_EXT_draw_buffers_indexed"]),
];

/// ES3 上下文里的 WebGL1 核心项（系统驱动不列扩展名时）也要满足 WebGL 那边的语义：
/// 只有 ANGLE 之外、而且上下文是 ES3 时才按 core_in_es3 放行。WebGL1 的 ANGLE 上下文是 ES2，必须真有扩展
pub fn available(e: &Ext, have: &[String], version: u8, webgl_compat: bool) -> bool {
    if e.always {
        return true;
    }
    if e.gl.iter().any(|g| have.iter().any(|h| h == g)) {
        return true;
    }
    e.core_in_es3 && !webgl_compat && version == 1 && have.iter().any(|h| h == "GL_ES3")
}
