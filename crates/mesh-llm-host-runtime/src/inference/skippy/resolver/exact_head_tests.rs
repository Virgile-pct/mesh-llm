use super::*;
use crate::plugin::MeshConfig;

#[test]
fn effective_derived_profile_selects_the_matching_duplicate_model_at_runtime_boundary() {
    let config: MeshConfig = toml::from_str(
        r#"
[defaults.throughput]
threads_batch = 7

[[models]]
model = "shared/model"
[models.throughput]
threads = 3

[[models]]
model = "shared/model"
[models.throughput]
threads = 11
"#,
    )
    .expect("duplicate profile config parses");
    let mut effective = config.models[1].clone();
    effective
        .throughput
        .as_mut()
        .expect("model throughput")
        .threads_batch = config
        .defaults
        .as_ref()
        .and_then(|defaults| defaults.throughput.as_ref())
        .and_then(|throughput| throughput.threads_batch);
    let selected_profile = effective.derived_profile();
    let model_file = tempfile::NamedTempFile::new().expect("temporary model file");

    let resolved = resolve_skippy_config(SkippyConfigResolveRequest {
        mesh_config: &config,
        model_id: &selected_profile,
        model_path: model_file.path(),
        model_bytes: 1024,
        allocatable_memory_bytes: None,
        request_defaults: None,
        package_generation: None,
    })
    .expect("profile-specific config resolves");
    assert_eq!(resolved.throughput.threads, Some(11));
}
