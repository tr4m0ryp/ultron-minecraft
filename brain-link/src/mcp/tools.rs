//! Tool definitions shown to Ultron's thought layer in the game. Descriptions say what a tool is for and
//! when to call it; schemas mirror `wire::Request` exactly.

use serde_json::{json, Value};

use crate::bridge::{reflex_ids, SUPPORTED_GOALS};
use crate::wire::{entity_labels, GoalName};

pub const TOOL_NAMES: [&str; 10] = [
    "observe",
    "wait_event",
    "set_goal",
    "set_mode",
    "set_gain",
    "inhibit",
    "arm",
    "say",
    "recent_chat",
    "log",
];

const REFLEXES: &str = "R1 lava, R3 put out fire, R4 contact hazard, R5 edge guard (no deep drops), R7 void, R8 surface for air, R9 suffocation, R10 dig guard, R11 totem, R14 dodge projectiles, D1 creeper distance, D2 melee defence, D3 ranged cover, D5 enderman gaze, D11 never hit friends, H1 eat, H2 retreat at low hp, H8 stay near companion, O1 look at speaker, O2 looming, C6 unstuck";

fn tool(name: &str, description: String, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        },
    })
}

fn xyz() -> Value {
    json!({"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3})
}

pub fn tool_list() -> Vec<Value> {
    let goals: Vec<&str> = GoalName::ALL.iter().map(|g| g.label()).collect();
    let runnable: Vec<&str> = SUPPORTED_GOALS.iter().map(|g| g.label()).collect();
    let entities = entity_labels();
    let entity = json!({"type": "string", "enum": entities});
    vec![
        tool("observe", "Your current state: position, hp, food, held item, day or night, nearby entities, companion distance, goal and its status, and a digest of what your reflexes did since the last observe. Call it when you need to plan, not every turn.".into(), json!({}), &[]),
        tool("wait_event", "Block until something needs you (chat to you, goal reached or interrupted, danger, an armed response fired, a reflex overridden) or the timeout passes. This is your idle loop: call it whenever you have nothing to decide.".into(), json!({
            "timeout_s": {"type": "number", "minimum": 0, "maximum": 300, "default": 30},
        }), &[]),
        tool("set_goal", format!("Replace your active goal (the old one is queued, up to 3). The body walks and fights on its own; never micromanage movement. Runnable now: {}. Others return an error until supported. end: drop the active goal. follow: stay within distance of the companion. go_to: walk to pos. come_here: walk to where the companion is now. stay: hold this spot.", runnable.join(", ")), json!({
            "goal": {"type": "string", "enum": goals},
            "params": {"type": "object", "additionalProperties": false, "properties": {
                "companion": {"type": "string", "description": "player name, default your companion"},
                "distance": {"type": "number", "exclusiveMinimum": 0},
                "pos": xyz(),
                "kind": {"type": "string", "description": "block, item or entity id, e.g. iron_ore"},
                "count": {"type": "integer", "minimum": 1},
            }},
            "priority": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.5},
        }), &["goal"]),
        tool("set_mode", "Switch the body's temperament preset; resets all gains to that preset. fight: bold, mine and build: careful, retreat: avoid combat, follow: tight leash, idle_safe: cautious standby.".into(), json!({
            "mode": {"type": "string", "enum": ["explore", "mine", "build", "fight", "retreat", "follow", "idle_safe"]},
        }), &["mode"]),
        tool("set_gain", "Tune one reflex gain. aggression, caution, vigilance: 0..1. hp_floor: retreat below this hp (0..20). hunger_floor: eat at or below this food (0..20). leash: max blocks from companion, 0 disables. air_floor: surface below this air (0..300 ticks).".into(), json!({
            "gain": {"type": "string", "enum": ["aggression", "caution", "hp_floor", "hunger_floor", "leash", "vigilance", "air_floor"]},
            "value": {"type": "number"},
        }), &["gain", "value"]),
        tool("inhibit", format!("Switch a reflex off where it gets in the way, e.g. R5 while building on a cliff. Lethal danger still overrides and wakes you. Reflexes: {REFLEXES}. Scope everywhere (default), region (min/max corners), here (box of radius around you) or until_goal_ends."), json!({
            "reflex": {"type": "string", "enum": reflex_ids()},
            "scope": {"type": "object", "additionalProperties": false, "required": ["kind"], "properties": {
                "kind": {"type": "string", "enum": ["everywhere", "region", "here", "until_goal_ends"]},
                "min": xyz(), "max": xyz(),
                "radius": {"type": "number", "exclusiveMinimum": 0},
            }},
            "strength": {"type": "number", "minimum": 0, "maximum": 1, "default": 1},
            "ttl_s": {"type": "number", "exclusiveMinimum": 0, "description": "omit for no expiry"},
        }), &["reflex"]),
        tool("arm", "Prepare an if-then response the body runs instantly without asking you. Arm ahead of predictable danger, e.g. creeper near companion: place_block_between creeper.".into(), json!({
            "trigger": {"type": "object", "additionalProperties": false, "required": ["kind"], "properties": {
                "kind": {"type": "string", "enum": ["entity_within", "hp_below", "companion_hp_below", "chat_matches"]},
                "entity": entity.clone(), "dist": {"type": "number", "exclusiveMinimum": 0},
                "of_companion": {"type": "boolean", "description": "measure dist from the companion"},
                "hp": {"type": "number"}, "pattern": {"type": "string", "description": "case-insensitive substring"},
            }},
            "response": {"type": "object", "additionalProperties": false, "required": ["kind"], "properties": {
                "kind": {"type": "string", "enum": ["say", "retreat_to", "flee_from", "attack", "place_block_between"]},
                "text": {"type": "string", "maxLength": 256}, "pos": xyz(), "entity": entity,
            }},
            "ttl_s": {"type": "number", "exclusiveMinimum": 0},
            "once": {"type": "boolean", "default": true},
        }), &["trigger", "response"]),
        tool("say", "Send one chat line. Keep it short and natural.".into(), json!({
            "text": {"type": "string", "minLength": 1, "maxLength": 256},
        }), &["text"]),
        tool("recent_chat", "The last chat lines heard, oldest first. Use it to catch up on a conversation.".into(), json!({
            "limit": {"type": "integer", "minimum": 1, "maximum": 50, "default": 10},
        }), &[]),
        tool("log", "Full event log, newest first, paged. Only for debugging what the body did.".into(), json!({
            "page": {"type": "integer", "minimum": 0, "default": 0},
            "page_size": {"type": "integer", "minimum": 1, "maximum": 100, "default": 20},
        }), &[]),
    ]
}
