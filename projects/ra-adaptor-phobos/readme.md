# ra-adaptor-phobos

Phobos 扩展能力适配。心灵终结 3（Mental Omega 3）等内容布局**归本 crate 承载**，不再使用独立的 `ra-adaptor-mo3`。

依赖仅 `ra-types`。

## 提供

| 符号 | 作用 |
|------|------|
| `mo_layout_profile` / `profile` | MO 安装资源表（YR 基座 + expandmo* 等） |
| `looks_like_phobos` | Phobos DLL 痕迹 |
| `looks_like_mo_layout` | MO 内容布局痕迹 |
| `looks_like` | 上述任一命中 |

配置仍可写 `edition = "mo3"`：表示 YR 基座 + Phobos 系 MO 布局快捷方式，不是第三套互斥「版本轴」。
