use super::*;
use std::collections::BTreeSet;

fn attribute_keys(attributes: Vec<KeyValue>) -> BTreeSet<String> {
    attributes
        .into_iter()
        .map(|attribute| attribute.key.to_string())
        .collect()
}

#[test]
fn external_model_values_never_become_telemetry_attributes() {
    let source = SurveyTelemetrySource {
        node_id: "source-node".into(),
        node_role: "client".into(),
    };
    let lifecycle = SurveyAttributes::from_disabled_spec(SurveyModelSpec {
        model: "https://user:secret@example.test/private/model.gguf?token=leaked",
        model_path: None,
        launch_kind: SurveyLaunchKind::Startup,
        pinned_gpu: None,
        backend: None,
        context_length: None,
    });
    let request = RequestAttributes::from_request(
        Some("org/private-model:variant"),
        2,
        RequestOutcome::Success(RequestService::Remote),
        source.clone(),
    );
    let attempt = RouteAttemptAttributes::from_attempt(
        Some("/private/models/secret.gguf"),
        &AttemptTarget::Endpoint("https://private-endpoint.example/v1".into()),
        AttemptOutcome::Rejected,
        source,
    );

    for keys in [
        attribute_keys(lifecycle.key_values(None)),
        attribute_keys(request.key_values()),
        attribute_keys(attempt.key_values()),
    ] {
        assert!(!keys.contains("mesh_llm.model"));
    }
}
