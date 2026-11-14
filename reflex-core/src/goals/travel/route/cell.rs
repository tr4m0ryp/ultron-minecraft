//! What the route planner knows about one block: whether a body passes through it, stands
//! on it, may break it on the way, and what it costs. The adapter fills `Cell` from its
//! world; the planner never sees block names, only these classes.

use crate::math::BlockPos;

/// One block as the route planner sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cell {
    /// No collision and no harm: air, plants, torches, carpets of snow.
    Open,
    Water,
    Lava,
    /// No collision but harmful or slowing: fire, cactus, berry bushes, powder snow, cobweb.
    Hazard,
    /// Collides. `floor`: a body can stand on its top. `dig`: ticks to break it with the
    /// best tool at hand when the route may break it (natural terrain), `None` when it must
    /// stay (builds, containers, bedrock, ice that melts into water). `falls`: sand, gravel.
    Solid {
        floor: bool,
        dig: Option<u16>,
        falls: bool,
    },
    /// Not loaded, or outside the world.
    Unknown,
}

impl Cell {
    pub const STONE: Cell = Cell::Solid {
        floor: true,
        dig: Some(30),
        falls: false,
    };
    pub const WALL: Cell = Cell::Solid {
        floor: true,
        dig: None,
        falls: false,
    };

    pub fn is_open(self) -> bool {
        self == Cell::Open
    }
    pub fn is_liquid(self) -> bool {
        matches!(self, Cell::Water | Cell::Lava)
    }
    /// A body can stand on top of it.
    pub fn is_floor(self) -> bool {
        matches!(self, Cell::Solid { floor: true, .. })
    }
    pub fn is_solid(self) -> bool {
        matches!(self, Cell::Solid { .. })
    }
    /// A block may be placed into it.
    pub fn takes_block(self) -> bool {
        matches!(self, Cell::Open | Cell::Water)
    }
}

/// Block reads for the planner and the path follower.
pub trait Terrain {
    fn cell(&self, p: BlockPos) -> Cell;
}

impl<F: Fn(BlockPos) -> Cell> Terrain for F {
    fn cell(&self, p: BlockPos) -> Cell {
        self(p)
    }
}

/// A box of blocks (inclusive corners in any order) the route must never break.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub min: BlockPos,
    pub max: BlockPos,
}

impl Region {
    pub fn new(a: BlockPos, b: BlockPos) -> Self {
        Self {
            min: BlockPos::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)),
            max: BlockPos::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)),
        }
    }
    pub fn contains(&self, p: BlockPos) -> bool {
        (self.min.x..=self.max.x).contains(&p.x)
            && (self.min.y..=self.max.y).contains(&p.y)
            && (self.min.z..=self.max.z).contains(&p.z)
    }
}

/// Natural terrain a route may dig through. Anything else (planks, cobblestone, glass,
/// bricks, logs, wool, doors, containers) may be part of a build and stays. Ice and packed
/// ice are left out (ice melts into water), infested stone too (silverfish).
pub fn natural(name: &str) -> bool {
    const EXACT: [&str; 34] = [
        "stone",
        "deepslate",
        "granite",
        "diorite",
        "andesite",
        "tuff",
        "calcite",
        "dirt",
        "coarse_dirt",
        "rooted_dirt",
        "grass_block",
        "podzol",
        "mycelium",
        "mud",
        "clay",
        "sand",
        "red_sand",
        "gravel",
        "sandstone",
        "red_sandstone",
        "terracotta",
        "netherrack",
        "basalt",
        "smooth_basalt",
        "blackstone",
        "end_stone",
        "soul_sand",
        "soul_soil",
        "snow_block",
        "moss_block",
        "dripstone_block",
        "magma_block",
        "pale_moss_block",
        "suspicious_gravel",
    ];
    if name.starts_with("infested_") {
        return false;
    }
    EXACT.contains(&name) || name.ends_with("_ore") || name.ends_with("_leaves")
}

/// Blocks a route or a placement may use as throwaway scaffold, cheapest first. Planks
/// and logs are building material and are never spent on scaffold.
pub const SCAFFOLD: [&str; 10] = [
    "dirt",
    "cobblestone",
    "cobbled_deepslate",
    "netherrack",
    "andesite",
    "diorite",
    "granite",
    "tuff",
    "stone",
    "blackstone",
];

/// What breaking a block leaves in the inventory, for counting a mining goal's yield by
/// what was picked up. Unlisted blocks drop themselves.
pub fn drop_of(block: &str) -> &str {
    match block {
        "stone" => "cobblestone",
        "deepslate" => "cobbled_deepslate",
        "grass_block" | "podzol" | "mycelium" | "rooted_dirt" => "dirt",
        "coal_ore" | "deepslate_coal_ore" => "coal",
        "iron_ore" | "deepslate_iron_ore" => "raw_iron",
        "copper_ore" | "deepslate_copper_ore" => "raw_copper",
        "gold_ore" | "deepslate_gold_ore" => "raw_gold",
        "diamond_ore" | "deepslate_diamond_ore" => "diamond",
        "emerald_ore" | "deepslate_emerald_ore" => "emerald",
        "lapis_ore" | "deepslate_lapis_ore" => "lapis_lazuli",
        "redstone_ore" | "deepslate_redstone_ore" => "redstone",
        "nether_quartz_ore" => "quartz",
        "clay" => "clay_ball",
        "snow_block" => "snowball",
        "glowstone" => "glowstone_dust",
        _ => block,
    }
}
