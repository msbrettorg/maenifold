use super::*;

#[test]
fn teams_identity_maps_graph_preferred_data_locations_to_chatsvc_regions() {
    for code in ["NAM", "CAN", "BRA"] {
        assert_eq!(map_preferred_data_location(code).as_deref(), Some("amer"));
    }
    for code in [
        "EUR", "FRA", "CHE", "DEU", "GBR", "NOR", "ZAF", "ARE", "SWE",
    ] {
        assert_eq!(map_preferred_data_location(code).as_deref(), Some("emea"));
    }
    for code in ["APC", "AUS", "JPN", "IND", "KOR", "SGP"] {
        assert_eq!(map_preferred_data_location(code).as_deref(), Some("apac"));
    }
    assert_eq!(map_preferred_data_location("unknown"), None);
}

#[tokio::test]
#[ignore = "requires local Azure CLI sign-in and Graph access"]
async fn teams_identity_live_graph_preferred_data_location_is_runnable_locally() {
    let _ = detect_chatsvc_region_via_graph().await;
}
