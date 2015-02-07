//! 安装探测与对局前装载（浏览器侧，对标 `ra-napi` host boot）。

/// 当前是否具备一次可尝试的装载输入（已导入安装文件）。
pub fn has_install_input() -> bool {
    false
}
