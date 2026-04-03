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
///
/// Input JSON:
/// - `name`: SF Symbol name, app path, or NSImage name
/// - `size`: pixel size (default 32)
/// - `mode`: "sf" | "app" | "system"
/// - `style`: "plain" (default) | "grayBadge" (white glyph on gray rounded rect) | "grayscaleApp" (desaturated app icon)
///
/// Output: JSON { "base64": "data:image/png;base64,..." }
@_cdecl("msw_render_sf_symbol")
public func renderSFSymbol(_ jsonInput: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar> {
    let input = String(cString: jsonInput)
    guard let data = input.data(using: .utf8),
          let params = try? JSONDecoder().decode(SFSymbolParams.self, from: data) else {
        return strdup("{\"base64\":\"\"}")!
    }

    let size = CGFloat(params.size ?? 32)
    let mode = params.mode ?? "sf"
    let style = params.style ?? "plain"

    // Step 1: Get the source image
    var image: NSImage?

    switch mode {
    case "app":
        let resolvedPath = params.name.replacingOccurrences(of: "HOMEDIR", with: NSHomeDirectory())
        image = NSWorkspace.shared.icon(forFile: resolvedPath)
    case "file":
        image = NSImage(contentsOfFile: params.name)
    case "system":
        image = NSImage(named: NSImage.Name(params.name))
    default: // "sf"
        if let sfImage = NSImage(systemSymbolName: params.name, accessibilityDescription: nil) {
            // For grayBadge style, render in white. Otherwise use label color.
            let color: NSColor = (style == "grayBadge") ? .white : .labelColor
            let config = NSImage.SymbolConfiguration(pointSize: size * 0.65, weight: .medium)
                .applying(.init(paletteColors: [color]))
            image = sfImage.withSymbolConfiguration(config) ?? sfImage
        }
    }

    guard let sourceImage = image else {
        return strdup("{\"base64\":\"\"}")!
    }

    // Step 2: Render to bitmap based on style
    let scale: CGFloat = 2.0 // retina
    let pixelSize = NSSize(width: size * scale, height: size * scale)

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

    let fullRect = NSRect(origin: .zero, size: pixelSize)

    switch style {
    case "grayBadge":
        // Match macOS Storage pane: 64px total with 7px padding → 48px icon area
        // Gray rounded rect at 48/64 = 75% of total size, centered
        // Border radius 25% of the rect size
        // Glyph ~35/48 ≈ 73% of rect size

        let padding = pixelSize.width * (7.0 / 64.0) // Match app icon visual size (25 CSS px badge)
        let rectSize = pixelSize.width - (padding * 2)
        let rectOrigin = NSPoint(x: padding, y: padding)
        let badgeRect = NSRect(origin: rectOrigin, size: NSSize(width: rectSize, height: rectSize))
        let cornerRadius = rectSize * 0.25

        // Soft drop shadow
        let shadow = NSShadow()
        shadow.shadowColor = NSColor(calibratedWhite: 0.0, alpha: 0.2)
        shadow.shadowOffset = NSSize(width: 0, height: -1 * scale)
        shadow.shadowBlurRadius = 2 * scale
        shadow.set()

        // Gray gradient background: #A9A9A9 top → #898989 bottom
        let bgPath = NSBezierPath(roundedRect: badgeRect, xRadius: cornerRadius, yRadius: cornerRadius)
        let topColor = NSColor(calibratedRed: 0.663, green: 0.663, blue: 0.663, alpha: 1.0) // #A9A9A9
        let bottomColor = NSColor(calibratedRed: 0.455, green: 0.455, blue: 0.455, alpha: 1.0) // #747474
        let gradient = NSGradient(starting: topColor, ending: bottomColor)!
        gradient.draw(in: bgPath, angle: 270)

        // Reset shadow for glyph drawing
        NSShadow().set()

        // Draw glyph centered in the badge rect, preserving aspect ratio
        let glyphScaleFactor = CGFloat(params.glyphScale ?? 1.0)
        let maxGlyphDim = rectSize * 0.76 * glyphScaleFactor
        let naturalSize = sourceImage.size
        let aspect = naturalSize.width / max(naturalSize.height, 1)
        let glyphW: CGFloat
        let glyphH: CGFloat
        if aspect >= 1.0 {
            glyphW = maxGlyphDim
            glyphH = maxGlyphDim / aspect
        } else {
            glyphH = maxGlyphDim
            glyphW = maxGlyphDim * aspect
        }
        let glyphRect = NSRect(
            x: badgeRect.midX - glyphW / 2,
            y: badgeRect.midY - glyphH / 2,
            width: glyphW,
            height: glyphH
        )

        if mode == "sf" {
            sourceImage.draw(in: glyphRect, from: .zero, operation: .sourceOver, fraction: 1.0)
        } else {
            let tinted = tintImage(sourceImage, color: .white)
            tinted.draw(in: glyphRect, from: .zero, operation: .sourceOver, fraction: 1.0)
        }

    case "grayBadgeHier":
        // Same gray badge but with hierarchical symbol rendering (preserves depth layers)
        let hierPadding = pixelSize.width * (7.0 / 64.0)
        let hierRectSize = pixelSize.width - (hierPadding * 2)
        let hierOrigin = NSPoint(x: hierPadding, y: hierPadding)
        let hierBadgeRect = NSRect(origin: hierOrigin, size: NSSize(width: hierRectSize, height: hierRectSize))
        let hierCorner = hierRectSize * 0.25

        let hierShadow = NSShadow()
        hierShadow.shadowColor = NSColor(calibratedWhite: 0.0, alpha: 0.2)
        hierShadow.shadowOffset = NSSize(width: 0, height: -1 * scale)
        hierShadow.shadowBlurRadius = 2 * scale
        hierShadow.set()

        let hierBgPath = NSBezierPath(roundedRect: hierBadgeRect, xRadius: hierCorner, yRadius: hierCorner)
        let hierTopColor = NSColor(calibratedRed: 0.663, green: 0.663, blue: 0.663, alpha: 1.0)
        let hierBottomColor = NSColor(calibratedRed: 0.455, green: 0.455, blue: 0.455, alpha: 1.0)
        let hierGradient = NSGradient(starting: hierTopColor, ending: hierBottomColor)!
        hierGradient.draw(in: hierBgPath, angle: 270)

        NSShadow().set()

        // Draw glyph with hierarchical rendering for SF, white tint for system images
        let hierGlyphScale = CGFloat(params.glyphScale ?? 1.0)
        let hierMaxDim = hierRectSize * 0.76 * hierGlyphScale
        if mode == "sf", let sfImg = NSImage(systemSymbolName: params.name, accessibilityDescription: nil) {
            let hierConfig = NSImage.SymbolConfiguration(pointSize: size * 0.65, weight: .medium)
                .applying(.init(hierarchicalColor: .white))
            let hierSymbol = sfImg.withSymbolConfiguration(hierConfig) ?? sfImg
            let hierNat = hierSymbol.size
            let hierAsp = hierNat.width / max(hierNat.height, 1)
            let hierGW = hierAsp >= 1 ? hierMaxDim : hierMaxDim * hierAsp
            let hierGH = hierAsp >= 1 ? hierMaxDim / hierAsp : hierMaxDim
            let hierGRect = NSRect(x: hierBadgeRect.midX - hierGW/2, y: hierBadgeRect.midY - hierGH/2, width: hierGW, height: hierGH)
            hierSymbol.draw(in: hierGRect, from: .zero, operation: .sourceOver, fraction: 1.0)
        } else {
            // System image: tint white
            let tinted = tintImage(sourceImage, color: .white)
            let hierNat = tinted.size
            let hierAsp = hierNat.width / max(hierNat.height, 1)
            let hierGW = hierAsp >= 1 ? hierMaxDim : hierMaxDim * hierAsp
            let hierGH = hierAsp >= 1 ? hierMaxDim / hierAsp : hierMaxDim
            let hierGRect = NSRect(x: hierBadgeRect.midX - hierGW/2, y: hierBadgeRect.midY - hierGH/2, width: hierGW, height: hierGH)
            tinted.draw(in: hierGRect, from: .zero, operation: .sourceOver, fraction: 1.0)
        }

    case "blueBadge":
        // White rounded rect with blue SF Symbol (for iCloud)
        let padding = pixelSize.width * (7.0 / 64.0)
        let rectSize = pixelSize.width - (padding * 2)
        let rectOrigin = NSPoint(x: padding, y: padding)
        let badgeRect = NSRect(origin: rectOrigin, size: NSSize(width: rectSize, height: rectSize))
        let cornerRadius = rectSize * 0.25

        let shadow = NSShadow()
        shadow.shadowColor = NSColor(calibratedWhite: 0.0, alpha: 0.12)
        shadow.shadowOffset = NSSize(width: 0, height: -1 * scale)
        shadow.shadowBlurRadius = 2 * scale
        shadow.set()

        let bgPath = NSBezierPath(roundedRect: badgeRect, xRadius: cornerRadius, yRadius: cornerRadius)
        NSColor(calibratedWhite: 0.95, alpha: 1.0).setFill()
        bgPath.fill()

        // Light border
        NSColor(calibratedWhite: 0.85, alpha: 1.0).setStroke()
        bgPath.lineWidth = 0.5 * scale
        bgPath.stroke()

        NSShadow().set()

        // Render SF Symbol in blue
        if let sfImage = NSImage(systemSymbolName: params.name, accessibilityDescription: nil) {
            let blueConfig = NSImage.SymbolConfiguration(pointSize: size * 0.65, weight: .medium)
                .applying(.init(paletteColors: [NSColor.systemBlue]))
            let blueSymbol = sfImage.withSymbolConfiguration(blueConfig) ?? sfImage
            let maxDim = rectSize * 0.76
            let nat = blueSymbol.size
            let asp = nat.width / max(nat.height, 1)
            let gW = asp >= 1 ? maxDim : maxDim * asp
            let gH = asp >= 1 ? maxDim / asp : maxDim
            let gRect = NSRect(x: badgeRect.midX - gW/2, y: badgeRect.midY - gH/2, width: gW, height: gH)
            blueSymbol.draw(in: gRect, from: .zero, operation: .sourceOver, fraction: 1.0)
        }

    case "grayscaleApp":
        // Desaturate app icon, add soft drop shadow, draw full-size
        let shadow = NSShadow()
        shadow.shadowColor = NSColor(calibratedWhite: 0.0, alpha: 0.2)
        shadow.shadowOffset = NSSize(width: 0, height: -1 * scale)
        shadow.shadowBlurRadius = 2 * scale
        shadow.set()

        let gray = desaturateImage(sourceImage) ?? sourceImage
        gray.draw(in: fullRect, from: .zero, operation: .sourceOver, fraction: 1.0)

        NSShadow().set()

    default: // "plain"
        sourceImage.draw(in: fullRect, from: .zero, operation: .sourceOver, fraction: 1.0)
    }

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
    let style: String?
    let glyphScale: Double?
}

/// Desaturate an NSImage to grayscale using CIFilter.
private func desaturateImage(_ image: NSImage) -> NSImage? {
    guard let tiffData = image.tiffRepresentation,
          let ciImage = CIImage(data: tiffData) else { return nil }

    let filter = CIFilter(name: "CIColorControls")!
    filter.setValue(ciImage, forKey: kCIInputImageKey)
    filter.setValue(0.0, forKey: kCIInputSaturationKey)

    guard let output = filter.outputImage else { return nil }

    let rep = NSCIImageRep(ciImage: output)
    let nsImage = NSImage(size: rep.size)
    nsImage.addRepresentation(rep)
    return nsImage
}

/// Tint a template/system image to a specific color.
private func tintImage(_ image: NSImage, color: NSColor) -> NSImage {
    let tinted = NSImage(size: image.size)
    tinted.lockFocus()
    color.set()
    let rect = NSRect(origin: .zero, size: image.size)
    image.draw(in: rect, from: .zero, operation: .sourceOver, fraction: 1.0)
    rect.fill(using: .sourceAtop)
    tinted.unlockFocus()
    return tinted
}

// MARK: - List Available System Images

/// Returns a JSON array of all known NSImage system image names.
@_cdecl("msw_list_system_images")
public func listSystemImages() -> UnsafeMutablePointer<CChar> {
    // All documented NSImage.Name constants
    let names: [String] = [
        "NSAddTemplate", "NSAdvanced", "NSApplicationIcon", "NSBluetoothTemplate",
        "NSBonjour", "NSBookmarksTemplate", "NSCaution", "NSColorPanel",
        "NSColumnViewTemplate", "NSComputer", "NSEnterFullScreenTemplate",
        "NSEveryone", "NSExitFullScreenTemplate", "NSFlowViewTemplate",
        "NSFolder", "NSFolderBurnable", "NSFolderSmart",
        "NSFollowLinkFreestandingTemplate", "NSFontPanel", "NSGoBackTemplate",
        "NSGoForwardTemplate", "NSGoLeftTemplate", "NSGoRightTemplate",
        "NSHomeTemplate", "NSIChatTheaterTemplate", "NSIconViewTemplate",
        "NSInfo", "NSInvalidDataFreestandingTemplate",
        "NSLeftFacingTriangleTemplate", "NSListViewTemplate",
        "NSLockLockedTemplate", "NSLockUnlockedTemplate", "NSMenuMixedStateTemplate",
        "NSMenuOnStateTemplate", "NSMobileMe", "NSMultipleDocuments",
        "NSNetwork", "NSPathTemplate", "NSPreferencesGeneral",
        "NSQuickLookTemplate", "NSRefreshFreestandingTemplate",
        "NSRefreshTemplate", "NSRemoveTemplate", "NSRevealFreestandingTemplate",
        "NSRightFacingTriangleTemplate", "NSShareTemplate",
        "NSSlideshowTemplate", "NSSmartBadgeTemplate",
        "NSStatusAvailable", "NSStatusNone", "NSStatusPartiallyAvailable",
        "NSStatusUnavailable", "NSStopProgressFreestandingTemplate",
        "NSStopProgressTemplate", "NSTrashEmpty", "NSTrashFull",
        "NSUser", "NSUserAccounts", "NSUserGroup", "NSUserGuest",
        "NSActionTemplate", "NSMenuMixedStateTemplate",
        "NSMenuOnStateTemplate", "NSTouchBarAddDetailTemplate",
        "NSTouchBarAddTemplate", "NSTouchBarColorPickerFill",
        "NSTouchBarColorPickerFont", "NSTouchBarColorPickerStroke",
        "NSTouchBarCommunicationAudioTemplate",
        "NSTouchBarCommunicationVideoTemplate",
        "NSTouchBarComposeTemplate", "NSTouchBarDeleteTemplate",
        "NSTouchBarDownloadTemplate",
    ]

    guard let data = try? JSONEncoder().encode(names),
          let str = String(data: data, encoding: .utf8) else {
        return strdup("[]")!
    }
    return strdup(str)!
}

// MARK: - Memory Management

/// Free a string allocated by this library. Must be called from Rust for every
/// string returned by msw_* functions to avoid memory leaks.
@_cdecl("msw_free_string")
public func freeString(_ ptr: UnsafeMutablePointer<CChar>) {
    free(ptr)
}
