use neohub::LiveData;

#[test]
fn live_data() {
    assert_eq!(
        serde_json::from_str::<LiveData>(include_str!("live-data-1.json"))
            .unwrap()
            .devices
            .len(),
        6
    );
}
