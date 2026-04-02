use std::process::Command;

fn main() {
    // Declare custom cfg so Rust doesn't warn about it
    println!("cargo::rustc-check-cfg=cfg(has_swift_bridge)");

    tauri_build::build();

    // Compile the Swift intelligence bridge and link it.
    build_swift_bridge();
}

fn build_swift_bridge() {
    let swift_dir = std::path::Path::new("swift");
    if !swift_dir.exists() {
        return; // No Swift package — skip (e.g. CI without Xcode)
    }

    // Build the Swift static library
    let status = Command::new("swift")
        .args(["build", "-c", "release"])
        .current_dir(swift_dir)
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!("Warning: Swift bridge build failed (exit {}). AI features disabled.", s);
            return;
        }
        Err(e) => {
            eprintln!("Warning: Cannot run swift build ({}). AI features disabled.", e);
            return;
        }
    }

    // Find the static library
    let lib_dir = swift_dir.join(".build/arm64-apple-macosx/release");
    if !lib_dir.join("libNegativeSpaceIntelligence.a").exists() {
        eprintln!("Warning: libNegativeSpaceIntelligence.a not found. AI features disabled.");
        return;
    }

    // Tell cargo to link against the Swift static library
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=NegativeSpaceIntelligence");

    // Swift runtime libraries (required for any Swift static lib)
    let swift_lib_dir = get_swift_lib_dir();
    if let Some(dir) = swift_lib_dir {
        println!("cargo:rustc-link-search=native={}", dir);
    }

    // Link required system frameworks
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=AppKit");
    println!("cargo:rustc-link-lib=framework=CoreSpotlight");

    // Rebuild if Swift sources change
    println!("cargo:rerun-if-changed=swift/Sources/");
    println!("cargo:rerun-if-changed=swift/Package.swift");

    // Enable the feature flag so Rust code knows the bridge is available
    println!("cargo:rustc-cfg=has_swift_bridge");
}

fn get_swift_lib_dir() -> Option<String> {
    let output = Command::new("xcrun")
        .args(["--show-sdk-path"])
        .output()
        .ok()?;
    let sdk = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let lib_dir = format!("{}/usr/lib/swift", sdk);
    if std::path::Path::new(&lib_dir).exists() {
        Some(lib_dir)
    } else {
        None
    }
}
