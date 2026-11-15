//! The ledger file: one cell per line, `own|companion x y z kind`. Written to a temporary
//! file and renamed, so a crash never leaves half a ledger.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::math::BlockPos;

pub(super) fn load(
    path: &Path,
    own: &mut HashMap<BlockPos, String>,
    companion: &mut HashMap<BlockPos, String>,
) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    for line in text.lines() {
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

pub(super) fn save(
    path: &Path,
    own: &HashMap<BlockPos, String>,
    companion: &HashMap<BlockPos, String>,
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
    let tmp = path.with_extension("tmp");
    let mut f = fs::File::create(&tmp)?;
    f.write_all(lines.join("\n").as_bytes())?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    fs::rename(tmp, path)
}
