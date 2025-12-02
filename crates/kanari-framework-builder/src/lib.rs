//! Minimal library target for the kanari-framework-builder crate.
/// This crate primarily uses a build script (`build.rs`).
/// Cargo requires at least one library or binary target, so provide a small
/// no-op library here.
pub fn __kanari_framework_builder_noop() {
    // intentionally empty
}
