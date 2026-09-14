//! 部署与形态切换（`DeploysInto` / `Deployer` / `UndeploysInto`）。

use ra_types::{RuntimeDefinitions, TypeId};

use super::definitions_query::{deploy_into_type, undeploys_into_type};

/// 类型是否声明 `Deployer=yes`。
pub(crate) fn is_deployer(defs: &RuntimeDefinitions, type_id: TypeId) -> bool {
    defs.techno.get_by_id(type_id).is_some_and(|t| t.deployer)
}

/// 实体类型当前是否可接受 `Deploy` 命令（含蹲姿切换与解除部署）。
pub(crate) fn type_can_deploy(defs: &RuntimeDefinitions, type_id: TypeId) -> bool {
    deploy_into_type(defs, type_id).is_some() || undeploys_into_type(defs, type_id).is_some() || is_deployer(defs, type_id)
}
