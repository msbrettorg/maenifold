mod support;

use serde_json::json;
use support::read_authoritative_asset;

#[test]
fn builtin_sdd_definition_matches_authoritative_asset() {
    let actual = read_authoritative_asset("sdd");
    let expected = json!({
        "name": "sdd",
        "stages": [
            { "name": "swe", "verdicts": [], "transitions": { "pass": "red" } },
            { "name": "red", "verdicts": ["pass", "reject"], "transitions": { "pass": "blue", "reject": "pending" } },
            { "name": "blue", "verdicts": ["pass", "fail"], "transitions": { "pass": "done", "fail": "pending" } }
        ]
    });

    assert_eq!(actual, expected);
}

#[test]
fn builtin_gsd_definition_matches_authoritative_asset() {
    let actual = read_authoritative_asset("gsd");
    assert_eq!(actual["name"], "gsd");
    assert_eq!(actual["stages"][0]["name"], "research");
    assert_eq!(actual["stages"][1]["name"], "plan");
    assert_eq!(actual["stages"][2]["name"], "execute");
    assert_eq!(actual["stages"][3]["name"], "verify");
    assert_eq!(actual["stages"][3]["transitions"]["fail"], "execute");
}

#[test]
fn builtin_lightweight_definition_matches_authoritative_asset() {
    let actual = read_authoritative_asset("lightweight");
    assert_eq!(actual["name"], "lightweight");
    assert_eq!(actual["stages"][0]["name"], "swe");
    assert_eq!(actual["stages"][1]["name"], "blue");
    assert_eq!(actual["stages"][0]["transitions"]["pass"], "blue");
    assert_eq!(actual["stages"][1]["transitions"]["pass"], "done");
}

#[test]
fn builtin_tdd_definition_matches_authoritative_asset() {
    let actual = read_authoritative_asset("tdd");
    assert_eq!(actual["name"], "tdd");
    assert_eq!(actual["stages"][0]["name"], "swe");
    assert_eq!(actual["stages"][1]["name"], "red");
    assert_eq!(actual["stages"][2]["name"], "blue");
}

#[test]
fn authoritative_tdd_shape_is_currently_swe_red_blue() {
    let actual = read_authoritative_asset("tdd");
    let stages = actual["stages"].as_array().expect("stages should be an array");
    let names: Vec<&str> = stages
        .iter()
        .map(|stage| stage["name"].as_str().expect("stage name should be a string"))
        .collect();
    assert_eq!(names, vec!["swe", "red", "blue"]);
}

#[test]
fn stage_order_for_sdd_is_swe_then_red_then_blue() {
    let actual = read_authoritative_asset("sdd");
    let stages = actual["stages"].as_array().expect("stages should be an array");
    let names: Vec<&str> = stages
        .iter()
        .map(|stage| stage["name"].as_str().expect("stage name should be a string"))
        .collect();
    assert_eq!(names, vec!["swe", "red", "blue"]);
}

#[test]
fn gsd_fail_from_verify_retries_execute() {
    let actual = read_authoritative_asset("gsd");
    assert_eq!(actual["stages"][3]["name"], "verify");
    assert_eq!(actual["stages"][3]["transitions"]["fail"], "execute");
}

#[test]
fn lightweight_pass_from_swe_skips_to_blue() {
    let actual = read_authoritative_asset("lightweight");
    assert_eq!(actual["stages"][0]["name"], "swe");
    assert_eq!(actual["stages"][0]["transitions"]["pass"], "blue");
}
