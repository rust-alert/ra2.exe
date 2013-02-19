# ra-renderer

crate 头注释写得很硬：

> 读取世界状态，经现代 GPU（wgpu）绘制。  
> 原生后端：DX12 / Vulkan / Metal。Wasm：WebGL2。  
> 本 crate **故意不**实现 DirectDraw。

产品定位是现代化重写，不是 ddraw 兼容层。依赖：`ra-types`、`ra-world`、`wgpu`（工作区锁定 24）、`winit`、`pollster`、`bytemuck`。

## 三个模块，不是「场景图」

```
src/lib.rs         Renderer 状态机 + 清屏色
src/gpu.rs         GpuContext：适配器/设备/表面
src/rgba_image.rs  CPU 侧 RGBA 缓冲校验
src/sprite.rs      单张纹理四边形 + 内嵌 WGSL
```

今天能画的东西： **深色清屏 + 可选一张居中放大的预览精灵**。没有地形批次，没有单位排序，没有 UI 图集。

---

## 清屏色从哪来

```rust
const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.04,
    g: 0.06,
    b: 0.09,
    a: 1.0,
};
```

注释：「接近夜间战术图感觉，非最终主题。」改主题先改这个常量，别在桌面里清屏。

---

## `Renderer` 生命周期（按调用顺序读）

| 方法                         | 行为                                                              |
|------------------------------|-------------------------------------------------------------------|
| `new()`                      | `frames=0`，无 GPU，无 preview                                    |
| `set_preview(RgbaImage)`     | 若已有 GPU，立即建/换 `SpriteGpu`；否则只存 CPU 图，等附着        |
| `attach_window(Arc<Window>)` | 已绑定则 Ok 忽略；否则 `GpuContext::new`，若有 preview 则上传精灵 |
| `resize(w,h)`                | 转给 GPU 表面配置，宽高至少 1                                     |
| `draw_frame(Option<&World>)` | 无 GPU 直接 return；清屏；有 sprite 则画；`frames` wrapping_add   |
| `backend_name()`             | 来自 `GpuContext` 标签                                            |
| `has_preview()`              | 是否持有 preview 图                                               |
| `backend_hint(_) -> "wgpu"`  | 固定字符串，给诊断用                                              |

桌面在构造 `App` 时可能先 `set_preview`（启动解码出的 SHP），再在 `resumed` 里 `attach_window`。顺序反了也能工作：附着时会补建
sprite。

---

## `GpuContext`（`gpu.rs`）要点

- Backends：`PRIMARY`
- 电源偏好：`HighPerformance`
- 表面格式优先 sRGB；`PresentMode::Fifo`；`desired_maximum_frame_latency: 2`
- 设备标签一类字符串带 `ra.` 前缀
- `backend_label` 映射到 `wgpu/dx12`、`wgpu/vulkan`、`wgpu/metal`、`wgpu/gl`、`wgpu/webgpu` 或兜底 `"wgpu"`
- 初始化用 `pollster::block_on`——这是 **原生阻塞**路径。注释里的 WebGL2 是方向声明；本模块尚未分出
  `cfg(target_arch = "wasm32")` 的异步创建分支。`ra-webui` 也还没调用 `attach_window`。

---

## 精灵怎么画（`sprite.rs`）

- 采样：最近邻（像素风）
- 混合：标准 alpha
- 纹理：`Rgba8UnormSrgb`
- **显示缩放 `scale = 4.0`**——鼠标/步兵预览会被放大四倍
- 顶点按表面尺寸居中；初始占位按 1024×768 想；每帧 `write_vertices` 更新
- WGSL 入口：`vs_main` / `fs_main`，`textureSample`
- GPU 对象标签：`ra.sprite.*`

`RgbaImage::new` 校验 `pixels.len() == width*height*4` 且尺寸非零，失败则桌面跳过该预览候选。

---

## 和 `World` 的真实耦合度

`draw_frame` 签名吃世界引用，方便以后按 tick/edition 换内容。现状绘制路径几乎不读世界字段。标题栏上的 tick 是桌面自己读
`world.tick` 写的，不是渲染器画的文字。

## 构建

```shell
cargo build -p ra-renderer
```

无单元测试（GPU 测试需要适配器，CI 未接）。许可 MPL-2.0。

## 别在这里做的事

- 实现 DirectDraw / 向原版 exe 注入
- 在本 crate 读 MIX
- 把整份地图 CPU 软件光栅进一张大图再当「最终渲染架构」（预览可以，主路径应走 GPU 批次）

要加地形绘制，合理入口是新模块（例如 `terrain.rs`）消费 TMP/单元格，而不是继续把所有东西塞进单张 `SpriteGpu`。
