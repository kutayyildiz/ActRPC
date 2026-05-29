use actrpc_core::interception::InterceptionPhase;

#[test]
fn test_phase_serde() {
    assert_eq!(
        serde_json::to_string(&InterceptionPhase::Outbound).unwrap(),
        "\"outbound\""
    );
    assert_eq!(
        serde_json::to_string(&InterceptionPhase::Inbound).unwrap(),
        "\"inbound\""
    );

    let de_out: InterceptionPhase = serde_json::from_str("\"outbound\"").unwrap();
    assert_eq!(de_out, InterceptionPhase::Outbound);

    let de_in: InterceptionPhase = serde_json::from_str("\"inbound\"").unwrap();
    assert_eq!(de_in, InterceptionPhase::Inbound);
}
