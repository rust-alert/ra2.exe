//! VXL 简易等距正交投影（预览用，无光照）。

use crate::{HvaFile, Palette, VplFile, VxlFile};

/// 投影后的精灵。
#[derive(Debug, Clone)]
pub struct VxlSprite {
    /// 像素宽。
    pub width: u32,
    /// 像素高。
    pub height: u32,
    /// 相对投影原点：包围盒中心对齐 (0,0) 的 X。
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
/// 组装后绕模型原点做 8 向偏航。
pub fn rasterize_vxl_frame(
    vxl: &VxlFile,
    palette: &Palette,
    hva: Option<&HvaFile>,
    facing: u8,
    frame: u32,
) -> Option<VxlSprite> {
    rasterize_vxl_layers(&[(vxl, hva)], palette, facing, frame)
}

/// 一层 VXL 的姿态（炮塔可与车身不同 facing）。
#[derive(Debug, Clone, Copy)]
pub struct VxlLayerPose<'a> {
    /// 本层体素模型。
    pub vxl: &'a VxlFile,
    /// 可选 HVA 动画。
    pub hva: Option<&'a HvaFile>,
    /// 8 向朝向（0..=255，每 32 一步）。
    pub facing: u8,
    /// HVA 帧下标。
    pub frame: u32,
}

/// 多层 VXL（车身 / 炮塔 / 炮管）合成一张精灵；各层共用 facing/frame。
pub fn rasterize_vxl_layers(
    layers: &[(&VxlFile, Option<&HvaFile>)],
    palette: &Palette,
    facing: u8,
    frame: u32,
) -> Option<VxlSprite> {
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
                let (x, y, z) = yaw_point(mx, my, mz, 0.0, 0.0, layer.facing);
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
        let xi = x.round() as i32;
        let yi = y.round() as i32;
        let zi = z.round() as i32;
        let sx = xi - yi;
        let sy = (xi + yi) / 2 - zi;
        let depth = xi + yi + zi;
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

    Some(VxlSprite { width, height, offset_x: -(width as i32) / 2, offset_y: -(height as i32) / 2, rgba })
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
    (
        m[0] * x + m[1] * y + m[2] * z + m[3] * s,
        m[4] * x + m[5] * y + m[6] * z + m[7] * s,
        m[8] * x + m[9] * y + m[10] * z + m[11] * s,
    )
}

fn yaw_point(x: f32, y: f32, z: f32, cx: f32, cy: f32, facing: u8) -> (f32, f32, f32) {
    let steps = facing / 32;
    if steps == 0 {
        return (x, y, z);
    }
    let angle = f32::from(steps) * std::f32::consts::FRAC_PI_4;
    let (s, c) = angle.sin_cos();
    let dx = x - cx;
    let dy = y - cy;
    (c * dx - s * dy + cx, s * dx + c * dy + cy, z)
}
