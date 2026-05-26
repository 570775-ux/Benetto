//! JNI bridge — type conversion between Kotlin and Rust.
//!
//! # Safety
//! This module contains the only `unsafe` blocks in the codebase — limited to JNI boundary.
//! All internal logic is safe Rust.

// JNI type conversions (String ↔ JString, array conversions)
// See lib.rs for actual JNI exports.
