#[test]
fn test_acp_relay_loc_within_limit() {
    let line_count = include_str!("acp_relay.rs").lines().count();

    assert!(
        line_count <= 300,
        "acp_relay.rs has {line_count} lines; expected <= 300"
    );
}

#[test]
fn test_acp_relay_session_loc_within_limit() {
    let line_count = include_str!("acp_relay_session.rs").lines().count();

    assert!(
        line_count <= 300,
        "acp_relay_session.rs has {line_count} lines; expected <= 300"
    );
}

#[test]
fn test_mod_rs_exports_new_modules() {
    let mod_rs = include_str!("mod.rs");

    assert!(
        mod_rs.contains("acp_relay_lifecycle"),
        "mod.rs must export acp_relay_lifecycle for extracted relay lifecycle code"
    );
}
