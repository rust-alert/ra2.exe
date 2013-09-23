//! 类型定义标识。

use crate::id::TypeId;

/// 指向 [`crate::definition::RuntimeDefinitions`] 内某类型行的稳定键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeDefinitionId(pub TypeId);
