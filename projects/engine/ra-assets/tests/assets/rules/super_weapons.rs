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

#[test]
fn from_layered_overrides_recharge_and_appends_list() {
    let base = IniDocument::parse(
        b"[SuperWeaponTypes]\n0=LightningStorm\n\
[LightningStorm]\nType=LightningStorm\nRechargeTime=10\n",
    )
    .unwrap();
    let top = IniDocument::parse(
        b"[SuperWeaponTypes]\n1=Nuke\n\
[LightningStorm]\nRechargeTime=3\n\
[Nuke]\nType=MultiMissile\nRechargeTime=5\n",
    )
    .unwrap();
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::MergeSection,
    };
    let docs = [base, top];
    let reg = SuperWeaponTypeRegistry::from_layered(LayeredIniView::new(&docs, &policy));
    assert_eq!(reg.len(), 2);
    assert_eq!(reg.get("LightningStorm").unwrap().recharge_time, 3);
    assert_eq!(reg.get("LightningStorm").unwrap().kind, "LIGHTNINGSTORM");
    assert_eq!(reg.get("Nuke").unwrap().kind, "MULTIMISSILE");
}
