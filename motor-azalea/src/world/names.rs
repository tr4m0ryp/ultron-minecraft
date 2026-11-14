//! Registry names without the namespace, and query-kind matching over the registries.

use azalea::{
    block::BlockState,
    inventory::ItemStack,
    registry::{builtin::BlockKind, Registry},
};
use reflex_core::goals::kind_matches;

pub fn bare(s: &'static str) -> &'static str {
    s.strip_prefix("minecraft:").unwrap_or(s)
}

pub fn block_name(state: BlockState) -> String {
    bare(BlockKind::from(state).to_str()).to_string()
}

pub fn item_name(stack: &ItemStack) -> Option<&'static str> {
    (!stack.is_empty()).then(|| bare(stack.kind().to_str()))
}

pub fn item_matches(kind: &str, stack: &ItemStack) -> bool {
    item_name(stack).is_some_and(|n| kind_matches(kind, n))
}

/// Every block kind whose name matches the query kind ("bed" gives all sixteen beds).
pub fn block_kinds(kind: &str) -> Vec<BlockKind> {
    (0u32..)
        .map_while(BlockKind::from_u32)
        .filter(|k| kind_matches(kind, bare(k.to_str())))
        .collect()
}
