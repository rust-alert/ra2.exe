//! 原生 GUI 示例入口：等价于产品侧 `ra2 launch`（需本机已配置或默认目录）。

fn main() {
    if let Err(e) = ra_desktop::run() {
        eprintln!("ra-desktop 错误: {e}");
        std::process::exit(1);
    }
}
