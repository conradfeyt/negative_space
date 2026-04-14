// Bridge.swift — C-exported entry points for Rust FFI.
//
// All functions use @_cdecl to export with C calling convention.
// Data crosses the boundary as JSON strings (simple, no complex marshaling).
// Rust calls these via `extern "C"` declarations.

import Foundation
import AppKit
import UniformTypeIdentifiers
import Vision
import CoreML
import Photos


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
    case "uttype":
        // Get icon for a file extension or UTType identifier via UTType
        if let ut = UTType(filenameExtension: params.name) {
            image = NSWorkspace.shared.icon(for: ut)
        } else if let ut = UTType(params.name) {
            image = NSWorkspace.shared.icon(for: ut)
        }
    case "system":
        image = NSImage(named: NSImage.Name(params.name))
    default: // "sf"
        if let sfImage = NSImage(systemSymbolName: params.name, accessibilityDescription: nil) {
            if style == "multicolor" {
                // Render in native multicolor (system-defined colors for each layer)
                var config = NSImage.SymbolConfiguration(pointSize: size * 0.65, weight: .medium)
                if #available(macOS 12.0, *) {
                    config = config.applying(.preferringMulticolor())
                }
                image = sfImage.withSymbolConfiguration(config) ?? sfImage
            } else {
                // For grayBadge style, render in white. Otherwise use label color.
                let color: NSColor = (style == "grayBadge") ? .white : .labelColor
                let config = NSImage.SymbolConfiguration(pointSize: size * 0.65, weight: .medium)
                    .applying(.init(paletteColors: [color]))
                image = sfImage.withSymbolConfiguration(config) ?? sfImage
            }
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

    case "blueGradientBadge":
        // Blue gradient rounded rect with white glyph (Privacy & Security style)
        let bluePadding = pixelSize.width * (7.0 / 64.0)
        let blueRectSize = pixelSize.width - (bluePadding * 2)
        let blueRectOrigin = NSPoint(x: bluePadding, y: bluePadding)
        let blueBadgeRect = NSRect(origin: blueRectOrigin, size: NSSize(width: blueRectSize, height: blueRectSize))
        let blueCornerRadius = blueRectSize * 0.25

        let blueShadow = NSShadow()
        blueShadow.shadowColor = NSColor(calibratedWhite: 0.0, alpha: 0.2)
        blueShadow.shadowOffset = NSSize(width: 0, height: -1 * scale)
        blueShadow.shadowBlurRadius = 2 * scale
        blueShadow.set()

        // Blue gradient: #47A8FF top → #0690FF bottom
        let blueBgPath = NSBezierPath(roundedRect: blueBadgeRect, xRadius: blueCornerRadius, yRadius: blueCornerRadius)
        let blueTopColor = NSColor(srgbRed: 0.278, green: 0.659, blue: 1.0, alpha: 1.0)   // #47A8FF
        let blueBottomColor = NSColor(srgbRed: 0.024, green: 0.565, blue: 1.0, alpha: 1.0) // #0690FF
        let blueGradient = NSGradient(starting: blueTopColor, ending: blueBottomColor)!
        blueGradient.draw(in: blueBgPath, angle: 270)

        NSShadow().set()

        // Draw white glyph
        let blueGlyphScale = CGFloat(params.glyphScale ?? 1.0)
        let blueMaxDim = blueRectSize * 0.76 * blueGlyphScale
        if mode == "sf", let sfImg = NSImage(systemSymbolName: params.name, accessibilityDescription: nil) {
            let whiteConfig = NSImage.SymbolConfiguration(pointSize: size * 0.65, weight: .medium)
                .applying(.init(paletteColors: [.white]))
            let whiteSymbol = sfImg.withSymbolConfiguration(whiteConfig) ?? sfImg
            let nat = whiteSymbol.size
            let asp = nat.width / max(nat.height, 1)
            let gW = asp >= 1 ? blueMaxDim : blueMaxDim * asp
            let gH = asp >= 1 ? blueMaxDim / asp : blueMaxDim
            let gRect = NSRect(x: blueBadgeRect.midX - gW/2, y: blueBadgeRect.midY - gH/2, width: gW, height: gH)
            whiteSymbol.draw(in: gRect, from: .zero, operation: .sourceOver, fraction: 1.0)
        } else {
            let tinted = tintImage(sourceImage, color: .white)
            let nat = tinted.size
            let asp = nat.width / max(nat.height, 1)
            let gW = asp >= 1 ? blueMaxDim : blueMaxDim * asp
            let gH = asp >= 1 ? blueMaxDim / asp : blueMaxDim
            let gRect = NSRect(x: blueBadgeRect.midX - gW/2, y: blueBadgeRect.midY - gH/2, width: gW, height: gH)
            tinted.draw(in: gRect, from: .zero, operation: .sourceOver, fraction: 1.0)
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
        // Preserve aspect ratio — center the image in the square canvas
        let nat = sourceImage.size
        let aspect = nat.width / max(nat.height, 1)
        let drawW: CGFloat
        let drawH: CGFloat
        if aspect >= 1.0 {
            drawW = pixelSize.width
            drawH = pixelSize.width / aspect
        } else {
            drawH = pixelSize.height
            drawW = pixelSize.height * aspect
        }
        let drawRect = NSRect(
            x: (pixelSize.width - drawW) / 2,
            y: (pixelSize.height - drawH) / 2,
            width: drawW,
            height: drawH
        )
        sourceImage.draw(in: drawRect, from: .zero, operation: .sourceOver, fraction: 1.0)
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

// MARK: - Photos Library Access

/// Check current Photos authorization status.
/// Output JSON: { "status": "authorized"|"denied"|"restricted"|"notDetermined"|"limited", "canPrompt": bool }
@_cdecl("msw_photos_auth_status")
public func photosAuthStatus() -> UnsafeMutablePointer<CChar> {
    let status = PHPhotoLibrary.authorizationStatus(for: .readWrite)
    let statusStr: String
    switch status {
    case .authorized: statusStr = "authorized"
    case .denied: statusStr = "denied"
    case .restricted: statusStr = "restricted"
    case .notDetermined: statusStr = "notDetermined"
    case .limited: statusStr = "limited"
    @unknown default: statusStr = "unknown"
    }
    let result = "{\"status\":\"\(statusStr)\",\"canPrompt\":\(status == .notDetermined)}"
    return strdup(result)!
}

/// Request Photos library read-write access (shows system prompt if not yet determined).
/// Blocks until the user responds. Returns "authorized", "denied", etc.
@_cdecl("msw_request_photos_access")
public func requestPhotosAccess() -> UnsafeMutablePointer<CChar> {
    let sem = DispatchSemaphore(value: 0)
    var resultStatus = ""
    PHPhotoLibrary.requestAuthorization(for: .readWrite) { status in
        switch status {
        case .authorized: resultStatus = "authorized"
        case .denied: resultStatus = "denied"
        case .restricted: resultStatus = "restricted"
        case .limited: resultStatus = "limited"
        case .notDetermined: resultStatus = "notDetermined"
        @unknown default: resultStatus = "unknown"
        }
        sem.signal()
    }
    sem.wait()
    return strdup(resultStatus)!
}

/// Enumerate Photos library image assets and return their file URLs.
/// Input JSON: { "min_size": u64 (optional, bytes) }
/// Output JSON: { "paths": [String], "count": Int, "error": String? }
///
/// Uses PHContentEditingInput to obtain file URLs for each image asset.
/// Processes in batches to avoid excessive memory use.
@_cdecl("msw_enumerate_photo_paths")
public func enumeratePhotoPaths(_ jsonInput: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar> {
    let input = String(cString: jsonInput)
    var minSize: UInt64 = 0
    if let data = input.data(using: .utf8),
       let params = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
        if let ms = params["min_size"] as? UInt64 { minSize = ms }
    }

    let status = PHPhotoLibrary.authorizationStatus(for: .readWrite)
    guard status == .authorized || status == .limited else {
        let err = "{\"paths\":[],\"count\":0,\"error\":\"Photos access not authorized (status: \(status.rawValue))\"}"
        return strdup(err)!
    }

    let fetchOptions = PHFetchOptions()
    fetchOptions.sortDescriptors = [NSSortDescriptor(key: "creationDate", ascending: false)]
    fetchOptions.includeHiddenAssets = false
    let assets = PHAsset.fetchAssets(with: .image, options: fetchOptions)

    var entries: [[String: String]] = []
    var skippedCloud: Int = 0

    let inputOptions = PHContentEditingInputRequestOptions()
    inputOptions.isNetworkAccessAllowed = false

    let workQueue = DispatchQueue(label: "com.conradfe.negativespace.photoenum", qos: .userInitiated)
    let outerSem = DispatchSemaphore(value: 0)

    workQueue.async {
        for idx in 0..<assets.count {
            autoreleasepool {
                let asset = assets.object(at: idx)

                if minSize > 0 {
                    let resources = PHAssetResource.assetResources(for: asset)
                    if let primary = resources.first {
                        let fileSize = (primary.value(forKey: "fileSize") as? NSNumber)?.uint64Value ?? 0
                        if fileSize < minSize { return }
                    }
                }

                let sem = DispatchSemaphore(value: 0)
                asset.requestContentEditingInput(with: inputOptions) { input, _ in
                    if let url = input?.fullSizeImageURL,
                       FileManager.default.fileExists(atPath: url.path) {
                        entries.append(["path": url.path, "id": asset.localIdentifier])
                    } else {
                        skippedCloud += 1
                    }
                    sem.signal()
                }
                sem.wait()
            }
        }
        outerSem.signal()
    }
    outerSem.wait()

    let paths = entries.map { $0["path"]! }
    var result: [String: Any] = [
        "paths": paths,
        "entries": entries,
        "count": entries.count,
        "total_assets": assets.count,
        "skipped_cloud": skippedCloud,
    ]
    if skippedCloud > 0 {
        result["error"] = "\(skippedCloud) iCloud-only photos skipped (not downloaded locally)."
    }
    guard let jsonData = try? JSONSerialization.data(withJSONObject: result),
          let jsonStr = String(data: jsonData, encoding: .utf8) else {
        return strdup("{\"paths\":[],\"count\":0,\"error\":\"JSON encoding failed\"}")!
    }
    return strdup(jsonStr)!
}

/// Delete photos from the Photos Library via PhotoKit.
/// Input JSON: array of localIdentifier strings, e.g. ["ABC-123/L0/001", ...]
/// Triggers the system confirmation dialog. Returns JSON with deleted count.
@_cdecl("msw_delete_photo_assets")
public func deletePhotoAssets(_ jsonInput: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar> {
    let input = String(cString: jsonInput)
    guard let data = input.data(using: .utf8),
          let identifiers = try? JSONSerialization.jsonObject(with: data) as? [String] else {
        return strdup("{\"deleted\":0,\"error\":\"Invalid input\"}")!
    }

    let fetchResult = PHAsset.fetchAssets(withLocalIdentifiers: identifiers, options: nil)
    guard fetchResult.count > 0 else {
        return strdup("{\"deleted\":0,\"error\":\"No matching assets found\"}")!
    }

    let sem = DispatchSemaphore(value: 0)
    var deletedCount = 0
    var deleteError: String? = nil

    PHPhotoLibrary.shared().performChanges({
        PHAssetChangeRequest.deleteAssets(fetchResult)
    }) { success, error in
        if success {
            deletedCount = fetchResult.count
        } else {
            deleteError = error?.localizedDescription ?? "Deletion denied"
        }
        sem.signal()
    }
    sem.wait()

    var result: [String: Any] = ["deleted": deletedCount]
    if let err = deleteError { result["error"] = err }
    guard let jsonData = try? JSONSerialization.data(withJSONObject: result),
          let jsonStr = String(data: jsonData, encoding: .utf8) else {
        return strdup("{\"deleted\":0,\"error\":\"JSON error\"}")!
    }
    return strdup(jsonStr)!
}

/// Generate a thumbnail for a Photos library asset by localIdentifier.
/// Input JSON: { "identifier": String, "size": Int }
/// Output: base64 JPEG string (or empty string on failure)
@_cdecl("msw_photos_thumbnail")
public func photosThumbnail(_ jsonInput: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar> {
    let input = String(cString: jsonInput)
    guard let data = input.data(using: .utf8),
          let params = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
          let identifier = params["identifier"] as? String else {
        return strdup("")!
    }
    let size = (params["size"] as? Int) ?? 200

    let fetchResult = PHAsset.fetchAssets(withLocalIdentifiers: [identifier], options: nil)
    guard let asset = fetchResult.firstObject else { return strdup("")! }

    let sem = DispatchSemaphore(value: 0)
    var resultB64 = ""

    let options = PHImageRequestOptions()
    options.isNetworkAccessAllowed = false
    options.isSynchronous = false
    options.deliveryMode = .highQualityFormat

    let targetSize = CGSize(width: size, height: size)
    PHImageManager.default().requestImage(for: asset, targetSize: targetSize, contentMode: .aspectFill, options: options) { image, _ in
        defer { sem.signal() }
        guard let image = image else { return }
        let bitmapRep = NSBitmapImageRep(data: image.tiffRepresentation!)!
        if let jpegData = bitmapRep.representation(using: .jpeg, properties: [.compressionFactor: 0.7]) {
            resultB64 = "data:image/jpeg;base64," + jpegData.base64EncodedString()
        }
    }
    sem.wait()

    return strdup(resultB64)!
}

// MARK: - NSFW Classification (file-system based)

/// Classify images for NSFW content using CoreML + Vision.
///
/// Input JSON: { "paths": [String], "model_path": String }
/// Output JSON: [{ "path": String, "score": Double }]
///
/// Uses VNCoreMLRequest to run the bundled OpenNSFW2 model on each image.
/// Returns the NSFW probability score (0.0 = safe, 1.0 = explicit) for each
/// image that could be successfully classified. Failed images are silently skipped.
@_cdecl("msw_classify_nsfw")
public func classifyNsfw(_ jsonInput: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar> {
    let input = String(cString: jsonInput)
    guard let data = input.data(using: .utf8),
          let params = try? JSONDecoder().decode(NsfwParams.self, from: data) else {
        return strdup("[]")!
    }

    let modelURL = URL(fileURLWithPath: params.model_path)
    guard FileManager.default.fileExists(atPath: params.model_path) else {
        return strdup("[]")!
    }

    guard let compiledModel = try? MLModel(contentsOf: modelURL),
          let vnModel = try? VNCoreMLModel(for: compiledModel) else {
        return strdup("[]")!
    }

    var results: [[String: Any]] = []

    for imagePath in params.paths {
        autoreleasepool {
            let imageURL = URL(fileURLWithPath: imagePath)
            guard let imageSource = CGImageSourceCreateWithURL(imageURL as CFURL, nil),
                  let cgImage = CGImageSourceCreateImageAtIndex(imageSource, 0, nil) else {
                return
            }

            let request = VNCoreMLRequest(model: vnModel)
            request.imageCropAndScaleOption = .scaleFill

            let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])
            do {
                try handler.perform([request])
            } catch {
                return
            }

            guard let observations = request.results as? [VNClassificationObservation] else { return }

            let nsfwScore = observations.first(where: { $0.identifier == "nsfw" })?.confidence ?? 0.0
            results.append([
                "path": imagePath,
                "score": Double(nsfwScore)
            ])
        }
    }

    guard let jsonData = try? JSONSerialization.data(withJSONObject: results),
          let jsonStr = String(data: jsonData, encoding: .utf8) else {
        return strdup("[]")!
    }
    return strdup(jsonStr)!
}

private struct NsfwParams: Codable {
    let paths: [String]
    let model_path: String
}

// MARK: - NudeNet Detection (YOLO-based body part detection)

/// NudeNet class labels (YOLOv8-nano 320n, 18 classes)
private let nudeNetLabels: [String] = [
    "FEMALE_GENITALIA_COVERED", "FACE_FEMALE", "BUTTOCKS_EXPOSED",
    "FEMALE_BREAST_EXPOSED", "FEMALE_GENITALIA_EXPOSED", "MALE_BREAST_EXPOSED",
    "ANUS_EXPOSED", "FEET_EXPOSED", "BELLY_COVERED", "FEET_COVERED",
    "ARMPITS_COVERED", "ARMPITS_EXPOSED", "FACE_MALE", "BELLY_EXPOSED",
    "MALE_GENITALIA_EXPOSED", "ANUS_COVERED", "FEMALE_BREAST_COVERED",
    "BUTTOCKS_COVERED"
]

/// Detect body parts in images using NudeNet CoreML model.
///
/// Input JSON: { "paths": [String], "model_path": String }
/// Output JSON: [{ "path": String, "detections": [{ "label": String, "confidence": Double }] }]
///
/// The NudeNet model outputs raw YOLO tensors [1, 22, 2100]. This function
/// performs score thresholding and greedy NMS post-processing, then returns
/// per-class detections with confidence scores.
@_cdecl("msw_detect_nsfw")
public func detectNsfw(_ jsonInput: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar> {
    let input = String(cString: jsonInput)
    guard let data = input.data(using: .utf8),
          let params = try? JSONDecoder().decode(NsfwParams.self, from: data) else {
        return strdup("[]")!
    }

    let modelURL = URL(fileURLWithPath: params.model_path)
    guard FileManager.default.fileExists(atPath: params.model_path) else {
        return strdup("[]")!
    }

    guard let mlModel = try? MLModel(contentsOf: modelURL),
          let vnModel = try? VNCoreMLModel(for: mlModel) else {
        return strdup("[]")!
    }

    var results: [[String: Any]] = []
    let numClasses = nudeNetLabels.count // 18
    let confThreshold: Float = 0.1
    let iouThreshold: Float = 0.45

    for imagePath in params.paths {
        autoreleasepool {
            let imageURL = URL(fileURLWithPath: imagePath)
            guard let imageSource = CGImageSourceCreateWithURL(imageURL as CFURL, nil),
                  let cgImage = CGImageSourceCreateImageAtIndex(imageSource, 0, nil) else {
                return
            }

            let request = VNCoreMLRequest(model: vnModel)
            request.imageCropAndScaleOption = .scaleFill

            let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])
            do {
                try handler.perform([request])
            } catch {
                return
            }

            // VNCoreMLRequest with a raw tensor model returns VNCoreMLFeatureValueObservation
            guard let featureResults = request.results as? [VNCoreMLFeatureValueObservation],
                  let firstResult = featureResults.first,
                  let multiArray = firstResult.featureValue.multiArrayValue else {
                return
            }

            // Shape: [1, 22, N] where 22 = 4 (cx,cy,w,h) + 18 (class scores)
            let shape = multiArray.shape.map { $0.intValue }
            let channels = shape.count == 3 ? shape[1] : (shape.count == 2 ? shape[0] : 0)
            let numDetections = shape.count == 3 ? shape[2] : (shape.count == 2 ? shape[1] : 0)
            guard channels == numClasses + 4, numDetections > 0 else { return }

            // Use safe subscript access — works regardless of data type (Float16/32/64)
            struct Detection {
                let cx: Float; let cy: Float; let w: Float; let h: Float
                let classIdx: Int; let score: Float
            }
            var candidates: [Detection] = []

            for j in 0..<numDetections {
                let idx: (Int) -> [NSNumber] = shape.count == 3
                    ? { row in [0 as NSNumber, row as NSNumber, j as NSNumber] }
                    : { row in [row as NSNumber, j as NSNumber] }

                let cx = multiArray[idx(0)].floatValue
                let cy = multiArray[idx(1)].floatValue
                let w  = multiArray[idx(2)].floatValue
                let h  = multiArray[idx(3)].floatValue

                var bestClass = 0
                var bestScore: Float = 0
                for c in 0..<numClasses {
                    let score = multiArray[idx(4 + c)].floatValue
                    if score > bestScore {
                        bestScore = score
                        bestClass = c
                    }
                }
                if bestScore >= confThreshold {
                    candidates.append(Detection(cx: cx, cy: cy, w: w, h: h,
                                                classIdx: bestClass, score: bestScore))
                }
            }

            // Greedy NMS per class
            candidates.sort { $0.score > $1.score }
            var keep: [Detection] = []
            var suppressed = [Bool](repeating: false, count: candidates.count)

            for i in 0..<candidates.count {
                if suppressed[i] { continue }
                let a = candidates[i]
                keep.append(a)
                for jj in (i+1)..<candidates.count {
                    if suppressed[jj] { continue }
                    let b = candidates[jj]
                    if a.classIdx != b.classIdx { continue }
                    let iou = computeIoU(a.cx, a.cy, a.w, a.h, b.cx, b.cy, b.w, b.h)
                    if iou > iouThreshold { suppressed[jj] = true }
                }
            }

            // Deduplicate to best-per-class for the result
            var bestPerClass: [String: Double] = [:]
            for det in keep {
                let label = nudeNetLabels[det.classIdx]
                let existing = bestPerClass[label] ?? 0
                if Double(det.score) > existing {
                    bestPerClass[label] = Double(det.score)
                }
            }

            let detections: [[String: Any]] = bestPerClass.map { label, conf in
                ["label": label, "confidence": conf]
            }

            results.append([
                "path": imagePath,
                "detections": detections
            ])
        }
    }

    guard let jsonData = try? JSONSerialization.data(withJSONObject: results),
          let jsonStr = String(data: jsonData, encoding: .utf8) else {
        return strdup("[]")!
    }
    return strdup(jsonStr)!
}

private func computeIoU(_ cx1: Float, _ cy1: Float, _ w1: Float, _ h1: Float,
                         _ cx2: Float, _ cy2: Float, _ w2: Float, _ h2: Float) -> Float {
    let x1a = cx1 - w1/2, y1a = cy1 - h1/2, x1b = cx1 + w1/2, y1b = cy1 + h1/2
    let x2a = cx2 - w2/2, y2a = cy2 - h2/2, x2b = cx2 + w2/2, y2b = cy2 + h2/2
    let interX = max(0, min(x1b, x2b) - max(x1a, x2a))
    let interY = max(0, min(y1b, y2b) - max(y1a, y2a))
    let interArea = interX * interY
    let union = w1 * h1 + w2 * h2 - interArea
    return union > 0 ? interArea / union : 0
}

// MARK: - Memory Management

/// Free a string allocated by this library. Must be called from Rust for every
/// string returned by msw_* functions to avoid memory leaks.
@_cdecl("msw_free_string")
public func freeString(_ ptr: UnsafeMutablePointer<CChar>) {
    free(ptr)
}
