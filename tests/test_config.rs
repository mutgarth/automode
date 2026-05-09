use automode::config::{Config, load_from_str, save_to_string};

#[test]
fn test_default_config_roundtrip() {
    let cfg = Config::default();
    let serialized = save_to_string(&cfg).unwrap();
    let parsed = load_from_str(&serialized).unwrap();
    assert_eq!(parsed.port, 7878);
    assert_eq!(parsed.llama_server_port, 8080);
}

#[test]
fn test_load_from_partial_toml() {
    let toml = r#"port = 9000
mode = "yolo"
model_path = "/tmp/model.gguf"
llama_server_bin = "/tmp/llama-server"
llama_server_port = 8080
log_level = "debug"
"#;
    let cfg = load_from_str(toml).unwrap();
    assert_eq!(cfg.port, 9000);
}
