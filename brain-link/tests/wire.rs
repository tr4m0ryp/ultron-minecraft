use brain_link::wire::{entity_labels, parse_entity, GoalName, Request, ALL_KINDS};
use serde_json::json;

#[test]
fn every_entity_label_parses_back() {
    for k in ALL_KINDS {
        assert_eq!(parse_entity(k.label()), Ok(k));
    }
    assert_eq!(entity_labels().len(), 29);
    assert!(parse_entity("Cave Spider").is_ok());
}

#[test]
fn every_goal_name_is_on_the_wire() {
    for g in GoalName::ALL {
        let req: Request =
            serde_json::from_value(json!({"op":"set_goal","goal":g.label()})).unwrap();
        assert!(matches!(req, Request::SetGoal { goal, .. } if goal == g));
    }
}

#[test]
fn requests_round_trip_through_json() {
    let lines = [
        json!({"op":"observe"}),
        json!({"op":"wait_event","timeout_s":5.0}),
        json!({"op":"set_goal","goal":"mine","params":{"kind":"iron_ore","count":3},"priority":0.5}),
        json!({"op":"set_mode","mode":"idle_safe"}),
        json!({"op":"set_gain","gain":"hp_floor","value":10.0}),
        json!({"op":"inhibit","reflex":"R5","scope":{"kind":"region","min":[0.0,0.0,0.0],"max":[1.0,2.0,3.0]},"ttl_s":60.0}),
        json!({"op":"arm","trigger":{"kind":"chat_matches","pattern":"help"},"response":{"kind":"retreat_to","pos":[1.0,2.0,3.0]},"once":false}),
        json!({"op":"say","text":"hi"}),
        json!({"op":"recent_chat","limit":4}),
        json!({"op":"log","page":1,"page_size":10}),
    ];
    for l in lines {
        let req: Request = serde_json::from_value(l.clone()).unwrap();
        assert_eq!(serde_json::to_value(&req).unwrap(), l);
    }
}
