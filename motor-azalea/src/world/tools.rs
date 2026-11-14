//! Which inventory tool breaks a block with drops.
//!
//! Azalea decodes the item `Tool` component's tag-based rules without their contents, so
//! its own correct-tool check never matches network items. This uses the vanilla mining
//! tags and tool tiers from the bundled registry instead.

use azalea::{
    block::{BlockState, BlockTrait},
    entity::inventory::Inventory,
    inventory::ItemStack,
    registry::{
        builtin::{BlockKind, ItemKind},
        tags::{blocks, items},
    },
    Client,
};
use reflex_core::goals::ToolNeed;

use super::inventory::{reflex_slot, PLAYER_SLOTS};
use super::names::{block_kinds, item_name};

/// Tool class name and a test for items of that class.
type ToolClass = (&'static str, fn(ItemKind) -> bool);

fn tool_class(b: BlockKind) -> Option<ToolClass> {
    if blocks::MINEABLE_PICKAXE.contains(&b) {
        Some(("pickaxe", |i| items::PICKAXES.contains(&i)))
    } else if blocks::MINEABLE_AXE.contains(&b) {
        Some(("axe", |i| items::AXES.contains(&i)))
    } else if blocks::MINEABLE_SHOVEL.contains(&b) {
        Some(("shovel", |i| items::SHOVELS.contains(&i)))
    } else if blocks::MINEABLE_HOE.contains(&b) {
        Some(("hoe", |i| items::HOES.contains(&i)))
    } else {
        None
    }
}

fn needed_tier(b: BlockKind) -> (u8, &'static str) {
    if blocks::NEEDS_DIAMOND_TOOL.contains(&b) {
        (3, "diamond")
    } else if blocks::NEEDS_IRON_TOOL.contains(&b) {
        (2, "iron")
    } else if blocks::NEEDS_STONE_TOOL.contains(&b) {
        (1, "stone")
    } else {
        (0, "wooden")
    }
}

/// Harvest tier from the material prefix; gold harvests like wood, copper like stone.
fn tier(stack: &ItemStack) -> u8 {
    let name = item_name(stack).unwrap_or("");
    match name.split('_').next() {
        Some("netherite") => 4,
        Some("diamond") => 3,
        Some("iron") => 2,
        Some("stone") | Some("copper") => 1,
        _ => 0,
    }
}

pub fn tool_for(bot: &Client, block: &str) -> ToolNeed {
    let Some(&kind) = block_kinds(block).first() else {
        return ToolNeed::Hand;
    };
    let required = Box::<dyn BlockTrait>::from(BlockState::from(kind))
        .behavior()
        .requires_correct_tool_for_drops;
    let Some((class, fits)) = tool_class(kind) else {
        return ToolNeed::Hand;
    };
    let (need, need_name) = if required { needed_tier(kind) } else { (0, "") };
    let ecs = bot.ecs.read();
    let Some(inv) = ecs.get::<Inventory>(bot.entity) else {
        return ToolNeed::Hand;
    };
    let menu = &inv.inventory_menu;
    let best = PLAYER_SLOTS
        .filter_map(|i| Some((i, menu.slot(i)?)))
        .filter(|(_, s)| !s.is_empty() && fits(s.kind()) && tier(s) >= need)
        .max_by_key(|(_, s)| tier(s));
    match (best.and_then(|(i, _)| reflex_slot(i)), required) {
        (Some(slot), _) => ToolNeed::Have(slot),
        (None, false) => ToolNeed::Hand,
        (None, true) => ToolNeed::Missing(format!("a {need_name} {class} or better")),
    }
}
