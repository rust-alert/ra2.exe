//! 超武类型注册表。

use ra_assets::*;

#[test]
fn parse_super_weapon_types_list() {
    let doc = IniDocument::parse(
        b"[SuperWeaponTypes]\n0=LightningStorm\n1=Nuke\n\
[LightningStorm]\nUIName=Name:LightningStorm\nType=LightningStorm\nAction=LightningStorm\n\
RechargeTime=10\nSidebarImage=SSWLSICON\nWeapon=LightningBolt\n\
[Nuke]\nUIName=Name:Nuke\nType=MultiMissile\nRechargeTime=5\n",
    )
    .unwrap();
    let reg = SuperWeaponTypeRegistry::from_rules(&doc);
    assert_eq!(reg.len(), 2);
    let ls = reg.get("lightningstorm").unwrap();
    assert_eq!(ls.ui_name, "Name:LightningStorm");
    assert_eq!(ls.kind, "LIGHTNINGSTORM");
    assert_eq!(ls.action, "LIGHTNINGSTORM");
    assert_eq!(ls.recharge_time, 10);
    assert_eq!(ls.sidebar_image, "SSWLSICON");
    assert_eq!(ls.weapon, "LIGHTNINGBOLT");
    assert_eq!(reg.get("NUKE").unwrap().kind, "MULTIMISSILE");
    assert_eq!(reg.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["LIGHTNINGSTORM", "NUKE"]);
}
