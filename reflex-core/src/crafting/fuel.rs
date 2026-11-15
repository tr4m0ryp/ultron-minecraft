//! Furnace fuel, counted in quarter smelts (one smelt is 200 ticks of burning).

/// Quarter smelts one item of `item` burns for; 0 when it is not fuel the planner uses.
pub fn quarters(item: &str) -> u32 {
    match item {
        "coal" | "charcoal" => 32,
        "coal_block" => 320,
        "blaze_rod" => 48,
        "dried_kelp_block" => 80,
        "stick" => 2,
        "bamboo" => 1,
        "bamboo_block" | "stripped_bamboo_block" | "bamboo_planks" => 6,
        _ if item.starts_with("crimson") || item.starts_with("warped") => 0,
        _ if ["_log", "_wood", "_planks"]
            .iter()
            .any(|s| item.ends_with(s)) =>
        {
            6
        }
        _ => 0,
    }
}

/// Lower ranks burn first: coal and charcoal, then other dense fuel, then wood.
fn rank(item: &str) -> u32 {
    match quarters(item) {
        32 => 0,
        q if q > 32 => 1,
        q if q >= 6 => 2,
        _ => 3,
    }
}

/// Fuel for `smelts` items from `inv`: whole items, coal and charcoal first. Returns the
/// picks and the quarter smelts still uncovered (0 when the inventory suffices).
pub fn choose(inv: &[(String, u32)], smelts: u32) -> (Vec<(String, u32)>, u32) {
    let mut need = smelts * 4;
    let mut fuels: Vec<(&String, u32, u32)> = inv
        .iter()
        .filter(|(n, c)| *c > 0 && quarters(n) > 0)
        .map(|(n, c)| (n, *c, quarters(n)))
        .collect();
    fuels.sort_by_key(|(n, c, _)| (rank(n), u32::MAX - c));
    let mut out = Vec::new();
    for (name, have, q) in fuels {
        if need == 0 {
            break;
        }
        let take = need.div_ceil(q).min(have);
        need = need.saturating_sub(take * q);
        out.push((name.clone(), take));
    }
    (out, need)
}
