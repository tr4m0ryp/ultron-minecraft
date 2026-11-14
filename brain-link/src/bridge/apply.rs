//! Applies one queued request to the executor on the tick thread.

use reflex_core::knobs::{Response, Trigger};
use reflex_core::Snapshot;
use serde_json::Value;

use super::args::{reflex_ids, scope_of, ticks};
use super::chat_gate::ChatGate;
use super::{goals, log, observe};
use crate::host::Host;
use crate::wire::{self, mode_label, Ack, Request};

pub fn run<H: Host>(
    req: Request,
    host: &mut H,
    snap: &Snapshot,
    chat: (&mut Vec<String>, &mut ChatGate),
    wakes_pending: usize,
) -> Value {
    match apply(req, host, snap, chat, wakes_pending) {
        Ok(v) => wire::ok(v),
        Err(e) => wire::err(e),
    }
}

fn apply<H: Host>(
    req: Request,
    host: &mut H,
    snap: &Snapshot,
    (say, gate): (&mut Vec<String>, &mut ChatGate),
    wakes_pending: usize,
) -> Result<Value, String> {
    let now = snap.tick;
    let ack = match req {
        Request::Observe => {
            let cap = host.knobs().report_tokens;
            let digest = host.upward().take_digest(cap);
            let owner = host.legs_owner();
            let status = host.goal_status();
            let obs = observe::observe(
                host.knobs(),
                owner.as_deref(),
                status.as_ref(),
                snap,
                digest.text,
                wakes_pending,
            );
            return Ok(serde_json::to_value(obs).expect("observation serialises"));
        }
        Request::SetGoal {
            goal,
            params,
            priority,
        } => goals::set_goal(host.knobs(), snap, goal, &params, priority)?,
        Request::SetMode { mode } => {
            host.knobs().set_mode(mode.into());
            Ack {
                id: None,
                done: format!(
                    "mode {}; gains reset to its preset",
                    mode_label(mode.into())
                ),
            }
        }
        Request::SetGain { gain, value } => {
            if !value.is_finite() {
                return Err("value must be a number".into());
            }
            host.knobs().set_gain(gain.into(), value);
            Ack {
                id: None,
                done: format!("{} = {}", label(&gain), gain_value(host, gain)),
            }
        }
        Request::Inhibit {
            reflex,
            scope,
            strength,
            ttl_s,
        } => {
            let reflex = reflex.trim().to_ascii_uppercase();
            if !reflex_ids().contains(&reflex.as_str()) {
                return Err(format!(
                    "unknown reflex '{reflex}'; known: {}",
                    reflex_ids().join(" ")
                ));
            }
            let scope = scope_of(scope, host, snap)?;
            let ttl = ticks(ttl_s)?;
            let strength = strength.unwrap_or(1.0);
            let id = host.knobs().inhibit(&reflex, scope, strength, ttl, now);
            Ack {
                id: Some(id),
                done: format!("inhibit #{id} {reflex}"),
            }
        }
        Request::Arm {
            trigger,
            response,
            ttl_s,
            once,
        } => {
            let trigger = Trigger::try_from(trigger)?;
            let response = Response::try_from(response)?;
            let ttl = ticks(ttl_s)?;
            let id = host
                .knobs()
                .arm(trigger, response, ttl, once.unwrap_or(true), now);
            Ack {
                id: Some(id),
                done: format!("armed #{id}"),
            }
        }
        Request::Say { text } => {
            let line = wire::chat_line(&text)?;
            gate.admit(&line, now)?;
            say.push(line);
            Ack {
                id: None,
                done: "sent".into(),
            }
        }
        Request::Log { page, page_size } => return Ok(log::page(host, page, page_size)),
        Request::WaitEvent { .. } | Request::RecentChat { .. } => {
            return Err("answered by the socket thread, not the tick loop".into())
        }
    };
    Ok(serde_json::to_value(ack).expect("ack serialises"))
}

fn label(g: &wire::WireGain) -> String {
    serde_json::to_value(g)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn gain_value<H: Host>(host: &mut H, g: wire::WireGain) -> String {
    use wire::WireGain::*;
    let gains = &host.knobs().gains;
    match g {
        Aggression => gains.aggression.to_string(),
        Caution => gains.caution.to_string(),
        HpFloor => gains.hp_floor.to_string(),
        HungerFloor => gains.hunger_floor.to_string(),
        Leash => gains.leash.to_string(),
        Vigilance => gains.vigilance.to_string(),
        AirFloor => gains.air_floor.to_string(),
    }
}
