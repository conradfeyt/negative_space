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
}

struct FileClassification: Codable {
    let path: String
    let safety: String        // "safe", "probably_safe", "risky"
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
    ]

    static func classify(json: String) -> String {
        guard let data = json.data(using: .utf8),
              let files = try? JSONDecoder().decode([FileInput].self, from: data) else {
            return "[]"
        }

        let classifications = files.map { classifyFile($0) }

        guard let resultData = try? JSONEncoder().encode(classifications),
              let resultString = String(data: resultData, encoding: .utf8) else {
            return "[]"
        }

        return resultString
    }

    private static func classifyFile(_ file: FileInput) -> FileClassification {
        let path = file.path
        let ext = file.file_type.lowercased()

        // Check safe cache paths
        for prefix in safeCachePrefixes {
            if path.contains(prefix) {
                return FileClassification(
                    path: path,
                    safety: "safe",
                    explanation: "Application cache -- will be regenerated automatically when needed.",
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
                    explanation: "Build artifact or temporary directory -- safe to remove.",
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

        // Trash items are always safe
        if path.contains("/.Trash/") {
            return FileClassification(
                path: path,
                safety: "safe",
                explanation: "Already in Trash -- safe to permanently remove.",
                confidence: 0.99
            )
        }

        // Default: probably safe for large old files
        return FileClassification(
            path: path,
            safety: "probably_safe",
            explanation: "Review before deleting. Check if any application depends on this file.",
            confidence: 0.50
        )
    }
}
