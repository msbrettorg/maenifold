mod support;

use serde_json::json;
use support::read_authoritative_asset;

#[test]
fn builtin_sdd_definition_matches_authoritative_asset() {
    let actual = read_authoritative_asset("sdd");
    let expected = json!({
        "schemaVersion": 1,
        "id": "sdd",
        "name": "Spec Driven Development",
        "initial": "pending",
        "states": {
            "pending": { "on": { "pass": "swe" } },
            "swe": { "on": { "fail": "pending", "pass": "red" } },
            "red": { "on": { "fail": "pending", "pass": "blue" } },
            "blue": { "on": { "fail": "pending", "pass": "done" } },
            "done": { "terminal": true }
        }
    });

    assert_eq!(actual, expected);
}

#[test]
fn builtin_gsd_definition_matches_authoritative_asset() {
    let actual = read_authoritative_asset("gsd");
    assert_eq!(actual["schemaVersion"], 1);
    assert_eq!(actual["id"], "gsd");
    assert_eq!(actual["initial"], "pending");
    assert_eq!(actual["states"]["pending"]["on"]["pass"], "research");
    assert_eq!(actual["states"]["verify"]["on"]["fail"], "execute");
    assert_eq!(actual["states"]["done"]["terminal"], true);
}

#[test]
fn builtin_lightweight_definition_matches_authoritative_asset() {
    let actual = read_authoritative_asset("lightweight");
    assert_eq!(actual["schemaVersion"], 1);
    assert_eq!(actual["id"], "lightweight");
    assert_eq!(actual["states"]["pending"]["on"]["pass"], "swe");
    assert_eq!(actual["states"]["swe"]["on"]["pass"], "blue");
    assert_eq!(actual["states"]["blue"]["on"]["pass"], "done");
}

#[test]
fn builtin_tdd_definition_matches_authoritative_asset() {
    let actual = read_authoritative_asset("tdd");
    assert_eq!(actual["schemaVersion"], 1);
    assert_eq!(actual["id"], "tdd");
    assert_eq!(actual["name"], "Test Driven Development");
    assert_eq!(actual["states"]["pending"]["on"]["pass"], "swe");
    assert_eq!(actual["states"]["red"]["on"]["pass"], "blue");
}
