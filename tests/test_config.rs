use automode::config::{load_from_str, save_to_string, Config, Target};

#[test]
fn test_default_config_roundtrip() {
    let cfg = Config::default();
    let serialized = save_to_string(&cfg).unwrap();
    let parsed = load_from_str(&serialized).unwrap();
    assert_eq!(parsed.port, 7878);
    assert_eq!(parsed.target, Target::Claude);
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
    assert_eq!(cfg.target, Target::Claude);
}

#[test]
fn test_load_codex_target() {
    let toml = r#"port = 9000
target = "codex"
mode = "mild"
model_path = "/tmp/model.gguf"
llama_server_bin = "/tmp/llama-server"
llama_server_port = 8080
log_level = "debug"
"#;
    let cfg = load_from_str(toml).unwrap();
    assert_eq!(cfg.target, Target::Codex);
}

#[test]
fn test_load_both_target() {
    let toml = r#"port = 9000
target = "both"
mode = "mild"
model_path = "/tmp/model.gguf"
llama_server_bin = "/tmp/llama-server"
llama_server_port = 8080
log_level = "debug"
"#;
    let cfg = load_from_str(toml).unwrap();
    assert_eq!(cfg.target, Target::Both);
}

#[test]
fn test_load_antigravity_target() {
    let toml = r#"port = 9000
target = "antigravity"
mode = "mild"
model_path = "/tmp/model.gguf"
llama_server_bin = "/tmp/llama-server"
llama_server_port = 8080
log_level = "debug"
"#;
    let cfg = load_from_str(toml).unwrap();
    assert_eq!(cfg.target, Target::Antigravity);
}

#[test]
fn test_load_all_target() {
    let toml = r#"port = 9000
target = "all"
mode = "mild"
model_path = "/tmp/model.gguf"
llama_server_bin = "/tmp/llama-server"
llama_server_port = 8080
log_level = "debug"
"#;
    let cfg = load_from_str(toml).unwrap();
    assert_eq!(cfg.target, Target::All);
}
