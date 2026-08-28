//! Permanent contract test for issue #1462 PR 1: static
//! `mesh-llm config validate` must produce a diagnostic for every manifest
//! entry whose wiring status is not `Wired`.
//!
//! `model_fit.keep_tokens` is a `basic_setting` (fully `Supported`) on the
//! built-in config schema, and static validation accepts any positive
//! integer for it. But the embedded staged runtime resolver
//! (`reject_unsupported_model_fit_controls` in
//! `crates/mesh-llm-host-runtime/src/inference/skippy/resolver/support.rs`)
//! bails at model load for any positive value:
//! `if config.keep_tokens.unwrap_or(0) > 0 { bail!(...) }`. This test
//! guards that gap: as long as `model_fit.keep_tokens` remains `Unwired`
//! with a `BailsDownstream` behavior in the manifest, static
//! `mesh-llm config validate` must flag `model_fit.keep_tokens = 128` as an
//! error rather than reporting the config valid.

use mesh_llm_config::{
    ConfigDiagnosticSeverity, MeshConfig, WIRING_MANIFEST, WiringBehavior, WiringStatus,
    built_in_config_schema, validate_config_diagnostics,
};

#[test]
fn static_validation_flags_a_field_that_bails_at_model_load() {
    let config: MeshConfig = toml::from_str(
        r#"
[defaults.model_fit]
keep_tokens = 128
"#,
    )
    .expect("config should parse");

    let diagnostics = validate_config_diagnostics(&config);
    let has_keep_tokens_error = diagnostics.iter().any(|diagnostic| {
        diagnostic
            .path
            .as_ref()
            .is_some_and(|path| path.render() == "defaults.model_fit.keep_tokens")
            && diagnostic.severity == ConfigDiagnosticSeverity::Error
    });

    assert!(
        has_keep_tokens_error,
        "model_fit.keep_tokens > 0 fails at model load in the embedded staged runtime \
         (reject_unsupported_model_fit_controls in \
         mesh-llm-host-runtime/src/inference/skippy/resolver/support.rs), but static \
         `mesh-llm config validate` reported no error for it: {diagnostics:#?}"
    );
}

#[test]
fn fit_target_mib_manifest_matches_its_tuner_and_live_resolver_consumers() {
    let entry = WIRING_MANIFEST
        .iter()
        .find(|entry| entry.path == "hardware.fit_target_mib")
        .expect("fit_target_mib must remain in the exhaustive wiring manifest");

    assert_eq!(entry.status, WiringStatus::Wired);
    assert_eq!(entry.behavior, WiringBehavior::None);
}

#[test]
fn pr4_supported_fields_do_not_emit_unsupported_diagnostics() {
    let config: MeshConfig = toml::from_str(
        r#"
[defaults.throughput]
continuous_batching = false
tuning_profile = "saver"

[defaults.skippy]
lifecycle_startup_timeout_ms = 120000
lifecycle_readiness_interval_ms = 125
lifecycle_health_interval_ms = 5000
"#,
    )
    .expect("config should parse");

    let diagnostics = validate_config_diagnostics(&config);

    assert!(
        diagnostics.is_empty(),
        "wired PR4 fields must pass static validation without unsupported warnings: {diagnostics:#?}"
    );
}

#[test]
fn pr4_fields_without_runtime_consumers_are_rejected() {
    let config: MeshConfig = toml::from_str(
        r#"
[defaults.throughput]
priority = "normal"
poll = "busy"
cpu_affinity = "0-3"
numa = "distribute"
slot_prompt_similarity = 0.75

[defaults.skippy]
stage_model_path = "/models/stage.gguf"
stage_role = "middle"
stage_topology = "legacy-lock"
binary_stage_transport = "binary"
"#,
    )
    .expect("config should parse");

    let diagnostics = validate_config_diagnostics(&config);
    let rejected = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == ConfigDiagnosticSeverity::Error)
        .filter_map(|diagnostic| diagnostic.path.as_ref().map(|path| path.render()))
        .collect::<std::collections::BTreeSet<_>>();

    let expected = std::collections::BTreeSet::from([
        "defaults.skippy.binary_stage_transport".to_string(),
        "defaults.skippy.stage_model_path".to_string(),
        "defaults.skippy.stage_role".to_string(),
        "defaults.skippy.stage_topology".to_string(),
        "defaults.throughput.cpu_affinity".to_string(),
        "defaults.throughput.numa".to_string(),
        "defaults.throughput.poll".to_string(),
        "defaults.throughput.priority".to_string(),
        "defaults.throughput.slot_prompt_similarity".to_string(),
    ]);
    assert_eq!(rejected, expected);
}

#[test]
fn prompt_shape_metrics_are_accepted_for_bounded_otlp_export() {
    let config: MeshConfig = toml::from_str(
        r#"
[telemetry]
prompt_shape_metrics = true
"#,
    )
    .expect("config should parse");

    let diagnostics = validate_config_diagnostics(&config);

    assert!(
        diagnostics.is_empty(),
        "prompt-shape metrics must pass static validation once their bounded exporter is wired: {diagnostics:#?}"
    );
}

#[test]
fn wiring_manifest_covers_every_builtin_schema_path_in_both_directions() {
    let manifest = WIRING_MANIFEST
        .iter()
        .map(|entry| entry.path.trim_end_matches(".*"))
        .collect::<std::collections::BTreeSet<_>>();
    let schema = built_in_config_schema();
    let mut normalized = schema
        .settings
        .iter()
        .map(|setting| {
            let path = setting
                .path
                .render()
                .replace("plugin.<plugin-name>", "plugin.<name>");
            path.strip_prefix("defaults.")
                .or_else(|| path.strip_prefix("models.<model-ref>."))
                .unwrap_or(&path)
                .trim_end_matches(".*")
                .to_string()
        })
        .collect::<std::collections::BTreeSet<_>>();
    normalized.insert("plugin.<name>.settings".to_string());

    let missing = normalized
        .iter()
        .filter(|path| !manifest.contains(path.as_str()))
        .collect::<Vec<_>>();
    let stale = manifest
        .iter()
        .filter(|path| !normalized.contains(**path))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "schema paths missing from wiring manifest: {missing:?}"
    );
    assert!(
        stale.is_empty(),
        "wiring manifest paths missing from schema: {stale:?}"
    );
}

#[test]
fn native_mmap_controls_and_tensor_split_mode_pass_static_validation() {
    let config: MeshConfig = toml::from_str(
        r#"
[defaults.hardware]
use_mmap_prefetch = true
use_mmap_buffer = true
split_mode = "tensor"
"#,
    )
    .expect("documented native controls must parse");

    let diagnostics = validate_config_diagnostics(&config);

    assert!(
        diagnostics.is_empty(),
        "documented native controls must validate before runtime resolution: {diagnostics:#?}"
    );
}

#[test]
fn closeout_audit_names_every_manifest_row_and_required_boundary() {
    let audit_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/CONFIGURATION_PR8_CLOSEOUT_AUDIT.md");
    let audit = std::fs::read_to_string(&audit_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", audit_path.display()));

    for entry in WIRING_MANIFEST {
        assert!(
            audit.contains(&format!("`{}`", entry.path)),
            "closeout audit is missing manifest path {}",
            entry.path
        );
    }
    for boundary in [
        "parsed",
        "validated",
        "final consumer",
        "reverse audit",
        "hardware limitation",
    ] {
        assert!(
            audit.to_ascii_lowercase().contains(boundary),
            "closeout audit is missing required evidence boundary {boundary:?}"
        );
    }
}
