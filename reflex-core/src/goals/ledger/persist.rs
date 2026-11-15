//! The ledger file: one cell per line, `own|companion x y z kind`, and one protected box per
//! line, `region x0 y0 z0 x1 y1 z1`. Written to a temporary file and renamed, so a crash
//! never leaves half a ledger.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::goals::travel::route::Region;
use crate::math::BlockPos;

pub(super) fn load(
    path: &Path,
    own: &mut HashMap<BlockPos, String>,
    companion: &mut HashMap<BlockPos, String>,
    regions: &mut Vec<Region>,
) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    for line in text.lines() {
        if let Some(r) = line.strip_prefix("region ").and_then(region) {
            if !regions.contains(&r) {
                regions.push(r);
            }
            continue;
        }
        let mut w = line.split_whitespace();
        let (Some(who), Some(x), Some(y), Some(z), Some(kind)) =
            (w.next(), w.next(), w.next(), w.next(), w.next())
        else {
            continue;
        };
        let (Ok(x), Ok(y), Ok(z)) = (x.parse(), y.parse(), z.parse()) else {
            continue;
        };
        let cell = BlockPos::new(x, y, z);
        match who {
            "own" => own.insert(cell, kind.to_string()),
            "companion" => companion.insert(cell, kind.to_string()),
            _ => continue,
        };
    }
}

fn region(rest: &str) -> Option<Region> {
    let v: Vec<i32> = rest
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    let [x0, y0, z0, x1, y1, z1] = v[..] else {
        return None;
    };
    Some(Region::new(
        BlockPos::new(x0, y0, z0),
        BlockPos::new(x1, y1, z1),
    ))
}

pub(super) fn save(
    path: &Path,
    own: &HashMap<BlockPos, String>,
    companion: &HashMap<BlockPos, String>,
    regions: &[Region],
) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut lines: Vec<String> = Vec::with_capacity(own.len() + companion.len());
    for (who, cells) in [("own", own), ("companion", companion)] {
        let mut v: Vec<_> = cells.iter().collect();
        v.sort_by_key(|(p, _)| **p);
        lines.extend(
            v.into_iter()
                .map(|(p, k)| format!("{who} {} {} {} {k}", p.x, p.y, p.z)),
        );
    }
    lines.extend(regions.iter().map(|r| {
        let (a, b) = (r.min, r.max);
        format!("region {} {} {} {} {} {}", a.x, a.y, a.z, b.x, b.y, b.z)
    }));
    let tmp = path.with_extension("tmp");
    let mut f = fs::File::create(&tmp)?;
    f.write_all(lines.join("\n").as_bytes())?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    fs::rename(tmp, path)
}
