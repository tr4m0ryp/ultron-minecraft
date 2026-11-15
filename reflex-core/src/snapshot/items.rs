/// Inventory slot addressing: 0..=8 hotbar, 9..=35 main inventory, 40 offhand (vanilla menu order).
pub type Slot = u8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ItemKind {
    Sword,
    Axe,
    Pickaxe,
    Shovel,
    Shield,
    Totem,
    WaterBucket,
    EmptyBucket,
    GoldenApple,
    Food {
        nutrition: u8,
        saturation: f32,
        /// Raw meat, fish or potato: a furnace makes it a better meal, so the reflexes
        /// keep it for cooking and eat it as it is only in an emergency.
        cookable: bool,
    },
    /// A full solid block the bot may spend freely (dirt, cobblestone, netherrack).
    ThrowawayBlock,
    Torch,
    CarvedPumpkin,
    Other,
}

impl ItemKind {
    pub fn is_food(self) -> bool {
        matches!(self, ItemKind::Food { .. } | ItemKind::GoldenApple)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemStack {
    pub kind: ItemKind,
    pub count: u8,
    /// Remaining durability as a fraction, 1.0 for items without durability.
    pub durability: f32,
}

impl ItemStack {
    pub fn new(kind: ItemKind, count: u8) -> Self {
        Self {
            kind,
            count,
            durability: 1.0,
        }
    }
}
