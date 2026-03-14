use actrpc_core::participant::{Participant, ParticipantType};

#[test]
fn test_participant_serde() {
    let p = Participant {
        kind: ParticipantType::Interceptor,
        id: "safety-v3".to_string(),
    };

    let ser = serde_json::to_string(&p).unwrap();
    let expected = r#"{"kind":"interceptor","id":"safety-v3"}"#;
    assert_eq!(ser, expected);

    let de: Participant = serde_json::from_str(expected).unwrap();
    assert_eq!(de, p);
}
