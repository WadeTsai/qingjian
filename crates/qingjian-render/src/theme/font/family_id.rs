//! 主題字族表（[`super::FontFamilies`]）裡一條回退鏈的序號；0 是主題的預設字族。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct FamilyId(pub(crate) u16);
