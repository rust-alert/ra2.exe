//! 集成测试：原 `src/command.rs` 内联测试迁出。

use ra_types::*;

#[test]
fn scheduled_command_exposes_kind_and_target() {
    let cmd = ScheduledCommand::new(CommandId(7), PlayerId(1), Tick(3), CommandBody::Attack { attacker: EntityId(1), target: EntityId(2) });
    assert_eq!(cmd.id, CommandId(7));
    assert_eq!(cmd.player, PlayerId(1));
    assert_eq!(cmd.tick, Tick(3));
    assert_eq!(cmd.kind(), CommandKind::Attack);
    assert_eq!(cmd.target(), CommandTarget::Entity(EntityId(2)));
}
