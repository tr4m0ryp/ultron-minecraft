//! Turning a recipe into concrete grid contents: which held item goes in which cell, and
//! whether a laid-out grid is one the server will accept (shaped patterns match mirrored).

use std::collections::HashMap;

use super::book::{Ingredient, Recipe, Shape};

/// Concrete grid contents for `crafts` crafts: each cell's index and item, `crafts` deep.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub crafts: u32,
    pub cells: Vec<(usize, String)>,
}

/// Fits the recipe into a `grid`-wide grid from `inv`. A cell holds one item kind, so the
/// depth drops below `crafts` when no single kind covers a cell that deep (two oak planks and
/// two birch planks still make a table, one deep). `None` when not even one craft fits.
pub fn layout(r: &Recipe, grid: usize, inv: &[(String, u32)], crafts: u32) -> Option<Layout> {
    let fits = match &r.shape {
        Shape::Shaped { width, height, .. } => (*width as usize) <= grid && (*height as usize) <= grid,
        Shape::Shapeless(v) => v.len() <= grid * grid,
        Shape::Smelt { .. } => false,
    };
    if !fits {
        return None;
    }
    (1..=crafts.max(1)).rev().find_map(|n| assign(r, grid, inv, n))
}

fn assign(r: &Recipe, grid: usize, inv: &[(String, u32)], n: u32) -> Option<Layout> {
    let mut left: HashMap<&str, u32> = HashMap::new();
    for (name, c) in inv {
        *left.entry(name.as_str()).or_default() += c;
    }
    let mut cells: Vec<(usize, String)> = Vec::new();
    let mut chosen: Vec<(&Ingredient, String)> = Vec::new();
    for (slot, ing) in r.cells(grid) {
        let ok = |name: &str, left: &HashMap<&str, u32>| left.get(name).is_some_and(|c| *c >= n);
        let reuse = chosen
            .iter()
            .find(|(k, name)| *k == ing && ok(name, &left))
            .map(|(_, name)| name.clone());
        let pick = reuse.or_else(|| {
            ing.0
                .iter()
                .filter(|a| ok(a, &left))
                .max_by_key(|a| (left[a.as_str()], std::cmp::Reverse(a.as_str())))
                .cloned()
        })?;
        *left.get_mut(pick.as_str()).expect("checked") -= n;
        chosen.push((ing, pick.clone()));
        cells.push((slot, pick));
    }
    Some(Layout { crafts: n, cells })
}

/// Whether the server crafts `r` from this `grid`-wide grid (row-major, `None` empty).
pub fn matches(r: &Recipe, grid: usize, items: &[Option<String>]) -> bool {
    match &r.shape {
        Shape::Shaped {
            width,
            height,
            cells,
        } => {
            let used: Vec<usize> = (0..items.len()).filter(|i| items[*i].is_some()).collect();
            let (Some(r0), Some(r1)) = (used.iter().map(|i| i / grid).min(), used.iter().map(|i| i / grid).max()) else {
                return false;
            };
            let c0 = used.iter().map(|i| i % grid).min().unwrap_or(0);
            let c1 = used.iter().map(|i| i % grid).max().unwrap_or(0);
            let (w, h) = (*width as usize, *height as usize);
            if c1 - c0 + 1 > w || r1 - r0 + 1 > h || c0 + w > grid || r0 + h > grid {
                return false;
            }
            let fit = |mirror: bool| {
                (0..h).all(|row| {
                    (0..w).all(|col| {
                        let pc = if mirror { w - 1 - col } else { col };
                        let want = &cells[row * w + pc];
                        let got = items.get((r0 + row) * grid + c0 + col).cloned().flatten();
                        match (want, got) {
                            (None, None) => true,
                            (Some(i), Some(g)) => i.accepts(&g),
                            _ => false,
                        }
                    })
                })
            };
            fit(false) || fit(true)
        }
        Shape::Shapeless(ings) => {
            let got: Vec<&String> = items.iter().flatten().collect();
            got.len() == ings.len() && pair(&got, &mut vec![false; ings.len()], ings)
        }
        Shape::Smelt { .. } => false,
    }
}

/// Every laid item matched to a distinct shapeless ingredient (backtracking, at most nine).
fn pair(got: &[&String], used: &mut Vec<bool>, ings: &[Ingredient]) -> bool {
    let Some((first, rest)) = got.split_first() else {
        return true;
    };
    for i in 0..ings.len() {
        if !used[i] && ings[i].accepts(first) {
            used[i] = true;
            if pair(rest, used, ings) {
                return true;
            }
            used[i] = false;
        }
    }
    false
}
