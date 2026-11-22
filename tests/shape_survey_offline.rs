//! Offline unit tests of the shape survey's pure code (sample selection and
//! redaction). The live target `shape_survey` is `test = false`, so its unit
//! tests only run here; this target needs no network, no token and no files
//! outside the crate.
#![allow(
    dead_code,
    clippy::unwrap_used,
    reason = "the survey modules are shared with the live probe; this target compiles only their pure parts' tests"
)]

#[path = "shape_survey/sample.rs"]
mod sample;
#[path = "common/scrub.rs"]
mod scrub;
#[path = "shape_survey/shape.rs"]
mod shape;
