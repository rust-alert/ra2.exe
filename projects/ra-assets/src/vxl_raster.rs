//! VXL 简易等距正交投影（预览用，无光照）。

use crate::{HvaFile, Palette, VxlFile};

/// 投影后的精灵。
#[derive(Debug, Clone)]
pub struct VxlSprite {
    pub width: u32,
    pub height: u32,
    /// 相对投影原点：包围盒中心对齐 (0,0)。
    pub offset_x: i32,
    pub offset_y: i32,
    pub rgba: Vec<u8>,
}

/// 无姿态投影（等价于 facing=0、frame=0、无 HVA）。
pub fn rasterize_vxl(vxl: &VxlFile, palette: &Palette) -> Option<VxlSprite> {
    rasterize_vxl_frame(vxl, palette, None, 0, 0)
}

/// 按朝向投影（HVA 第 0 帧）。
pub fn rasterize_vxl_posed(
    vxl: &VxlFile,
    palette: &Palette,
    hva: Option<&HvaFile>,
    facing: u8,
) -> Option<VxlSprite> {
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
    pub vxl: &'a VxlFile,
    pub hva: Option<&'a HvaFile>,
    pub facing: u8,
    pub frame: u32,
}

/// 多层 VXL（车身 / 炮塔 / 炮管）合成一张精灵；各层共用 facing/frame。
pub fn rasterize_vxl_layers(
    layers: &[(&VxlFile, Option<&HvaFile>)],
    palette: &Palette,
    facing: u8,
    frame: u32,
) -> Option<VxlSprite> {
    let poses: Vec<VxlLayerPose<'_>> = layers
        .iter()
        .map(|&(vxl, hva)| VxlLayerPose {
            vxl,
            hva,
            facing,
            frame,
        })
        .collect();
    rasterize_vxl_layer_poses(&poses, palette)
}

/// 多层合成；每层可有独立 facing / HVA 帧。
pub fn rasterize_vxl_layer_poses(
    layers: &[VxlLayerPose<'_>],
    palette: &Palette,
) -> Option<VxlSprite> {
    let mut assembled: Vec<(f32, f32, f32, u8)> = Vec::new();
    for layer in layers {
        let frame_idx = match layer.hva {
            Some(h) if h.frame_count > 0 => layer.frame % h.frame_count,
            _ => 0,
        };
        for (section, limb) in layer.vxl.limbs.iter().enumerate() {
            let bone = layer
                .hva
                .and_then(|h| h.get_transform(frame_idx, section as u32))
                .unwrap_or(&limb.transform);
            for v in &limb.voxels {
                let (mx, my, mz) = section_point(limb, v, bone);
                let (x, y, z) = yaw_point(mx, my, mz, 0.0, 0.0, layer.facing);
                assembled.push((x, y, z, v.color_index));
            }
        }
    }
    if assembled.is_empty() {
        return None;
    }

    let mut points: Vec<(i32, i32, i32, u8)> = Vec::with_capacity(assembled.len());
    for (x, y, z, color_index) in assembled {
        let xi = x.round() as i32;
        let yi = y.round() as i32;
        let zi = z.round() as i32;
        let sx = xi - yi;
        let sy = (xi + yi) / 2 - zi;
        let depth = xi + yi + zi;
        points.push((sx, sy, depth, color_index));
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

    Some(VxlSprite {
        width,
        height,
        offset_x: -(width as i32) / 2,
        offset_y: -(height as i32) / 2,
        rgba,
    })
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
    (
        bx + limb.bounds[0],
        by + limb.bounds[1],
        bz + limb.bounds[2],
    )
}

fn section_axis_scale(extent: f32, size: u8) -> f32 {
    if size == 0 {
        1.0
    } else {
        extent / f32::from(size)
    }
}

/// HVA 3×4：旋转作用在坐标上，平移乘肢节 `scale`。
fn apply_matrix_scaled(m: &[f32; 12], x: f32, y: f32, z: f32, limb_scale: f32) -> (f32, f32, f32) {
    let s = if limb_scale.is_finite() && limb_scale > 0.0 {
        limb_scale
    } else {
        1.0
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Rgba, VxlLimb, VxlVoxel};

    fn limb_with(voxels: Vec<VxlVoxel>) -> VxlFile {
        VxlFile {
            limb_count: 1,
            body_size: 0,
            limbs: vec![VxlLimb {
                name: "body".into(),
                scale: 1.0,
                bounds: [0.0, 0.0, 0.0, 4.0, 4.0, 4.0],
                transform: [
                    1.0, 0.0, 0.0, 0.0, //
                    0.0, 1.0, 0.0, 0.0, //
                    0.0, 0.0, 1.0, 0.0,
                ],
                size_x: 4,
                size_y: 4,
                size_z: 4,
                normals_mode: 4,
                voxels,
            }],
        }
    }

    #[test]
    fn raster_one_voxel_opaque() {
        let vxl = limb_with(vec![VxlVoxel {
            x: 1,
            y: 1,
            z: 0,
            color_index: 10,
            normal_index: 0,
        }]);
        let mut colors = [Rgba::transparent(); 256];
        colors[10] = Rgba::rgb(1, 2, 3);
        let pal = Palette { colors };
        let sprite = rasterize_vxl(&vxl, &pal).unwrap();
        assert_eq!(sprite.width, 1);
        assert_eq!(sprite.height, 1);
        assert_eq!(&sprite.rgba[..4], &[1, 2, 3, 255]);
    }

    #[test]
    fn posed_with_identity_hva_matches() {
        let vxl = limb_with(vec![VxlVoxel {
            x: 2,
            y: 0,
            z: 0,
            color_index: 10,
            normal_index: 0,
        }]);
        let mut colors = [Rgba::transparent(); 256];
        colors[10] = Rgba::rgb(9, 9, 9);
        let pal = Palette { colors };
        let hva = HvaFile {
            frame_count: 1,
            section_count: 1,
            section_names: vec!["body".into()],
            transforms: vec![[
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0,
            ]],
        };
        let a = rasterize_vxl(&vxl, &pal).unwrap();
        let b = rasterize_vxl_posed(&vxl, &pal, Some(&hva), 0).unwrap();
        assert_eq!(a.width, b.width);
        assert_eq!(a.height, b.height);
        assert_eq!(a.rgba, b.rgba);
    }

    #[test]
    fn yaw_facing_changes_bounds() {
        let voxels: Vec<_> = (0..8)
            .map(|i| VxlVoxel {
                x: i,
                y: 0,
                z: 0,
                color_index: 10,
                normal_index: 0,
            })
            .collect();
        let vxl = limb_with(voxels);
        let mut colors = [Rgba::transparent(); 256];
        colors[10] = Rgba::rgb(1, 1, 1);
        let pal = Palette { colors };
        let a = rasterize_vxl_posed(&vxl, &pal, None, 0).unwrap();
        let b = rasterize_vxl_posed(&vxl, &pal, None, 32).unwrap();
        assert!(a.width > b.width);
    }

    #[test]
    fn hva_translation_scaled_by_limb_scale() {
        let mut vxl = limb_with(vec![VxlVoxel {
            x: 0,
            y: 0,
            z: 0,
            color_index: 10,
            normal_index: 0,
        }]);
        vxl.limbs[0].scale = 0.5;
        let mut colors = [Rgba::transparent(); 256];
        colors[10] = Rgba::rgb(1, 1, 1);
        let pal = Palette { colors };
        let hva = HvaFile {
            frame_count: 1,
            section_count: 1,
            section_names: vec!["body".into()],
            transforms: vec![[
                1.0, 0.0, 0.0, 10.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0,
            ]],
        };
        // 平移 10 * scale0.5 = 5，应仍得到有限小精灵。
        let s = rasterize_vxl_posed(&vxl, &pal, Some(&hva), 0).unwrap();
        assert_eq!(s.width, 1);
        assert_eq!(s.height, 1);
    }
}
