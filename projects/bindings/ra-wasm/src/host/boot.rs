//! 安装探测与对局前装载（浏览器侧，对标 `ra-napi` host boot）。
//!
//! 安装字节入口见 [`super::install`]；对局会话打开后续再接。

/// 当前是否具备一次可尝试的装载输入（已导入安装文件）。
pub fn has_install_input(file_count: usize) -> bool {
    file_count > 0
}

/// 是否已具备可继续 boot 的挂载结果（根包已挂上）。
pub fn can_boot(mounted_root: u32, missing_base: usize) -> bool {
    mounted_root > 0 && missing_base == 0
}
