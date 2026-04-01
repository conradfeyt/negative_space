// Bridge.swift — C-exported entry points for Rust FFI.
//
// All functions use @_cdecl to export with C calling convention.
// Data crosses the boundary as JSON strings (simple, no complex marshaling).
// Rust calls these via `extern "C"` declarations.

import Foundation

// MARK: - Availability Check

/// Returns 1 if Apple Intelligence features are available, 0 otherwise.
/// Checks: macOS 26+, Apple Silicon, Foundation Models loadable.
@_cdecl("msw_check_intelligence_available")
public func checkIntelligenceAvailable() -> Int32 {
    // Foundation Models requires macOS 26+ and Apple Silicon.
    // Until Xcode 26 SDK is available, this returns 0 (graceful fallback).
    #if canImport(FoundationModels)
    if #available(macOS 26, *) {
        return 1
    }
    #endif
    return 0
}

// MARK: - File Classification

/// Classify files by safety for deletion.
/// Input: JSON array of { "path": string, "name": string, "size": number, "file_type": string }
/// Output: JSON array of { "path": string, "safety": string, "explanation": string, "confidence": number }
@_cdecl("msw_classify_files")
public func classifyFiles(_ jsonInput: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar> {
    let input = String(cString: jsonInput)

    // When Foundation Models is available, this will use the on-device LLM.
    // For now, use rule-based classification as a useful fallback.
    let result = RuleBasedClassifier.classify(json: input)
    return strdup(result)!
}

// MARK: - Scan Summary Generation

/// Generate a natural language summary of scan results.
/// Input: JSON object with domain results (sizes, counts, etc.)
/// Output: JSON { "summary": string }
@_cdecl("msw_generate_scan_summary")
public func generateScanSummary(_ jsonInput: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar> {
    let input = String(cString: jsonInput)

    // When Foundation Models is available, this will generate rich summaries.
    // For now, use template-based generation.
    let result = TemplateSummaryGenerator.generate(json: input)
    return strdup(result)!
}

// MARK: - Memory Management

/// Free a string allocated by this library. Must be called from Rust for every
/// string returned by msw_* functions to avoid memory leaks.
@_cdecl("msw_free_string")
public func freeString(_ ptr: UnsafeMutablePointer<CChar>) {
    free(ptr)
}
