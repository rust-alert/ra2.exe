# ra-components

UI **组件语义与画面组合**骨架：视觉组件、语义组合、`screen/` 画面装配。

- 使用 `ra-layout` 求解矩形，产出绘制 / 命中 / 焦点 / 事件描述。
- **不**依赖 winit / web-sys / DOM；**不**直调 wgpu；**不**直读对局权威状态。
- `screen/` 只是组合器，不是独立 UI 框架 crate。

布局求解见 `ra-layout`。
