//! AITriggerTypes 最小执行：按冷却排队产队。

use crate::common::{test_engine, battle_from_rules};
use ra_adaptor::RulesSystem;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, RulesGlobals, OverlayTypeRegistry, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{Session, SessionBootKind};
use ra_map::MapInfo;
use ra_types::GameEdition;

fn rules_with_e1() -> RulesSystem {
    let rules = IniDocument::parse(
        b"[InfantryTypes]\n0=E1\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n",
    )
    .unwrap();
    RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        globals: RulesGlobals::from_rules(&rules),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
        super_weapons: SuperWeaponTypeRegistry::default(),
    }
}

#[test]
fn ai_trigger_spawns_team_on_campaign_tick() {
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=Strike,TM1,Russians,0\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers.len(), 1);
    assert!(ra_map::campaign_blocking_capability_message(&map).is_none());

    let engine = test_engine();
    let mut session = Session::from_state(battle_from_rules(&rules_with_e1(), map), "ai");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    assert!(session.expect_battle().world.ai_trigger_runtime.enabled);

    session.tick(&engine.runtime());
    let e1 = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert!(e1 >= 2, "AITrigger should enqueue Create Team on first tick, got {e1}");
}
