//! The shopping list: one line naming every missing link of the chain, depth first, then
//! the raw materials to fetch.

use super::resolve::Need;

fn clauses(n: &Need, out: &mut Vec<String>, get: &mut Vec<(String, u32)>) {
    if n.missing() == 0 {
        return;
    }
    let mut line = format!("need {} {}, have {}", n.need, n.label, n.have);
    match &n.via {
        Some(v) => {
            let from: Vec<String> = v.from.iter().map(|(l, k)| format!("{k} {l}")).collect();
            let verb = if v.smelt { "smelt" } else { "craft" };
            line.push_str(&format!(": {verb} {} from {}", v.makes, from.join(" and ")));
            out.push(line);
            for s in &v.subs {
                clauses(s, out, get);
            }
        }
        None => {
            out.push(line);
            let short = n.missing();
            match get.iter_mut().find(|(l, _)| *l == n.label) {
                Some((_, k)) => *k += short,
                None => get.push((n.label.clone(), short)),
            }
        }
    }
}

/// "cannot craft X: need ...; need .... Get 2 any log, 1 coal."
pub fn shopping(verb: &str, item: &str, nodes: &[Need], short_fuel: u32) -> String {
    let mut out = Vec::new();
    let mut get = Vec::new();
    for n in nodes {
        clauses(n, &mut out, &mut get);
    }
    if short_fuel > 0 {
        let coal = short_fuel.div_ceil(8);
        let wood = (short_fuel * 2).div_ceil(3);
        out.push(format!(
            "need fuel for {short_fuel} more smelts: {coal} coal or {wood} logs or planks"
        ));
        get.push(("coal".into(), coal));
    }
    let get: Vec<String> = get.iter().map(|(l, k)| format!("{k} {l}")).collect();
    format!("cannot {verb} {item}: {}. Get {}.", out.join("; "), get.join(", "))
}
