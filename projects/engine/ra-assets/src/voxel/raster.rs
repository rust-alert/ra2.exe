//! VXL 简易等距投影（预览用；车身 VPL 为法线→亮度页粗映射）。
//!
//! 朝向与相机约定对齐经典等距体素绘制：32 档偏航（四舍五入量化）、车身绕 Z
//! 的 `(step - 8) * -π/16`，再乘俯仰 −60° / 偏航 −45° 的视图，屏幕取 `(x, -y)`。
//! 精灵锚在模型原点，避免按不透明包围盒居中导致转向／HVA 走动时整车乱跳。

use std::collections::HashSet;

use super::{hva::HvaFile, vpl::VplFile, vxl::VxlFile};
use crate::image::pal::Palette;

/// 落影相对底面投影的屏幕 X 光向偏移（像素）。
pub const VXL_SHADOW_LIGHT_OFFSET_X: i32 = 3;

/// 体素偏航量化档数（每轴 32 档；本预览路径只做 yaw + 固定等距相机）。
///
/// 勿与载具 **SHP** 的 8 向帧槽混淆。
pub const VXL_FACING_STEPS: u32 = 32;

/// 历史导出名：一档对应的朝向字节跨度约 `256/32=8`（实际量化见 [`vxl_yaw_steps`] 的四舍五入）。
pub const VXL_FACING_BYTE_STEP: u8 = (256 / VXL_FACING_STEPS) as u8;

/// 相机俯仰（弧度）：−60°。
const CAMERA_PITCH: f32 = -std::f32::consts::FRAC_PI_3;

/// 相机偏航（弧度）：−45°，使模型北与等距格对齐。
const CAMERA_YAW: f32 = -std::f32::consts::FRAC_PI_4;

/// 将游戏朝向字节量化为偏航档位（0..=31）。
///
/// 对半档四舍五入：`((facing >> 2) + 1) >> 1`，边界在 facing=4（半个 11.25°）。
#[inline]
pub fn vxl_yaw_steps(facing: u8) -> u8 {
    ((((u32::from(facing) >> 2) + 1) >> 1) & (VXL_FACING_STEPS - 1)) as u8
}

/// 车身绕模型 Z 的偏航角（弧度）：`(step - 8) * -π/16`。
///
/// `step=8`（facing≈64）为零转；与相机 −45° 合起来对应经典「北」对齐。
#[inline]
pub fn vxl_yaw_radians(facing: u8) -> f32 {
    (f32::from(vxl_yaw_steps(facing)) - 8.0) * -(std::f32::consts::PI / 16.0)
}

/// 投影后的精灵。
#[derive(Debug, Clone)]
pub struct VxlSprite {
    /// 像素宽。
    pub width: u32,
    /// 像素高。
    pub height: u32,
    /// 相对投影原点：模型原点映射到精灵内的 X（非包围盒居中）。
    pub offset_x: i32,
    /// 相对投影原点的 Y。
    pub offset_y: i32,
    /// 行优先 RGBA。
    pub rgba: Vec<u8>,
}

/// 无姿态投影（等价于 facing=0、frame=0、无 HVA）。
pub fn rasterize_vxl(vxl: &VxlFile, palette: &Palette) -> Option<VxlSprite> {
    rasterize_vxl_frame(vxl, palette, None, 0, 0)
}

/// 按朝向投影（HVA 第 0 帧）。
pub fn rasterize_vxl_posed(vxl: &VxlFile, palette: &Palette, hva: Option<&HvaFile>, facing: u8) -> Option<VxlSprite> {
    rasterize_vxl_frame(vxl, palette, hva, facing, 0)
}

/// 按朝向与 HVA 动画帧投影。
///
/// 节变换：`bounds_min + bone(scale(grid))`，其中 bone 平移乘 `limb.scale`。
/// 组装后按 [`VXL_FACING_STEPS`] 档车身偏航，再经固定等距相机投影。
pub fn rasterize_vxl_frame(vxl: &VxlFile, palette: &Palette, hva: Option<&HvaFile>, facing: u8, frame: u32) -> Option<VxlSprite> {
    rasterize_vxl_layers(&[(vxl, hva)], palette, facing, frame)
}

/// 一层 VXL 的姿态（炮塔可与车身不同 facing）。
#[derive(Debug, Clone, Copy)]
pub struct VxlLayerPose<'a> {
    /// 本层体素模型。
    pub vxl: &'a VxlFile,
    /// 可选 HVA 动画。
    pub hva: Option<&'a HvaFile>,
    /// 游戏朝向字节（0..=255）；光栅前量化为 [`VXL_FACING_STEPS`] 档偏航。
    pub facing: u8,
    /// HVA 帧下标。
    pub frame: u32,
}

/// 多层 VXL（车身 / 炮塔 / 炮管）合成一张精灵；各层共用 facing/frame。
pub fn rasterize_vxl_layers(layers: &[(&VxlFile, Option<&HvaFile>)], palette: &Palette, facing: u8, frame: u32) -> Option<VxlSprite> {
    let poses: Vec<VxlLayerPose<'_>> = layers.iter().map(|&(vxl, hva)| VxlLayerPose { vxl, hva, facing, frame }).collect();
    rasterize_vxl_layer_poses(&poses, palette, None)
}

/// 多层合成；每层可有独立 facing / HVA 帧。可选 VPL 按法线粗映射亮度页。
pub fn rasterize_vxl_layer_poses(layers: &[VxlLayerPose<'_>], palette: &Palette, vpl: Option<&VplFile>) -> Option<VxlSprite> {
    let mut assembled: Vec<(f32, f32, f32, u8, u8)> = Vec::new();
    for layer in layers {
        let frame_idx = match layer.hva {
            Some(h) if h.frame_count > 0 => layer.frame % h.frame_count,
            _ => 0,
        };
        for (section, limb) in layer.vxl.limbs.iter().enumerate() {
            let bone = layer.hva.and_then(|h| h.get_transform(frame_idx, section as u32)).unwrap_or(&limb.transform);
            for v in &limb.voxels {
                let (mx, my, mz) = section_point(limb, v, bone);
                let (x, y, z) = camera_point(mx, my, mz, layer.facing);
                assembled.push((x, y, z, v.color_index, v.normal_index));
            }
        }
    }
    if assembled.is_empty() {
        return None;
    }

    let mut points: Vec<(i32, i32, i32, u8)> = Vec::with_capacity(assembled.len());
    for (x, y, z, color_index, normal_index) in assembled {
        let shaded = match vpl {
            Some(table) => {
                let page = table.page_from_normal(normal_index);
                table.remap_color(page, color_index)
            }
            None => color_index,
        };
        let (sx, sy, depth) = project_screen(x, y, z);
        points.push((sx, sy, depth, shaded));
    }

    let mut min_sx = i32::MAX;
    let mut min_sy = i32::MAX;
    let mut max_sx = i32::MIN;
    let mut max_sy = i32::MIN;
    for &(sx, sy, _, _) in &points {
        min_sx = min_sx.min(sx);
        min_sy = min_sy.min(sy);
        max_sx = max_sx.max(sx);
        max_sy = max_sy.max(sy);
    }

    let width = (max_sx - min_sx + 1).clamp(1, 512) as u32;
    let height = (max_sy - min_sy + 1).clamp(1, 512) as u32;
    let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];

    points.sort_by_key(|&(sx, sy, depth, _)| (depth, sy, sx));
    for (sx, sy, _, color_index) in points {
        let c = palette.colors[color_index as usize];
        if c.a == 0 {
            continue;
        }
        let px = sx - min_sx;
        let py = sy - min_sy;
        if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
            continue;
        }
        let di = ((py as u32 * width + px as u32) * 4) as usize;
        rgba[di] = c.r;
        rgba[di + 1] = c.g;
        rgba[di + 2] = c.b;
        rgba[di + 3] = c.a;
    }

    // 精灵 (0,0) 对应投影 (min_sx,min_sy)；offset=min 使模型原点落到叠画锚点（再加钻石中心）。
    // 勿用 -width/2 包围盒居中，否则转向／HVA 走动时整车会跟着 AABB 漂移。
    Some(VxlSprite { width, height, offset_x: min_sx, offset_y: min_sy, rgba })
}

/// 体素落影：各占用柱压到模型最低高度后做等距投影，再加光向偏移。
///
/// 返回精灵的不透明黑像素为落影模板；叠画端应对目标像素压暗，而不是源覆盖。
/// 炮塔 / 炮管层通常不参与；调用方只传入车身层即可。
pub fn rasterize_vxl_shadow_layer_poses(layers: &[VxlLayerPose<'_>]) -> Option<VxlSprite> {
    // 模型空间点 + 各层朝向；落影按模型 XY 柱去重，再压到最低 Z 后走同一相机。
    let mut world: Vec<(f32, f32, f32, u8)> = Vec::new();
    for layer in layers {
        let frame_idx = match layer.hva {
            Some(h) if h.frame_count > 0 => layer.frame % h.frame_count,
            _ => 0,
        };
        for (section, limb) in layer.vxl.limbs.iter().enumerate() {
            let bone = layer.hva.and_then(|h| h.get_transform(frame_idx, section as u32)).unwrap_or(&limb.transform);
            for v in &limb.voxels {
                let (mx, my, mz) = section_point(limb, v, bone);
                world.push((mx, my, mz, layer.facing));
            }
        }
    }
    if world.is_empty() {
        return None;
    }

    let ground_z = world.iter().map(|(_, _, z, _)| *z).fold(f32::INFINITY, f32::min);
    let mut columns = HashSet::new();
    let mut points: Vec<(i32, i32)> = Vec::new();
    for &(x, y, _, facing) in &world {
        let xi = x.round() as i32;
        let yi = y.round() as i32;
        if !columns.insert((xi, yi)) {
            continue;
        }
        let (cx, cy, cz) = camera_point(xi as f32, yi as f32, ground_z, facing);
        let (sx, sy, _) = project_screen(cx, cy, cz);
        points.push((sx + VXL_SHADOW_LIGHT_OFFSET_X, sy));
    }
    if points.is_empty() {
        return None;
    }

    let mut min_sx = i32::MAX;
    let mut min_sy = i32::MAX;
    let mut max_sx = i32::MIN;
    let mut max_sy = i32::MIN;
    for &(sx, sy) in &points {
        min_sx = min_sx.min(sx);
        min_sy = min_sy.min(sy);
        max_sx = max_sx.max(sx);
        max_sy = max_sy.max(sy);
    }

    let width = (max_sx - min_sx + 1).clamp(1, 512) as u32;
    let height = (max_sy - min_sy + 1).clamp(1, 512) as u32;
    let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];
    for (sx, sy) in points {
        let px = sx - min_sx;
        let py = sy - min_sy;
        if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
            continue;
        }
        let di = ((py as u32 * width + px as u32) * 4) as usize;
        rgba[di] = 0;
        rgba[di + 1] = 0;
        rgba[di + 2] = 0;
        rgba[di + 3] = 255;
    }

    Some(VxlSprite { width, height, offset_x: min_sx, offset_y: min_sy, rgba })
}

/// 节局部点：`bounds_min + bone(section_scale * grid)`。
fn section_point(limb: &crate::VxlLimb, v: &crate::VxlVoxel, bone: &[f32; 12]) -> (f32, f32, f32) {
    let sx = section_axis_scale(limb.bounds[3] - limb.bounds[0], limb.size_x);
    let sy = section_axis_scale(limb.bounds[4] - limb.bounds[1], limb.size_y);
    let sz = section_axis_scale(limb.bounds[5] - limb.bounds[2], limb.size_z);
    let px = f32::from(v.x) * sx;
    let py = f32::from(v.y) * sy;
    let pz = f32::from(v.z) * sz;
    let (bx, by, bz) = apply_matrix_scaled(bone, px, py, pz, limb.scale);
    (bx + limb.bounds[0], by + limb.bounds[1], bz + limb.bounds[2])
}

fn section_axis_scale(extent: f32, size: u8) -> f32 {
    if size == 0 { 1.0 } else { extent / f32::from(size) }
}

/// HVA 3×4：旋转作用在坐标上，平移乘肢节 `scale`。
fn apply_matrix_scaled(m: &[f32; 12], x: f32, y: f32, z: f32, limb_scale: f32) -> (f32, f32, f32) {
    let s = if limb_scale.is_finite() && limb_scale > 0.0 { limb_scale } else { 1.0 };
    (m[0] * x + m[1] * y + m[2] * z + m[3] * s, m[4] * x + m[5] * y + m[6] * z + m[7] * s, m[8] * x + m[9] * y + m[10] * z + m[11] * s)
}

/// 车身偏航后再乘固定等距相机（先 Rz(camera_yaw)，再 Rx(camera_pitch)）。
fn camera_point(x: f32, y: f32, z: f32, facing: u8) -> (f32, f32, f32) {
    let (x, y, z) = rotate_z(x, y, z, vxl_yaw_radians(facing));
    let (x, y, z) = rotate_z(x, y, z, CAMERA_YAW);
    rotate_x(x, y, z, CAMERA_PITCH)
}

fn project_screen(x: f32, y: f32, z: f32) -> (i32, i32, i32) {
    (x.round() as i32, (-y).round() as i32, z.round() as i32)
}

fn rotate_z(x: f32, y: f32, z: f32, angle: f32) -> (f32, f32, f32) {
    if angle == 0.0 {
        return (x, y, z);
    }
    let (s, c) = angle.sin_cos();
    (c * x - s * y, s * x + c * y, z)
}

fn rotate_x(x: f32, y: f32, z: f32, angle: f32) -> (f32, f32, f32) {
    if angle == 0.0 {
        return (x, y, z);
    }
    let (s, c) = angle.sin_cos();
    (x, c * y - s * z, s * y + c * z)
}
