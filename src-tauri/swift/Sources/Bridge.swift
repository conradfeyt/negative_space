// Bridge.swift — C-exported entry points for Rust FFI.
//
// All functions use @_cdecl to export with C calling convention.
// Data crosses the boundary as JSON strings (simple, no complex marshaling).
// Rust calls these via `extern "C"` declarations.

import Foundation
import AppKit

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

// MARK: - SF Symbols

/// Render a system icon to a base64 PNG string.
/// Input: JSON { "name": "symbol.or.path", "size": 24, "mode": "sf"|"app"|"system" }
/// Output: JSON { "base64": "data:image/png;base64,..." } or { "base64": "" } on failure
@_cdecl("msw_render_sf_symbol")
public func renderSFSymbol(_ jsonInput: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar> {
    let input = String(cString: jsonInput)
    guard let data = input.data(using: .utf8),
          let params = try? JSONDecoder().decode(SFSymbolParams.self, from: data) else {
        return strdup("{\"base64\":\"\"}")!
    }

    let size = CGFloat(params.size ?? 32)
    let mode = params.mode ?? "sf"
    var image: NSImage?

    switch mode {
    case "app":
        // Get app icon by path (e.g. "/System/Applications/Books.app")
        image = NSWorkspace.shared.icon(forFile: params.name)
    case "system":
        // Get system icon by NSImage name
        image = NSImage(named: NSImage.Name(params.name))
    default:
        // SF Symbol
        if let sfImage = NSImage(systemSymbolName: params.name, accessibilityDescription: nil) {
            let config = NSImage.SymbolConfiguration(pointSize: size, weight: .medium)
                .applying(.init(paletteColors: [.labelColor]))
            image = sfImage.withSymbolConfiguration(config) ?? sfImage
        }
    }

    guard let finalImage = image else {
        return strdup("{\"base64\":\"\"}")!
    }

    // Render to bitmap at 2x for retina
    let pixelSize = NSSize(width: size * 2, height: size * 2)
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: Int(pixelSize.width),
        pixelsHigh: Int(pixelSize.height),
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .deviceRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    )!

    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)

    let drawRect = NSRect(origin: .zero, size: pixelSize)
    finalImage.draw(in: drawRect, from: .zero, operation: .sourceOver, fraction: 1.0)

    NSGraphicsContext.restoreGraphicsState()

    guard let pngData = rep.representation(using: .png, properties: [:]) else {
        return strdup("{\"base64\":\"\"}")!
    }

    let base64 = pngData.base64EncodedString()
    let result = "{\"base64\":\"data:image/png;base64,\(base64)\"}"
    return strdup(result)!
}

private struct SFSymbolParams: Codable {
    let name: String
    let size: Int?
    let mode: String?
}

// MARK: - Memory Management

/// Free a string allocated by this library. Must be called from Rust for every
/// string returned by msw_* functions to avoid memory leaks.
@_cdecl("msw_free_string")
public func freeString(_ ptr: UnsafeMutablePointer<CChar>) {
    free(ptr)
}
