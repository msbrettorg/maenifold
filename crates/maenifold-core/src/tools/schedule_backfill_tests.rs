use super::*;

#[test]
fn backfill_unknown_schedule_returns_invalid_argument() {
    let (_t, c) = ctx();
    let result = schedule(
        &c,
        &json!({"action":"backfill","id":"missing","since":"2026-05-10T00:00:00Z"}),
    );
    assert!(matches!(
        result,
        Err(MaenifoldError::InvalidArgument(message))
            if message == "unknown scheduleId 'missing'"
    ));
}

#[test]
fn backfill_no_missing_occurrences_returns_zero_executed() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", false);
    let payload = parse(
        &schedule(
            &c,
            &json!({
                "action":"backfill",
                "id":"alpha",
                "since":"2026-05-10T00:00:00Z",
                "until":"2026-05-10T00:00:00Z"
            }),
        )
        .expect("backfill"),
    );
    assert_eq!(payload["plannedCount"], 0);
    assert_eq!(payload["executedCount"], 0);
    assert_eq!(payload["skippedCount"], 0);
    assert_eq!(payload["runIds"], json!([]));
}

#[test]
fn backfill_writes_placeholder_runs_for_missing_occurrences() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", false);
    let payload = parse(
        &schedule(
            &c,
            &json!({
                "action":"backfill",
                "id":"alpha",
                "since":"2026-05-10T00:00:00Z",
                "until":"2026-05-12T00:00:00Z"
            }),
        )
        .expect("backfill"),
    );

    assert_eq!(payload["plannedCount"], 2);
    assert_eq!(payload["executedCount"], payload["plannedCount"]);
    assert_eq!(payload["skippedCount"], 0);
    assert_eq!(
        payload["runIds"].as_array().expect("runIds").len(),
        payload["executedCount"].as_u64().expect("executed") as usize
    );
    for run_id in payload["runIds"].as_array().expect("runIds") {
        let path = schedule_store(&c)
            .task_dir("alpha")
            .join("logs")
            .join(run_id.as_str().expect("run id"));
        assert!(path.exists(), "backfill should write placeholder log");
    }
}

#[test]
fn backfill_includes_occurrence_exactly_at_since() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", false);
    let payload = parse(
        &schedule(
            &c,
            &json!({
                "action":"backfill",
                "id":"alpha",
                "since":"2026-05-10T09:00:00Z",
                "until":"2026-05-10T10:00:00Z"
            }),
        )
        .expect("backfill"),
    );

    assert_eq!(payload["plannedCount"], 1);
    assert_eq!(payload["executedCount"], 1);
    assert_eq!(payload["skippedCount"], 0);
    assert_eq!(payload["runIds"], json!(["20260510T090000Z.log"]));
}

#[test]
fn backfill_idempotent_skips_existing_runs() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", false);
    let payload = json!({
        "action":"backfill",
        "id":"alpha",
        "since":"2026-05-10T00:00:00Z",
        "until":"2026-05-12T00:00:00Z"
    });
    schedule(&c, &payload).expect("first backfill");
    let second = parse(&schedule(&c, &payload).expect("second backfill"));

    assert_eq!(second["plannedCount"], 2);
    assert_eq!(second["skippedCount"], second["plannedCount"]);
    assert_eq!(second["executedCount"], 0);
    assert_eq!(second["runIds"], json!([]));
}
