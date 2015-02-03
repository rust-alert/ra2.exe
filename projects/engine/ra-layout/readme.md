# ra-layout

UI **空间求解**骨架：viewport / DPI / 约束 / 锚点 / 栅格 → `LayoutSnapshot`。

- 吸收原版低分辨率矩形作**参考输入**，不以之为世界坐标。
- **不知**页面名、SHP/PCX、按钮绘制、对局状态。
- 依赖：`ra-types`。

组件语义与画面组合见 `ra-widgets`。
