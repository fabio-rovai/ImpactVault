use impactvault::domain::dpga::{DpgEntry, suggest_recipients};

#[test]
fn test_dpg_entry_creation() {
    let entry = DpgEntry {
        name: "DHIS2".into(),
        description: "Health management platform".into(),
        website: "https://dhis2.org".into(),
        repositories: vec!["https://github.com/dhis2/dhis2-core".into()],
        stage: "DPG".into(),
        wallet_address: Some("0x1234".into()),
    };
    assert_eq!(entry.name, "DHIS2");
    assert!(entry.wallet_address.is_some());
}

#[test]
fn test_suggest_recipients_filters_active() {
    let dpgs = vec![
        DpgEntry {
            name: "Active DPG".into(),
            description: "Has repo and wallet".into(),
            website: "https://example.com".into(),
            repositories: vec!["https://github.com/org/repo".into()],
            stage: "DPG".into(),
            wallet_address: Some("0xabc".into()),
        },
        DpgEntry {
            name: "No Wallet".into(),
            description: "Missing wallet".into(),
            website: "https://example2.com".into(),
            repositories: vec!["https://github.com/org/repo2".into()],
            stage: "DPG".into(),
            wallet_address: None,
        },
        DpgEntry {
            name: "No Repos".into(),
            description: "Empty repos".into(),
            website: "https://example3.com".into(),
            repositories: vec![],
            stage: "DPG".into(),
            wallet_address: Some("0xdef".into()),
        },
    ];
    let suggested = suggest_recipients(&dpgs);
    assert_eq!(suggested.len(), 1);
    assert_eq!(suggested[0].name, "Active DPG");
}
