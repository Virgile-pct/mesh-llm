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

#[test]
fn unloading_one_model_preserves_another_loaded_model_gauge_series() {
    use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
    use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader};

    // Given: two different models with otherwise identical bounded lifecycle attributes.
    let model_attributes = |model| {
        SurveyAttributes::from_disabled_spec(SurveyModelSpec {
            model,
            model_path: None,
            launch_kind: SurveyLaunchKind::Startup,
            pinned_gpu: None,
            backend: Some("skippy"),
            context_length: Some(16_384),
        })
    };
    let first = model_attributes("org/model-a@main:model-a.gguf");
    let second = model_attributes("org/model-b@main:model-b.gguf");
    let exporter = InMemoryMetricExporter::default();
    let provider = SdkMeterProvider::builder()
        .with_reader(PeriodicReader::builder(exporter.clone()).build())
        .build();
    let mut recorder = SurveyRecorder::new(provider);

    // When: both models load and only the first model unloads.
    recorder.record(SurveyEvent::LaunchSuccess {
        attrs: first.clone(),
        duration_ms: 1.0,
    });
    recorder.record(SurveyEvent::LaunchSuccess {
        attrs: second,
        duration_ms: 1.0,
    });
    recorder.record(SurveyEvent::Unload {
        attrs: first,
        uptime_s: 1.0,
    });
    recorder._provider.force_flush().expect("metric flush");

    // Then: one series is unloaded while the other remains loaded.
    let exported = exporter.get_finished_metrics().expect("exported metrics");
    let mut loaded_values = exported
        .iter()
        .flat_map(|resource| resource.scope_metrics())
        .flat_map(|scope| scope.metrics())
        .filter(|metric| metric.name() == "mesh_llm_model_loaded")
        .flat_map(|metric| match metric.data() {
            AggregatedMetrics::U64(MetricData::Gauge(gauge)) => {
                gauge.data_points().map(|point| point.value()).collect()
            }
            _ => Vec::new(),
        })
        .collect::<Vec<_>>();
    loaded_values.sort_unstable();
    assert_eq!(loaded_values, vec![0, 1]);
}
