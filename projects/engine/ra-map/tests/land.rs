use ra_map::{LandType, ground_passable, land_passable, tmp_terrain_to_land_type};

#[test]
fn tmp_water_byte_blocks_ground() {
    // TMP terrain_type 9 → Water，不可走。
    assert_eq!(tmp_terrain_to_land_type(9), LandType::Water);
    assert!(!ground_passable(9));
}

#[test]
fn tmp_rock_bytes_block_ground() {
    assert_eq!(tmp_terrain_to_land_type(7), LandType::Rock);
    assert_eq!(tmp_terrain_to_land_type(8), LandType::Rock);
    assert!(!ground_passable(7));
    assert!(!ground_passable(8));
}

#[test]
fn tmp_road_and_clear_passable() {
    assert_eq!(tmp_terrain_to_land_type(0), LandType::Clear);
    assert_eq!(tmp_terrain_to_land_type(11), LandType::Road);
    assert_eq!(tmp_terrain_to_land_type(12), LandType::Road);
    assert!(ground_passable(0));
    assert!(ground_passable(11));
    // 越界 TMP 字节按 Clear。
    assert!(ground_passable(255));
}

#[test]
fn land_name_and_passable() {
    assert_eq!(LandType::parse_name("Road"), Some(LandType::Road));
    assert_eq!(LandType::parse_name("water"), Some(LandType::Water));
    assert!(land_passable(LandType::Road));
    assert!(!land_passable(LandType::Water));
    assert!(!land_passable(LandType::Wall));
}
