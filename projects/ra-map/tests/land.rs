use ra_map::ground_passable;

#[test]
fn water_and_rock_block_ground() {
    assert!(!ground_passable(3));
    assert!(!ground_passable(4));
    assert!(!ground_passable(5));
    assert!(ground_passable(0));
    assert!(ground_passable(2));
    assert!(ground_passable(255));
}
