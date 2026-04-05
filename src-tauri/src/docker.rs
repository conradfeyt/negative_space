// docker.rs — Docker detection, info gathering, and cleanup commands.
//
// Checks for Docker installation, gathers image/disk info from the Docker
// daemon, and provides a prune command for cleanup.

use crate::commands::{CleanResult, DockerInfo, DockerItem};

/// Quick check: is the Docker CLI binary present on disk?
/// No daemon contact — just checks common install paths + PATH.
#[tauri::command]
pub async fn is_docker_installed() -> bool {
    ["/usr/local/bin/docker", "/opt/homebrew/bin/docker"]
        .iter()
        .any(|p| std::path::Path::new(p).exists())
        || std::process::Command::new("which")
            .arg("docker")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

/// Check if Docker is installed and, if so, gather image/disk info.
#[tauri::command]
pub async fn get_docker_info() -> Result<DockerInfo, String> {
    // 1. Check if docker is installed by looking in common locations.
    // Tauri apps launched from the dock may have a minimal PATH that
    // doesn't include /usr/local/bin or /opt/homebrew/bin.
    let docker_bin = ["/usr/local/bin/docker", "/opt/homebrew/bin/docker"]
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|s| s.to_string())
        .or_else(|| {
            // Fall back to PATH lookup.
            std::process::Command::new("which")
                .arg("docker")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        });

    let docker_bin = match docker_bin {
        Some(bin) => bin,
        None => {
            return Ok(DockerInfo {
                installed: false,
                running: false,
                images: vec![],
                total_reclaimable: String::new(),
                disk_usage_raw: String::new(),
            });
        }
    };

    // 2. Get `docker system df` output to check if daemon is running.
    let df_output = std::process::Command::new(&docker_bin)
        .args(["system", "df"])
        .output();
    let disk_usage_raw = match df_output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => {
            // Docker is installed but daemon is not running.
            return Ok(DockerInfo {
                installed: true,
                running: false,
                images: vec![],
                total_reclaimable: String::new(),
                disk_usage_raw: String::new(),
            });
        }
    };

    // Try to extract reclaimable info from the last column of each row.
    let mut total_reclaimable = String::new();
    for line in disk_usage_raw.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // The "RECLAIMABLE" column is typically the last one and looks like
        // "1.23GB (50%)" — we just grab whatever is there.
        if let Some(_last) = parts.last() {
            if !total_reclaimable.is_empty() {
                total_reclaimable.push_str(", ");
            }
            // Grab the last two tokens to capture "1.23GB (50%)"
            let reclaim_parts: Vec<&str> = parts.iter().rev().take(2).copied().collect();
            total_reclaimable.push_str(
                &reclaim_parts
                    .into_iter()
                    .rev()
                    .collect::<Vec<&str>>()
                    .join(" "),
            );
        }
    }

    // 3. Get docker images.
    let images_output = std::process::Command::new(&docker_bin)
        .args([
            "images",
            "--format",
            "{{.Repository}}:{{.Tag}}\t{{.Size}}\t{{.ID}}",
        ])
        .output();

    let mut images: Vec<DockerItem> = Vec::new();
    if let Ok(o) = images_output {
        if o.status.success() {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines() {
                // Each line: "name:tag\tsize\tid"
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 3 {
                    images.push(DockerItem {
                        name: parts[0].to_string(),
                        size: parts[1].to_string(),
                        id: parts[2].to_string(),
                        item_type: "image".to_string(),
                    });
                }
            }
        }
    }

    Ok(DockerInfo {
        installed: true,
        running: true,
        images,
        total_reclaimable,
        disk_usage_raw,
    })
}

/// Run `docker system prune -f` (dangling resources only).
fn run_docker_prune_dangling() -> CleanResult {
    run_docker_prune(&["system", "prune", "-f"])
}

/// Run `docker system prune -f -a` (all unused resources, including images).
fn run_docker_prune_all() -> CleanResult {
    run_docker_prune(&["system", "prune", "-f", "-a"])
}

/// Shared implementation: run a `docker` prune command with the given args.
fn run_docker_prune(args: &[&str]) -> CleanResult {
    let output = std::process::Command::new("docker").args(args).output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();

            // Docker prune output typically ends with a line like:
            // "Total reclaimed space: 1.23GB"
            let freed_display = stdout
                .lines()
                .find(|line| line.contains("reclaimed space"))
                .unwrap_or("Total reclaimed space: 0B")
                .to_string();

            CleanResult {
                success: true,
                freed_bytes: 0, // Docker doesn't give us exact bytes easily
                deleted_count: 0,
                errors: vec![freed_display], // We repurpose errors to carry the message
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            CleanResult {
                success: false,
                freed_bytes: 0,
                deleted_count: 0,
                errors: vec![format!("docker prune failed: {}", stderr)],
            }
        }
        Err(e) => CleanResult {
            success: false,
            freed_bytes: 0,
            deleted_count: 0,
            errors: vec![format!("Failed to run docker: {}", e)],
        },
    }
}

/// Run `docker system prune` to free unused Docker resources.
/// If `prune_all` is true, also removes all unused images (not just dangling ones).
#[tauri::command]
pub async fn clean_docker(prune_all: bool) -> CleanResult {
    if prune_all {
        run_docker_prune_all()
    } else {
        run_docker_prune_dangling()
    }
}
