//! Registry items -> `ItemKind`, with food values and durability read from the item's
//! data components.

use azalea::{
    inventory::{
        components::{Damage, Food, MaxDamage},
        ItemStack as AzStack,
    },
    registry::{builtin::ItemKind as Az, tags::items},
};
use reflex_core::snapshot::{ItemKind, ItemStack};

pub fn map_stack(stack: &AzStack) -> Option<ItemStack> {
    let data = stack.as_present()?;
    let count = data.count.clamp(1, 255) as u8;
    Some(ItemStack {
        kind: kind_of(stack),
        count,
        durability: durability(stack),
    })
}

fn durability(stack: &AzStack) -> f32 {
    let Some(max) = stack.get_component::<MaxDamage>().map(|m| m.amount) else {
        return 1.0;
    };
    let used = stack.get_component::<Damage>().map_or(0, |d| d.amount);
    if max <= 0 {
        return 1.0;
    }
    (1.0 - used as f32 / max as f32).clamp(0.0, 1.0)
}

fn kind_of(stack: &AzStack) -> ItemKind {
    let item = stack.kind();
    match item {
        Az::Shield => return ItemKind::Shield,
        Az::TotemOfUndying => return ItemKind::Totem,
        Az::WaterBucket => return ItemKind::WaterBucket,
        Az::Bucket => return ItemKind::EmptyBucket,
        Az::GoldenApple | Az::EnchantedGoldenApple => return ItemKind::GoldenApple,
        Az::Torch => return ItemKind::Torch,
        Az::CarvedPumpkin => return ItemKind::CarvedPumpkin,
        Az::Cobblestone | Az::Dirt | Az::Netherrack | Az::CobbledDeepslate => {
            return ItemKind::ThrowawayBlock;
        }
        _ => {}
    }
    if items::SWORDS.contains(&item) {
        return ItemKind::Sword;
    }
    if items::AXES.contains(&item) {
        return ItemKind::Axe;
    }
    if items::PICKAXES.contains(&item) {
        return ItemKind::Pickaxe;
    }
    if items::SHOVELS.contains(&item) {
        return ItemKind::Shovel;
    }
    if let Some(food) = stack.get_component::<Food>() {
        return ItemKind::Food {
            nutrition: food.nutrition.clamp(0, 255) as u8,
            saturation: food.saturation,
            cookable: matches!(
                item,
                Az::Beef
                    | Az::Porkchop
                    | Az::Mutton
                    | Az::Chicken
                    | Az::Rabbit
                    | Az::Cod
                    | Az::Salmon
                    | Az::Potato
            ),
        };
    }
    ItemKind::Other
}
