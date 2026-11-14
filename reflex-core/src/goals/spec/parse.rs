use super::{AttackTarget, GoalSpec, Person};
use crate::math::Vec3;
use crate::snapshot::EntityKind;

const DEFAULT_FOLLOW: f64 = 3.0;

/// Parses a spec such as `mine iron_ore 3`, `mine(iron_ore, 3)`, `give Steve bread 2`,
/// `place cobblestone 10 64 -3` or `attack #42`. `target` is the goal's knob target and
/// stands in for missing coordinates. Returns a human-readable error on bad input.
pub fn parse(spec: &str, target: Option<Vec3>) -> Result<GoalSpec, String> {
    let cleaned: String = spec
        .chars()
        .map(|c| if matches!(c, '(' | ')' | ',') { ' ' } else { c })
        .collect();
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let Some((&head, args)) = words.split_first() else {
        return target.map(GoalSpec::GoTo).ok_or("empty goal".into());
    };
    let head = head.to_ascii_lowercase();
    let count = |i: usize| -> Result<u32, String> {
        args.get(i)
            .map_or(Ok(1), |w| w.parse().map_err(|_| format!("bad count {w:?}")))
    };
    let word = |i: usize, what: &str| -> Result<String, String> {
        args.get(i)
            .map(|w| w.trim_start_matches("minecraft:").to_ascii_lowercase())
            .ok_or(format!("{head} needs {what}"))
    };
    Ok(match head.as_str() {
        "follow" => {
            let (who, rest) = person_prefix(args);
            let distance = match rest.first() {
                Some(w) => w.parse().map_err(|_| format!("bad distance {w:?}"))?,
                None => DEFAULT_FOLLOW,
            };
            GoalSpec::Follow { who, distance }
        }
        "go_to" | "goto" | "walk" | "go" => match coords(args) {
            Some(v) => GoalSpec::GoTo(v),
            None => GoalSpec::GoTo(target.ok_or("go_to needs x y z")?),
        },
        "come_here" | "come" => GoalSpec::ComeHere,
        "stay" | "wait" => GoalSpec::Stay,
        "mine" | "dig" => GoalSpec::Mine {
            block: word(0, "a block kind")?,
            count: count(1)?,
        },
        "collect" | "pickup" => GoalSpec::Collect {
            item: word(0, "an item kind")?,
            count: count(1)?,
        },
        "attack" | "kill" => GoalSpec::Attack(attack_target(&word(0, "a target")?)?),
        "guard" | "protect" => GoalSpec::Guard(person_prefix(args).0),
        "craft" => GoalSpec::Craft {
            item: word(0, "an item")?,
            count: count(1)?,
        },
        "give" => give(args)?,
        "place" => {
            let block = word(0, "a block")?;
            let pos = coords(&args[1..])
                .or(target)
                .ok_or("place needs x y z")?
                .block();
            let to = args.get(4..).and_then(coords).map(Vec3::block);
            if let Some(t) = to {
                let cells = super::box_cells(pos, t);
                if cells > crate::goals::items::FILL_MAX as u64 {
                    return Err(format!(
                        "place box has {cells} cells; at most {} per goal",
                        crate::goals::items::FILL_MAX
                    ));
                }
            }
            GoalSpec::Place { block, pos, to }
        }
        "sleep" | "sleep_in_bed" => GoalSpec::SleepInBed,
        "eat" | "eat_now" => GoalSpec::EatNow,
        other => return Err(format!("unknown goal {other:?}")),
    })
}

/// A leading word that is neither a number nor "companion" names another player.
fn person_prefix<'a>(args: &'a [&'a str]) -> (Person, &'a [&'a str]) {
    match args.first() {
        Some(w) if w.parse::<f64>().is_err() => {
            let who = if w.eq_ignore_ascii_case("companion") || w.eq_ignore_ascii_case("me") {
                Person::Companion
            } else {
                Person::Named((*w).to_string())
            };
            (who, &args[1..])
        }
        _ => (Person::Companion, args),
    }
}

/// `give <item> [n]` or `give <who> <item> [n]`.
fn give(args: &[&str]) -> Result<GoalSpec, String> {
    let item_first = match args {
        [_] => true,
        [_, second] => second.parse::<u32>().is_ok(),
        _ => false,
    };
    let (who, rest) = if item_first {
        (Person::Companion, args)
    } else {
        person_prefix(args)
    };
    let item = rest.first().ok_or("give needs an item")?;
    let count = match rest.get(1) {
        Some(w) => w.parse().map_err(|_| format!("bad count {w:?}"))?,
        None => 1,
    };
    Ok(GoalSpec::Give {
        who,
        item: item.trim_start_matches("minecraft:").to_ascii_lowercase(),
        count,
    })
}

fn coords(args: &[&str]) -> Option<Vec3> {
    let n: Vec<f64> = args.iter().take(3).map_while(|w| w.parse().ok()).collect();
    (n.len() == 3).then(|| Vec3::new(n[0], n[1], n[2]))
}

fn attack_target(w: &str) -> Result<AttackTarget, String> {
    if let Some(id) = w.strip_prefix('#') {
        return id
            .parse()
            .map(AttackTarget::Id)
            .map_err(|_| format!("bad entity id {w:?}"));
    }
    if let Ok(id) = w.parse() {
        return Ok(AttackTarget::Id(id));
    }
    ALL_KINDS
        .iter()
        .find(|k| k.label() == w)
        .map(|k| AttackTarget::Kind(*k))
        .ok_or(format!("unknown entity kind {w:?}"))
}

const ALL_KINDS: [EntityKind; 24] = {
    use EntityKind::*;
    [
        Creeper,
        Zombie,
        Husk,
        Drowned,
        Skeleton,
        Stray,
        Bogged,
        Spider,
        CaveSpider,
        Enderman,
        Witch,
        Arrow,
        Trident,
        Fireball,
        FallingBlock,
        Item,
        Player,
        Wolf,
        IronGolem,
        Villager,
        Bee,
        ZombifiedPiglin,
        Passive,
        Other,
    ]
};
