// SummaryGenerator.swift — Template-based scan summary generation.
//
// Generates human-readable summaries of scan results. When Foundation Models
// becomes available, this will be replaced with LLM-generated summaries.
// The template version is still useful and accurate — it just lacks the
// natural conversational tone of the LLM.

import Foundation

struct DomainResult: Codable {
    let domain: String
    let item_count: Int
    let total_size: UInt64
}

struct ScanSummaryInput: Codable {
    let domains: [DomainResult]
    let total_reclaimable: UInt64
}

struct ScanSummaryOutput: Codable {
    let summary: String
    let ai_generated: Bool
}

enum TemplateSummaryGenerator {
    static func generate(json: String) -> String {
        guard let data = json.data(using: .utf8),
              let input = try? JSONDecoder().decode(ScanSummaryInput.self, from: data) else {
            let fallback = ScanSummaryOutput(summary: "", ai_generated: false)
            return encode(fallback)
        }

        // Sort domains by size descending
        let sorted = input.domains
            .filter { $0.total_size > 0 }
            .sorted { $0.total_size > $1.total_size }

        guard !sorted.isEmpty else {
            let output = ScanSummaryOutput(summary: "No reclaimable space found.", ai_generated: false)
            return encode(output)
        }

        var parts: [String] = []

        // Mention the top 2-3 domains
        let top = Array(sorted.prefix(3))
        for (i, domain) in top.enumerated() {
            let size = formatBytes(domain.total_size)
            let name = friendlyName(domain.domain)
            if i == 0 {
                parts.append("\(name) is using \(size) and is the biggest opportunity for reclaiming space")
            } else {
                parts.append("\(name) accounts for \(size)")
            }
        }

        let totalSize = formatBytes(input.total_reclaimable)
        var summary = parts.joined(separator: ". ") + "."
        summary += " In total, \(totalSize) can be reviewed for cleanup."

        let output = ScanSummaryOutput(summary: summary, ai_generated: false)
        return encode(output)
    }

    private static func friendlyName(_ domain: String) -> String {
        switch domain {
        case "caches": return "Application caches"
        case "logs": return "Log files"
        case "largeFiles": return "Large files"
        case "apps": return "Application leftovers"
        case "browsers": return "Browser data"
        case "trash": return "Trash"
        case "docker": return "Docker"
        case "security": return "Security findings"
        case "duplicates": return "Duplicate files"
        default: return domain.capitalized
        }
    }

    private static func formatBytes(_ bytes: UInt64) -> String {
        let gb = Double(bytes) / 1_073_741_824
        let mb = Double(bytes) / 1_048_576
        if gb >= 1.0 {
            return String(format: "%.1f GB", gb)
        } else {
            return String(format: "%.0f MB", mb)
        }
    }

    private static func encode(_ output: ScanSummaryOutput) -> String {
        guard let data = try? JSONEncoder().encode(output),
              let str = String(data: data, encoding: .utf8) else {
            return "{\"summary\":\"\",\"ai_generated\":false}"
        }
        return str
    }
}
