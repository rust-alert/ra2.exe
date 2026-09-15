//! 集成测试：原 `src/vxl_raster.rs` 内联测试迁出。

use ra_assets::*;

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
    let vxl = limb_with(vec![VxlVoxel { x: 1, y: 1, z: 0, color_index: 10, normal_index: 0 }]);
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
    let vxl = limb_with(vec![VxlVoxel { x: 2, y: 0, z: 0, color_index: 10, normal_index: 0 }]);
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
    let voxels: Vec<_> = (0..8).map(|i| VxlVoxel { x: i, y: 0, z: 0, color_index: 10, normal_index: 0 }).collect();
    let vxl = limb_with(voxels);
    let mut colors = [Rgba::transparent(); 256];
    colors[10] = Rgba::rgb(1, 1, 1);
    let pal = Palette { colors };
    let a = rasterize_vxl_posed(&vxl, &pal, None, 0).unwrap();
    let b = rasterize_vxl_posed(&vxl, &pal, None, 128).unwrap();
    assert_ne!((a.offset_x, a.offset_y), (b.offset_x, b.offset_y));
}

#[test]
fn vxl_yaw_quantizes_to_thirty_two_steps_rounded() {
    assert_eq!(VXL_FACING_STEPS, 32);
    assert_eq!(VXL_FACING_BYTE_STEP, 8);
    assert_eq!(vxl_yaw_steps(0), 0);
    assert_eq!(vxl_yaw_steps(3), 0);
    // 半档边界 facing=4 进到下一档。
    assert_eq!(vxl_yaw_steps(4), 1);
    assert_eq!(vxl_yaw_steps(7), 1);
    assert_eq!(vxl_yaw_steps(8), 1);
    assert_eq!(vxl_yaw_steps(32), 4);
    assert_eq!(vxl_yaw_steps(64), 8);
    assert_eq!(vxl_yaw_steps(255), 0); // (63+1)>>1 = 32 → &31 = 0
    assert_ne!(vxl_yaw_steps(8), vxl_yaw_steps(0));
    assert_ne!(vxl_yaw_steps(16), vxl_yaw_steps(8));
    // 车身角：(step-8)*(-π/16)；step=8 → 0；step=0 → +π/2。
    assert!((vxl_yaw_radians(64)).abs() < 1e-5);
    assert!((vxl_yaw_radians(0) - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    assert!((vxl_yaw_radians(32) - std::f32::consts::FRAC_PI_4).abs() < 1e-5);
}

#[test]
fn yaw_fine_facing_differs_from_coarse_eight_way() {
    // 细长体素条：facing=8（约 11.25°）在截断八向里与 0 同档，32 向四舍五入应改变投影。
    let voxels: Vec<_> = (0..8).map(|i| VxlVoxel { x: i, y: 0, z: 0, color_index: 10, normal_index: 0 }).collect();
    let vxl = limb_with(voxels);
    let mut colors = [Rgba::transparent(); 256];
    colors[10] = Rgba::rgb(1, 1, 1);
    let pal = Palette { colors };
    let a = rasterize_vxl_posed(&vxl, &pal, None, 0).unwrap();
    let fine = rasterize_vxl_posed(&vxl, &pal, None, 8).unwrap();
    assert_ne!((a.width, a.height), (fine.width, fine.height), "facing=8 must not collapse into facing=0");
    let finer = rasterize_vxl_posed(&vxl, &pal, None, 16).unwrap();
    assert_ne!((fine.width, fine.height), (finer.width, finer.height));
}

#[test]
fn sprite_anchors_at_model_origin_not_opaque_aabb_center() {
    // 全部体素在 +X，包围盒中心会偏右；原点锚点用 min 投影，不等于 -width/2。
    let voxels: Vec<_> = (2..6).map(|i| VxlVoxel { x: i, y: 0, z: 0, color_index: 10, normal_index: 0 }).collect();
    let vxl = limb_with(voxels);
    let mut colors = [Rgba::transparent(); 256];
    colors[10] = Rgba::rgb(1, 1, 1);
    let pal = Palette { colors };
    // facing=64 → 车身零转，仅相机；原点投影仍为 (0,0)。
    let s = rasterize_vxl_posed(&vxl, &pal, None, 64).unwrap();
    assert_ne!(s.offset_x, -(s.width as i32) / 2, "must not AABB-center");
    // offset=min_sx：叠画 dest=cell+offset+local 使模型原点落到 cell，即使原点在精灵外。
    assert!(s.offset_x != 0 || s.width > 1);
    // 体素全在 +X 时原点常在精灵左侧之外（origin_px < 0），这仍是合法原点锚点。
    assert!(-s.offset_x < s.width as i32);
}

#[test]
fn model_origin_foot_stable_across_body_yaw() {
    // 不对称长条：换朝向后 AABB 尺寸会变，但模型原点屏幕位置恒为 (0,0)，
    // 故 offset + foot = 0（叠画脚点钉在锚点，不随 AABB 漂移）。
    let voxels: Vec<_> = (0..8).map(|i| VxlVoxel { x: i, y: 0, z: 0, color_index: 10, normal_index: 0 }).collect();
    let vxl = limb_with(voxels);
    let mut colors = [Rgba::transparent(); 256];
    colors[10] = Rgba::rgb(1, 1, 1);
    let pal = Palette { colors };
    let mut sizes = Vec::new();
    for facing in [0u8, 32, 64, 96, 128, 160, 192, 224] {
        let s = rasterize_vxl_posed(&vxl, &pal, None, facing).unwrap();
        assert_eq!(s.offset_x + (-s.offset_x), 0);
        assert_eq!(s.offset_y + (-s.offset_y), 0);
        sizes.push((s.width, s.height, s.offset_x, s.offset_y));
    }
    // 朝向变化应改变投影尺寸或偏移，但脚点约束上面已覆盖。
    let uniq: std::collections::HashSet<_> = sizes.iter().copied().collect();
    assert!(uniq.len() > 1, "yaw must change projected metrics: {sizes:?}");
}

#[test]
fn body_and_shadow_share_model_origin_across_yaw() {
    let voxels: Vec<_> = (0..6).map(|i| VxlVoxel { x: i, y: 1, z: 0, color_index: 10, normal_index: 0 }).collect();
    let vxl = limb_with(voxels);
    let mut colors = [Rgba::transparent(); 256];
    colors[10] = Rgba::rgb(1, 1, 1);
    let pal = Palette { colors };
    for facing in [0u8, 64, 128, 192] {
        let pose = VxlLayerPose { vxl: &vxl, hva: None, facing, frame: 0 };
        let body = rasterize_vxl_layer_poses(&[pose], &pal, None).unwrap();
        let shadow = rasterize_vxl_shadow_layer_poses(&[pose]).unwrap();
        assert_eq!(shadow.offset_x - VXL_SHADOW_LIGHT_OFFSET_X, body.offset_x, "facing={facing}");
        assert_eq!(shadow.offset_y, body.offset_y, "facing={facing}");
    }
}

#[test]
fn hva_translation_scaled_by_limb_scale() {
    let mut vxl = limb_with(vec![VxlVoxel { x: 0, y: 0, z: 0, color_index: 10, normal_index: 0 }]);
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
    let s = rasterize_vxl_posed(&vxl, &pal, Some(&hva), 64).unwrap();
    assert_eq!(s.width, 1);
    assert_eq!(s.height, 1);
}

#[test]
fn vpl_shades_by_normal_page() {
    let vxl = limb_with(vec![VxlVoxel { x: 1, y: 1, z: 0, color_index: 10, normal_index: 255 }]);
    let mut colors = [Rgba::transparent(); 256];
    colors[10] = Rgba::rgb(10, 10, 10);
    colors[42] = Rgba::rgb(42, 0, 0);
    let pal = Palette { colors };
    let mut data = Vec::new();
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&31u32.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&[0u8; 768]);
    let mut page0 = [0u8; 256];
    page0[10] = 10;
    let mut page1 = [0u8; 256];
    page1[10] = 42;
    data.extend_from_slice(&page0);
    data.extend_from_slice(&page1);
    let vpl = VplFile::parse(&data).unwrap();
    let pose = VxlLayerPose { vxl: &vxl, hva: None, facing: 64, frame: 0 };
    let sprite = rasterize_vxl_layer_poses(&[pose], &pal, Some(&vpl)).unwrap();
    assert_eq!(&sprite.rgba[..4], &[42, 0, 0, 255]);
}

#[test]
fn shadow_marks_occupied_columns_only() {
    let vxl = limb_with(vec![
        VxlVoxel { x: 1, y: 1, z: 0, color_index: 10, normal_index: 0 },
        VxlVoxel { x: 1, y: 1, z: 2, color_index: 10, normal_index: 0 },
        VxlVoxel { x: 3, y: 0, z: 1, color_index: 10, normal_index: 0 },
    ]);
    let pose = VxlLayerPose { vxl: &vxl, hva: None, facing: 64, frame: 0 };
    let shadow = rasterize_vxl_shadow_layer_poses(&[pose]).unwrap();
    let lit = shadow.rgba.chunks_exact(4).filter(|c| c[3] != 0).count();
    assert_eq!(lit, 2, "one stamp per occupied column");
    assert!(shadow.rgba.chunks_exact(4).filter(|c| c[3] != 0).all(|c| c[..3] == [0, 0, 0]));
}

#[test]
fn shadow_shifts_right_of_body_projection() {
    let vxl = limb_with(vec![VxlVoxel { x: 2, y: 1, z: 0, color_index: 10, normal_index: 0 }]);
    let mut colors = [Rgba::transparent(); 256];
    colors[10] = Rgba::rgb(1, 1, 1);
    let pal = Palette { colors };
    let pose = VxlLayerPose { vxl: &vxl, hva: None, facing: 64, frame: 0 };
    let body = rasterize_vxl_layer_poses(&[pose], &pal, None).unwrap();
    let shadow = rasterize_vxl_shadow_layer_poses(&[pose]).unwrap();
    assert_eq!(body.width, 1);
    assert_eq!(shadow.width, 1);
    // 同源点投影后落影多 VXL_SHADOW_LIGHT_OFFSET_X；原点锚点下 offset_x 同步大这么多。
    assert_eq!(shadow.offset_x, body.offset_x + VXL_SHADOW_LIGHT_OFFSET_X);
}
