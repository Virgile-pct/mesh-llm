//! Translate Mesh configuration and policy into Skippy preparation inputs.
//! Inference, model lifecycle and serving remain owned by Skippy.
pub mod config;
pub mod hash_cache;
mod load_options;
pub mod package;
mod stage;
pub use load_options::{SkippyDeviceDescriptor, SkippyModelLoadOptions, SkippyTelemetryOptions};
pub use skippy_api::family_policy;
pub use skippy_api::family_policy::family_policy_for_model_path;
pub use skippy_api::kv_cache::{KvCachePolicy, KvCacheType};
pub use skippy_api::package::SkippyPackageIdentity;
pub use stage::single_stage_config;

/// Mesh's application cache, `<platform cache>/mesh-llm`, handed to Skippy
/// explicitly. Skippy's `application_cache_dir()` is `<platform cache>/skippy`,
/// so delegating to it would move model usage, metadata, weight digests, the
/// blobstore and materialized stages away from where earlier releases left
/// them. `mesh_llm_system::native_runtime_install::mesh_cache_dir` resolves
/// the same directory.
pub fn mesh_cache_dir() -> std::path::PathBuf {
    dirs::cache_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join(".cache")))
        .unwrap_or_else(|| std::env::temp_dir().join("mesh-llm-cache"))
        .join("mesh-llm")
}

pub fn synthetic_direct_gguf_package(
    model_id: &str,
    model_path: &std::path::Path,
) -> anyhow::Result<SkippyPackageIdentity> {
    skippy_api::source::synthetic_direct_gguf_package(model_id, model_path)
}

pub use skippy_api::serving::readiness;
