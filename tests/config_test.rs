use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_load_config_from_file() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, r#"
[general]
data_dir = "/tmp/impactvault-test"

[enforcer]
enabled = true
default_action = "block"
"#).unwrap();

    let config = impactvault::config::Config::load(f.path()).unwrap();
    assert_eq!(config.general.data_dir, "/tmp/impactvault-test");
    assert!(config.enforcer.enabled);
}

#[test]
fn test_config_defaults() {
    let config = impactvault::config::Config::default();
    assert!(config.enforcer.enabled);
    assert_eq!(config.enforcer.default_action, "block");
    assert_eq!(config.general.data_dir, "~/.impactvault");
}
