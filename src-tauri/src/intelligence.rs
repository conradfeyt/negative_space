// intelligence.rs — Apple Intelligence integration via Swift bridge.
//
// This module provides the Rust-side FFI bindings to the Swift
// NegativeSpaceIntelligence library. All data crosses the boundary as JSON strings.
//
// When the Swift bridge is not available (CI, non-macOS, Intel Macs),
// all functions return empty/default results via the `has_swift_bridge` cfg.

use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};

// ---------------------------------------------------------------------------
// FFI declarations (Swift @_cdecl exports)
// ---------------------------------------------------------------------------

#[cfg(has_swift_bridge)]
extern "C" {
    fn msw_check_intelligence_available() -> i32;
    fn msw_classify_files(json_input: *const c_char) -> *mut c_char;
    fn msw_generate_scan_summary(json_input: *const c_char) -> *mut c_char;
    fn msw_free_string(ptr: *mut c_char);
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileClassification {
    pub path: String,
    pub safety: String,
    pub explanation: String,
    pub confidence: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileClassificationInput {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub file_type: String,
    pub modified: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScanSummaryInput {
    pub domains: Vec<DomainResultInput>,
    pub total_reclaimable: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DomainResultInput {
    pub domain: String,
    pub item_count: u64,
    pub total_size: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScanSummaryOutput {
    pub summary: String,
    pub ai_generated: bool,
}

// ---------------------------------------------------------------------------
// Availability
// ---------------------------------------------------------------------------

static BRIDGE_AVAILABLE: AtomicBool = AtomicBool::new(false);
static CHECKED: AtomicBool = AtomicBool::new(false);

/// Check if the Swift intelligence bridge is available and loaded.
pub fn is_available() -> bool {
    if !CHECKED.load(Ordering::Relaxed) {
        #[cfg(has_swift_bridge)]
        {
            let available = unsafe { msw_check_intelligence_available() } == 1;
            BRIDGE_AVAILABLE.store(available, Ordering::Relaxed);
        }
        CHECKED.store(true, Ordering::Relaxed);
    }
    // The bridge itself (rule-based) is always available when compiled in
    #[cfg(has_swift_bridge)]
    return true;
    #[cfg(not(has_swift_bridge))]
    return false;
}

/// Check if the on-device LLM (Foundation Models) is available.
pub fn is_ai_available() -> bool {
    if !CHECKED.load(Ordering::Relaxed) {
        is_available(); // Force check
    }
    BRIDGE_AVAILABLE.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// FFI helpers
// ---------------------------------------------------------------------------

#[cfg(has_swift_bridge)]
fn call_swift(func: unsafe extern "C" fn(*const c_char) -> *mut c_char, input: &str) -> String {
    let c_input = match CString::new(input) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let result_ptr = unsafe { func(c_input.as_ptr()) };
    if result_ptr.is_null() {
        return String::new();
    }

    let result = unsafe { CStr::from_ptr(result_ptr) }
        .to_string_lossy()
        .to_string();

    unsafe { msw_free_string(result_ptr) };
    result
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Classify files by safety for deletion.
pub fn classify_files(files: &[FileClassificationInput]) -> Vec<FileClassification> {
    #[cfg(has_swift_bridge)]
    {
        let json = match serde_json::to_string(files) {
            Ok(j) => j,
            Err(_) => return vec![],
        };

        let result_json = call_swift(msw_classify_files, &json);
        serde_json::from_str(&result_json).unwrap_or_default()
    }

    #[cfg(not(has_swift_bridge))]
    {
        let _ = files;
        vec![]
    }
}

/// Generate a natural language scan summary.
pub fn generate_scan_summary(input: &ScanSummaryInput) -> ScanSummaryOutput {
    #[cfg(has_swift_bridge)]
    {
        let json = match serde_json::to_string(input) {
            Ok(j) => j,
            Err(_) => return ScanSummaryOutput { summary: String::new(), ai_generated: false },
        };

        let result_json = call_swift(msw_generate_scan_summary, &json);
        serde_json::from_str(&result_json).unwrap_or(ScanSummaryOutput {
            summary: String::new(),
            ai_generated: false,
        })
    }

    #[cfg(not(has_swift_bridge))]
    {
        let _ = input;
        ScanSummaryOutput { summary: String::new(), ai_generated: false }
    }
}
