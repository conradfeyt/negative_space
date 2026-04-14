// FileClassifier.swift — Rule-based file safety classification.
//
// This is the fallback classifier used when Foundation Models is unavailable.
// When the macOS 26 SDK is available, FoundationModelsClassifier will replace
// this for users with Apple Intelligence, while this remains the fallback.

import Foundation

struct FileInput: Codable {
    let path: String
    let name: String
    let size: UInt64
    let file_type: String
    let modified: String?  // ISO 8601 or date string from scan results
}

struct FileClassification: Codable {
    let path: String
    let safety: String        // "safe", "safe_stale", "safe_rebuild", "probably_safe", "risky", "unknown"
    let explanation: String
    let confidence: Float
}

// MARK: - Rule-Based Classifier (Always Available)

enum RuleBasedClassifier {
    /// Known-safe cache directories that are always regenerated.
    private static let safeCachePrefixes = [
        "/Library/Caches/com.apple",
        "/Library/Caches/CloudKit",
        "/Library/Developer/Xcode/DerivedData",
        "/Library/Developer/CoreSimulator",
        "/.npm/_cacache",
        "/.cache/",
        "/Library/Caches/Homebrew",
        "/Library/Caches/pip",
        "/Library/Caches/yarn",
        "/Library/Caches/Google",
        "/Library/Caches/Mozilla",
        "/Library/Caches/com.spotify",
        "/Library/Caches/com.microsoft",
        "/Library/Caches/Microsoft Edge",
        "/Library/Caches/com.todesktop",
        "/Library/Caches/SiriTTS",
        "/Library/Caches/node-gyp",
        "/Library/Caches/GeoServices",
    ]

    /// Extensions that are always safe to remove (generated/temporary).
    private static let safeExtensions: Set<String> = [
        "log", "tmp", "temp", "cache", "o", "pyc", "pyo",
        "class", "dex", "d", "gcda", "gcno",
    ]

    /// Extensions that may contain user data (risky to delete).
    private static let riskyExtensions: Set<String> = [
        "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pdf",
        "pages", "numbers", "key", "rtf", "txt", "md",
        "sqlite", "db", "realm",
        "psd", "ai", "sketch", "fig",
    ]

    /// Known safe directory patterns.
    private static let safeDirectoryPatterns = [
        "/node_modules/",
        "/.Trash/",
        "/DerivedData/",
        "/Build/Intermediates",
        "/Build/Products",
        "/__pycache__/",
        "/.gradle/caches/",
        "/target/debug/",
        "/target/release/",
        "/.cocoapods/repos/",
        "/GoogleUpdater/crx_cache/",
    ]

    /// Path-specific rules with custom explanations (checked before generic rules).
    /// Tuple: (path contains, safety, explanation, confidence)
    private static let pathRules: [(String, String, String, Float)] = [
        // Xcode & iOS development
        ("/Library/Developer/Xcode/iOS DeviceSupport/", "safe",
         "iOS device debug symbols. Xcode re-downloads for connected devices.", 0.95),
        ("/Library/Developer/Xcode/Archives/", "probably_safe",
         "Xcode build archives. Only needed if distributing these specific builds.", 0.85),
        ("/Library/Developer/CoreSimulator/Devices/", "safe",
         "iOS Simulator data. Delete unused simulators via Xcode > Window > Devices.", 0.90),
        ("/Library/Developer/Xcode/DerivedData/", "safe",
         "Xcode build cache. Regenerates on next build.", 0.95),

        // Android development
        ("/.android/avd/", "probably_safe",
         "Android emulator virtual device. Safe if you don't need this specific AVD.", 0.80),
        ("/Android/sdk/system-images/", "probably_safe",
         "Android SDK system image. Reinstall via Android Studio if needed.", 0.85),
        ("/.android/avd/", "probably_safe",
         "Android emulator data. Safe if you don't actively use this emulator.", 0.80),

        // Emulator snapshots & RAM dumps
        ("/snapshots/", "safe",
         "Emulator memory snapshot. Recreated automatically on next launch.", 0.92),
        ("/ram.bin", "safe",
         "Emulator RAM dump. Recreated on next emulator boot.", 0.92),

        // Virtual disk images
        ("/com.docker.docker/", "risky",
         "Docker virtual disk. Contains all Docker images and containers.", 0.90),
        ("/vm_bundles/", "risky",
         "Virtual machine bundle. May contain important VM data.", 0.85),

        // Homebrew
        ("/Homebrew/downloads/", "safe",
         "Homebrew download cache. Run 'brew cleanup' to clear safely.", 0.95),

        // Gradle
        ("/.gradle/caches/", "safe",
         "Gradle build cache. Regenerates on next build.", 0.92),
        ("/.gradle/wrapper/", "probably_safe",
         "Gradle wrapper distributions. Re-downloads when needed.", 0.88),

        // CocoaPods
        ("/.cocoapods/repos/", "safe",
         "CocoaPods spec repository cache. Re-clones with 'pod install'.", 0.92),

        // npm / node
        ("/.npm/", "safe",
         "npm package cache. Clear with 'npm cache clean --force'.", 0.95),
        ("/node_modules/", "safe",
         "Node.js dependencies. Reinstall with 'npm install'.", 0.95),
        ("/.nvm/versions/", "probably_safe",
         "Node.js version managed by nvm. Safe if you don't use this version.", 0.80),

        // Rust
        ("/.cargo/registry/", "safe",
         "Cargo crate cache. Re-downloads on next build.", 0.92),
        ("/.rustup/toolchains/", "probably_safe",
         "Rust toolchain. Safe to remove old versions you no longer use.", 0.80),
        ("/target/release/", "safe",
         "Rust build artifacts. Regenerates on next build.", 0.92),
        ("/target/debug/", "safe",
         "Rust debug build artifacts. Regenerates on next build.", 0.92),

        // Autodesk
        ("/Autodesk/", "probably_safe",
         "Autodesk application data. Logs and caches are safe; project files are not.", 0.70),

        // IDE state & caches
        ("/Application Support/Code/", "probably_safe",
         "VS Code application data. Extensions and cache can be large.", 0.70),
        ("/Application Support/Cursor/", "probably_safe",
         "Cursor editor data. Extensions and cache can be large.", 0.70),
        ("/state.vscdb.backup", "safe",
         "Editor state database backup. The primary copy is retained.", 0.90),
        ("/state.vscdb", "risky",
         "Editor state database. Contains workspace state and settings.", 0.85),

        // Cached installers / downloads
        ("/com.robotsandpencils.XcodesApp/", "safe",
         "Cached Xcode installer package. Re-downloads if needed.", 0.95),
        ("/webdeploy/production/", "safe",
         "Application web deployment cache. Regenerates automatically.", 0.90),

        // Negativ_ Archive
        ("/NegativeSpace/archive/", "archived",
         "Negativ_ Archive. Contains compressed copies of files you archived — originals were deleted. Manage from the Archive view.", 0.99),
        ("/NegativeSpace/vault/", "archived",
         "Negativ_ Vault. Contains securely stored files. Manage from the Sensitive Content view.", 0.99),
        ("/MyNegativeSpaceVault/", "archived",
         "Legacy Negativ_ Archive. Manage from the Archive view.", 0.99),

        // Affinity
        ("/Affinity", "risky",
         "Affinity application data. Autosaves may contain unsaved work.", 0.75),
        (".autosave", "risky",
         "Autosave file. May contain unsaved work — check the parent app first.", 0.70),
    ]

    /// Stale threshold: files not modified in this many days are considered stale.
    private static let staleDays: Double = 90

    /// Parse the modified string and return days since modification, or nil if unparseable.
    private static func daysSinceModified(_ modified: String?) -> Double? {
        guard let modified = modified, !modified.isEmpty else { return nil }
        // Try multiple date formats the frontend might provide
        let formatters: [DateFormatter] = {
            let iso = DateFormatter()
            iso.dateFormat = "yyyy-MM-dd'T'HH:mm:ss"
            iso.locale = Locale(identifier: "en_US_POSIX")
            let readable = DateFormatter()
            readable.dateFormat = "yyyy-MM-dd HH:mm:ss"
            readable.locale = Locale(identifier: "en_US_POSIX")
            return [iso, readable]
        }()
        for fmt in formatters {
            if let date = fmt.date(from: modified) {
                return max(0, Date().timeIntervalSince(date) / 86400)
            }
        }
        return nil
    }

    /// Refine a "safe" classification based on age. Caches/build artifacts that are
    /// recently used get "safe_rebuild" (safe but will cost rebuild time), while
    /// stale ones get "safe_stale" (safe and clearly unused).
    private static func refineSafetyByAge(_ base: FileClassification, file: FileInput) -> FileClassification {
        // Only refine cache/build classifications — not trash, not inherently safe files like .log
        let refinablePaths = [
            "/.gradle/", "/.cocoapods/", "/node_modules/", "/.npm/",
            "/target/debug/", "/target/release/", "/DerivedData/",
            "/CoreSimulator/", "/Build/Products", "/Build/Intermediates",
            "/.cargo/registry/", "/.rustup/toolchains/",
            "/Library/Caches/",
        ]
        let isRefinable = refinablePaths.contains { base.path.contains($0) }
        guard isRefinable && base.safety == "safe" else { return base }

        if let days = daysSinceModified(file.modified) {
            if days > staleDays {
                let months = Int(days / 30)
                let ageText = months > 0 ? "\(months) months" : "\(Int(days)) days"
                return FileClassification(
                    path: base.path,
                    safety: "safe_stale",
                    explanation: base.explanation + " Last modified \(ageText) ago.",
                    confidence: min(base.confidence + 0.05, 1.0)
                )
            } else {
                return FileClassification(
                    path: base.path,
                    safety: "safe_rebuild",
                    explanation: base.explanation + " Modified recently — deleting will require a re-download or rebuild.",
                    confidence: base.confidence
                )
            }
        } else {
            // No date available — don't refine, keep as "safe"
            return base
        }
    }

    static func classify(json: String) -> String {
        guard let data = json.data(using: .utf8),
              let files = try? JSONDecoder().decode([FileInput].self, from: data) else {
            return "[]"
        }

        let classifications = files.map { file -> FileClassification in
            let base = classifyFile(file)
            return refineSafetyByAge(base, file: file)
        }

        guard let resultData = try? JSONEncoder().encode(classifications),
              let resultString = String(data: resultData, encoding: .utf8) else {
            return "[]"
        }

        return resultString
    }

    private static func classifyFile(_ file: FileInput) -> FileClassification {
        let path = file.path
        let ext = file.file_type.lowercased()

        // Trash items are always safe
        if path.contains("/.Trash/") {
            return FileClassification(
                path: path,
                safety: "safe",
                explanation: "Already in Trash — safe to permanently remove.",
                confidence: 0.99
            )
        }

        // Check path-specific rules first (most specific)
        for rule in pathRules {
            if path.contains(rule.0) {
                return FileClassification(
                    path: path,
                    safety: rule.1,
                    explanation: rule.2,
                    confidence: rule.3
                )
            }
        }

        // Check safe cache paths
        for prefix in safeCachePrefixes {
            if path.contains(prefix) {
                return FileClassification(
                    path: path,
                    safety: "safe",
                    explanation: "Application cache — will be regenerated automatically when needed.",
                    confidence: 0.95
                )
            }
        }

        // Check safe directory patterns
        for pattern in safeDirectoryPatterns {
            if path.contains(pattern) {
                return FileClassification(
                    path: path,
                    safety: "safe",
                    explanation: "Build artifact or temporary directory — safe to remove.",
                    confidence: 0.90
                )
            }
        }

        // Check safe extensions
        if safeExtensions.contains(ext) {
            return FileClassification(
                path: path,
                safety: "safe",
                explanation: "Temporary or generated file that will be recreated as needed.",
                confidence: 0.85
            )
        }

        // Check risky extensions
        if riskyExtensions.contains(ext) {
            return FileClassification(
                path: path,
                safety: "risky",
                explanation: "May contain user-created content. Verify before deleting.",
                confidence: 0.80
            )
        }

        // Virtual disk images — context-dependent
        if ext == "qcow2" || ext == "vmdk" || ext == "vdi" || ext == "raw" {
            return FileClassification(
                path: path,
                safety: "probably_safe",
                explanation: "Virtual disk image. Safe if you don't need this virtual machine.",
                confidence: 0.70
            )
        }

        // DMG files in Application Support are usually cached installers
        if ext == "dmg" && path.contains("/Application Support/") {
            return FileClassification(
                path: path,
                safety: "safe",
                explanation: "Cached installer package. Can be re-downloaded if needed.",
                confidence: 0.88
            )
        }

        // JAR files in caches
        if ext == "jar" && (path.contains("/caches/") || path.contains("/transforms/")) {
            return FileClassification(
                path: path,
                safety: "safe",
                explanation: "Cached build dependency. Regenerates on next build.",
                confidence: 0.90
            )
        }

        // Pack files in git repos
        if ext == "pack" && path.contains("/.git/") {
            return FileClassification(
                path: path,
                safety: "probably_safe",
                explanation: "Git packfile. Can be regenerated with 'git gc' or re-clone.",
                confidence: 0.80
            )
        }

        // Default: unknown — empty explanation, don't show unhelpful generic text
        return FileClassification(
            path: path,
            safety: "unknown",
            explanation: "",
            confidence: 0.30
        )
    }
}
