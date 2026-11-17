use std::sync::Arc;

use crate::crafting::{Recipe, RecipeBook};
use crate::math::{BlockPos, Vec3};
use crate::snapshot::{Slot, Snapshot};

/// Block and item kinds are registry names without the namespace ("iron_ore", "oak_log").
/// A family word matches a name that equals it or ends with `_<kind>`: "bed" matches
/// "red_bed", "log" matches "oak_log". Only for kinds that are not a name themselves; see
/// `name_matches`.
pub fn kind_matches(kind: &str, name: &str) -> bool {
    name == kind
        || (name.len() > kind.len()
            && name.ends_with(kind)
            && name.as_bytes()[name.len() - kind.len() - 1] == b'_')
}

/// Whether `name` is what the query `kind` asks for. `full` says `kind` is itself the name
/// of a block or an item: then only that name matches, plus the deepslate form of an ore
/// ("iron_ore" also takes "deepslate_iron_ore", the same ore in other rock). Live, a count
/// of healing_potion took strong_healing_potion along, "furnace" found a blast furnace
/// and "beef" counted cooked beef. A family word ("bed", "log", "planks") keeps matching
/// by its last words.
pub fn name_matches(kind: &str, name: &str, full: bool) -> bool {
    if !full {
        return kind_matches(kind, name);
    }
    name == kind || (kind.ends_with("_ore") && name.strip_prefix("deepslate_") == Some(kind))
}

/// What breaking a block needs from the inventory.
#[derive(Clone, Debug, PartialEq)]
pub enum ToolNeed {
    /// Any item or the bare hand drops it.
    Hand,
    /// The best suitable tool sits in this slot (0..=8 hotbar, 9..=35 main inventory).
    Have(Slot),
    /// Without a tool of this class (e.g. "stone_pickaxe or better") the block drops nothing.
    Missing(String),
}

/// One visit to a furnace: the output is always taken first, then `input` and `fuel` are
/// loaded (a foreign item in the input slot is taken out to make room).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FurnaceJob {
    pub input: Option<(String, u32)>,
    pub fuel: Option<(String, u32)>,
}

/// A furnace's three slots as the last visit left them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FurnaceSlots {
    pub input: Option<(String, u32)>,
    pub fuel: Option<(String, u32)>,
    pub output: Option<(String, u32)>,
}

/// Which menu a station visit expects to open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StationMenu {
    BrewingStand,
    Smithing,
}

/// One visit to a brewing stand or a smithing table, in menu slot numbers (brewing: bottles
/// 0..=2, reagent 3, fuel 4; smithing: template 0, base 1, addition 2, result 3). The
/// `take` slots are emptied into the inventory first, then each `put` moves that many of an
/// item into its slot, then the `result` slots are taken once the server has filled them.
/// `fuel` is put only into an empty slot and only when held: blaze powder waits in a
/// stand's fuel slot until the stand needs it, so offering it is never wrong.
#[derive(Clone, Debug, PartialEq)]
pub struct StationJob {
    pub menu: StationMenu,
    pub take: Vec<usize>,
    pub put: Vec<(usize, String, u32)>,
    pub fuel: Option<(usize, String)>,
    pub result: Vec<usize>,
}

/// An item entity lying on the ground.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemDrop {
    pub entity_id: u32,
    pub item: String,
    pub count: u32,
    pub pos: Vec3,
}

/// Progress of a menu action (crafting) the adapter runs over several ticks.
#[derive(Clone, Debug, PartialEq)]
pub enum MenuStatus {
    Idle,
    Busy,
    Done,
    Failed(String),
}

/// World model, recipes and menu actions port. Queries are synchronous and cheap; menu
/// actions start on the tick the goal holds its channels and finish later.
pub trait World {
    /// `kind` is itself the registry name of a block or an item (a potion by its
    /// contents included), not a family word such as "bed" or "log".
    fn full_name(&self, kind: &str) -> bool;
    /// Whether a block or item called `name` is what the query `kind` asks for.
    fn matches(&self, kind: &str, name: &str) -> bool {
        name_matches(kind, name, self.full_name(kind))
    }
    /// Registry name of the block at `p`, `None` when unloaded.
    fn block_at(&self, p: BlockPos) -> Option<String>;
    /// Nearest loaded block whose name matches `kind`, within `radius` blocks of `from`,
    /// skipping `exclude`.
    fn nearest_block(
        &self,
        kind: &str,
        from: BlockPos,
        radius: i32,
        exclude: &[BlockPos],
    ) -> Option<BlockPos>;
    fn tool_for(&self, block: &str, snap: &Snapshot) -> ToolNeed;
    fn count_item(&self, item: &str) -> u32;
    /// Main inventory and hotbar by exact item name, counts summed per name.
    fn inventory(&self) -> Vec<(String, u32)>;
    /// Items held in the main inventory and hotbar, all kinds together.
    fn total_items(&self) -> u32;
    /// Slot of a stack whose name matches `item`, hotbar first.
    fn item_slot(&self, item: &str) -> Option<Slot>;
    /// Empty main-inventory and hotbar slots.
    fn free_slots(&self) -> u32;
    /// The recipe book (empty when the recipe data could not be loaded).
    fn recipe_book(&self) -> Arc<RecipeBook>;
    /// Item entities within `radius` of `around` whose name matches `item` (all when `None`).
    fn item_drops(&self, item: Option<&str>, around: Vec3, radius: f64) -> Vec<ItemDrop>;
    /// The player is lying in a bed.
    fn sleeping(&self) -> bool;
    /// Nothing but air above the head at the standing cell `feet`.
    fn open_sky(&self, _feet: BlockPos) -> bool {
        true
    }
    /// Start crafting `recipe` `crafts` times; `table` is the crafting table for 3x3 recipes.
    /// The adapter picks concrete held items for tag ingredients and may craft fewer times
    /// when no single kind covers a cell (the goal re-plans and goes again).
    fn start_craft(&mut self, recipe: &Recipe, crafts: u32, table: Option<BlockPos>);
    fn craft_status(&mut self) -> MenuStatus;
    /// Open the furnace at `pos`, collect its output, load the job and close it again: the
    /// furnace cooks with its menu closed.
    fn start_furnace(&mut self, pos: BlockPos, job: &FurnaceJob);
    fn furnace_status(&mut self) -> MenuStatus;
    /// The slots the last finished furnace visit saw.
    fn furnace_seen(&self) -> Option<FurnaceSlots>;
    /// Open the brewing stand or smithing table at `pos`, run the job and close it again.
    fn start_station(&mut self, pos: BlockPos, job: &StationJob);
    fn station_status(&mut self) -> MenuStatus;
    /// Throw up to `count` of `item` the way the player is facing. Returns how many left
    /// the inventory.
    fn toss(&mut self, item: &str, count: u32) -> u32;
}
