use actrpc_core::phase::Phase;

#[test]
fn test_phase_serde() {
    assert_eq!(
        serde_json::to_string(&Phase::Outbound).unwrap(),
        "\"outbound\""
    );
    assert_eq!(
        serde_json::to_string(&Phase::Inbound).unwrap(),
        "\"inbound\""
    );

    let de_out: Phase = serde_json::from_str("\"outbound\"").unwrap();
    assert_eq!(de_out, Phase::Outbound);

    let de_in: Phase = serde_json::from_str("\"inbound\"").unwrap();
    assert_eq!(de_in, Phase::Inbound);
}
