// process_info.rs — Shared process name dictionaries.
//
// Provides two lookup tables used by both memory.rs and vitals.rs:
//   1. `get_process_dictionary()` — maps process names to (description, category)
//   2. `get_app_bundle_mappings()` — maps binary path substrings to app group names
//
// This module was extracted to eliminate duplication between memory.rs and
// vitals.rs, which previously maintained independent (diverging) copies.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Process description dictionary
// ---------------------------------------------------------------------------
// Maps process names to (description, category).
// category: "system", "app", "developer", "background", "networking", "security",
//           "storage", "display", "input", "audio", "icloud"

pub fn get_process_dictionary() -> HashMap<&'static str, (&'static str, &'static str)> {
    let mut dict = HashMap::new();

    // -- Kernel & core system --
    dict.insert(
        "kernel_task",
        (
            "macOS kernel -- manages thermal throttling, memory pressure, and hardware I/O",
            "system",
        ),
    );
    dict.insert(
        "launchd",
        (
            "macOS init system -- the first process, manages all daemons and agents",
            "system",
        ),
    );
    dict.insert(
        "WindowServer",
        (
            "macOS display compositor -- renders all windows, handles GPU compositing",
            "display",
        ),
    );
    dict.insert(
        "loginwindow",
        (
            "Login window manager -- handles user sessions and logout",
            "system",
        ),
    );
    dict.insert(
        "UserEventAgent",
        (
            "Handles user-level system events and notifications",
            "system",
        ),
    );
    dict.insert(
        "SystemUIServer",
        (
            "Menu bar and system UI elements (clock, Wi-Fi, battery)",
            "display",
        ),
    );
    dict.insert(
        "Dock",
        (
            "The Dock, Launchpad, Mission Control, and desktop management",
            "display",
        ),
    );
    dict.insert("Finder", ("File manager and desktop icon rendering", "app"));
    dict.insert(
        "NotificationCenter",
        (
            "Notification Center -- manages and displays system notifications",
            "display",
        ),
    );
    dict.insert(
        "ControlCenter",
        (
            "Control Center -- quick settings (Wi-Fi, Bluetooth, AirDrop, etc.)",
            "display",
        ),
    );

    // -- Spotlight / search --
    dict.insert(
        "mds",
        (
            "Spotlight metadata server -- coordinates search indexing",
            "storage",
        ),
    );
    dict.insert(
        "mds_stores",
        (
            "Spotlight index data store -- manages the search database",
            "storage",
        ),
    );
    dict.insert(
        "mdworker",
        (
            "Spotlight indexing worker -- crawls and indexes file content",
            "storage",
        ),
    );
    dict.insert(
        "mdworker_shared",
        (
            "Shared Spotlight indexing worker for background indexing",
            "storage",
        ),
    );
    dict.insert(
        "corespotlightd",
        (
            "Core Spotlight daemon -- manages app-contributed search results",
            "storage",
        ),
    );

    // -- Networking --
    dict.insert(
        "mDNSResponder",
        (
            "DNS resolver and Bonjour/mDNS service discovery",
            "networking",
        ),
    );
    dict.insert(
        "networkd",
        (
            "Network configuration daemon -- manages interfaces and routing",
            "networking",
        ),
    );
    dict.insert(
        "WiFiAgent",
        (
            "Wi-Fi menu bar agent and connection management",
            "networking",
        ),
    );
    dict.insert(
        "airportd",
        ("Wi-Fi hardware management daemon", "networking"),
    );
    dict.insert(
        "configd",
        (
            "System configuration daemon -- network settings, DNS, proxies",
            "networking",
        ),
    );
    dict.insert(
        "CommCenter",
        (
            "Cellular and telephony services (continuity, SMS relay)",
            "networking",
        ),
    );
    dict.insert(
        "rapportd",
        (
            "Rapport daemon -- device-to-device communication (AirDrop, Handoff)",
            "networking",
        ),
    );
    dict.insert(
        "sharingd",
        (
            "AirDrop, Handoff, and Shared Clipboard services",
            "networking",
        ),
    );
    dict.insert(
        "identityservicesd",
        ("iMessage and FaceTime identity verification", "networking"),
    );
    dict.insert(
        "netbiosd",
        (
            "NetBIOS name resolution for SMB/Windows network browsing",
            "networking",
        ),
    );
    dict.insert(
        "symptomsd",
        ("Network diagnostics and symptom reporting", "networking"),
    );

    // -- Security --
    dict.insert(
        "securityd",
        (
            "Security daemon -- manages keychain access, certificates, crypto",
            "security",
        ),
    );
    dict.insert(
        "trustd",
        (
            "Certificate trust evaluation -- validates SSL/TLS certificates",
            "security",
        ),
    );
    dict.insert(
        "opendirectoryd",
        (
            "Open Directory -- authentication, LDAP, user account services",
            "security",
        ),
    );
    dict.insert(
        "SecurityAgent",
        (
            "Authentication dialog UI (password prompts, Touch ID)",
            "security",
        ),
    );
    dict.insert(
        "endpointsecurityd",
        (
            "Endpoint Security framework -- monitors system events for security tools",
            "security",
        ),
    );
    dict.insert(
        "XprotectService",
        ("XProtect -- Apple's built-in malware scanner", "security"),
    );
    dict.insert(
        "MRT",
        ("Malware Removal Tool -- removes known malware", "security"),
    );
    dict.insert(
        "GatekeeperService",
        (
            "Gatekeeper -- verifies app signatures before first launch",
            "security",
        ),
    );
    dict.insert(
        "com.apple.ManagedClient.agent",
        ("MDM (Mobile Device Management) agent", "security"),
    );

    // -- File system & storage --
    dict.insert(
        "distnoted",
        (
            "Distributed notification center -- inter-process notifications",
            "system",
        ),
    );
    dict.insert("fseventsd", ("File system event daemon -- powers file watching (used by Spotlight, Time Machine, etc.)", "storage"));
    dict.insert(
        "fsck_apfs",
        ("APFS filesystem consistency checker", "storage"),
    );
    dict.insert(
        "diskmanagementd",
        (
            "Disk management daemon -- handles mounting, formatting, partitioning",
            "storage",
        ),
    );
    dict.insert(
        "revisiond",
        (
            "Document version management -- handles auto-save versions",
            "storage",
        ),
    );
    dict.insert("backupd", ("Time Machine backup daemon", "storage"));
    dict.insert("tmutil", ("Time Machine utility process", "storage"));

    // -- iCloud --
    dict.insert(
        "cloudd",
        (
            "iCloud sync daemon -- coordinates iCloud Drive, Photos, etc.",
            "icloud",
        ),
    );
    dict.insert(
        "bird",
        (
            "iCloud document sync -- handles iCloud Drive file transfers",
            "icloud",
        ),
    );
    dict.insert(
        "nsurlsessiond",
        (
            "URL session daemon -- handles background network transfers (iCloud, updates)",
            "icloud",
        ),
    );
    dict.insert(
        "CloudKeychainProxy",
        ("iCloud Keychain sync across devices", "icloud"),
    );
    dict.insert("cloudphotod", ("iCloud Photos sync daemon", "icloud"));
    dict.insert("cloudpaird", ("iCloud device pairing service", "icloud"));
    dict.insert(
        "itunescloudd",
        ("iTunes/Music iCloud library sync", "icloud"),
    );
    dict.insert("progressd", ("iCloud sync progress tracking", "icloud"));

    // -- Display & graphics --
    dict.insert(
        "coreaudiod",
        (
            "Core Audio daemon -- manages all audio input/output",
            "audio",
        ),
    );
    dict.insert(
        "audioclocksyncd",
        ("Audio clock synchronization between devices", "audio"),
    );
    dict.insert(
        "com.apple.audio.SandboxHelper",
        ("Audio sandbox helper for app isolation", "audio"),
    );
    dict.insert(
        "corebrightnessd",
        (
            "Display brightness management (auto-brightness, True Tone, Night Shift)",
            "display",
        ),
    );
    dict.insert(
        "colorsync",
        (
            "ColorSync -- manages color profiles for displays and printing",
            "display",
        ),
    );

    // -- Input --
    dict.insert(
        "TouchBarServer",
        ("Touch Bar rendering and input handling", "input"),
    );
    dict.insert(
        "talagent",
        ("Typing prediction and text analytics", "input"),
    );
    dict.insert(
        "PressureKitService",
        ("Force Touch trackpad pressure sensing", "input"),
    );

    // -- App support daemons --
    dict.insert(
        "cfprefsd",
        (
            "Preferences daemon -- manages UserDefaults/plist access for all apps",
            "system",
        ),
    );
    dict.insert(
        "lsd",
        (
            "Launch Services daemon -- file-to-app associations, 'Open With' menu",
            "system",
        ),
    );
    dict.insert(
        "coreservicesd",
        (
            "Core Services -- manages app launch, UTI registration",
            "system",
        ),
    );
    dict.insert(
        "iconservicesagent",
        (
            "Icon cache manager -- serves app icons to Finder and Dock",
            "display",
        ),
    );
    dict.insert(
        "containermanagerd",
        ("App sandbox container management", "system"),
    );
    dict.insert(
        "runningboardd",
        (
            "Process lifecycle management -- tracks app states",
            "system",
        ),
    );
    dict.insert(
        "dasd",
        (
            "Duet Activity Scheduler -- coordinates background tasks for battery life",
            "system",
        ),
    );
    dict.insert(
        "thermalmonitord",
        (
            "Thermal management -- monitors CPU/GPU temperature",
            "system",
        ),
    );
    dict.insert(
        "powerd",
        ("Power management daemon -- sleep, wake, battery", "system"),
    );
    dict.insert(
        "coreduetd",
        (
            "Duet daemon -- learns usage patterns for Siri suggestions and app prediction",
            "system",
        ),
    );
    dict.insert(
        "mediaremoted",
        (
            "Media remote daemon -- handles play/pause/skip from keyboard, AirPods, etc.",
            "audio",
        ),
    );
    dict.insert("logd", ("Unified logging system daemon", "system"));
    dict.insert("syslogd", ("System log daemon (legacy)", "system"));
    dict.insert(
        "diagnosticd",
        ("Diagnostics daemon -- crash reports, analytics", "system"),
    );
    dict.insert(
        "ReportCrash",
        (
            "Crash reporter -- generates .crash files for crashed processes",
            "system",
        ),
    );
    dict.insert(
        "spindump",
        (
            "Hang reporter -- captures stack traces when apps become unresponsive",
            "system",
        ),
    );
    dict.insert(
        "watchdogd",
        (
            "System watchdog -- monitors for system hangs and panics",
            "system",
        ),
    );
    dict.insert(
        "UserNotificationCenter",
        ("User notification display and management", "display"),
    );

    // -- Developer tools --
    dict.insert("com.apple.dt.Xcode", ("Xcode IDE", "developer"));
    dict.insert(
        "XCBBuildService",
        ("Xcode build service -- compiles projects", "developer"),
    );
    dict.insert(
        "IBAgent",
        (
            "Interface Builder agent -- renders storyboards",
            "developer",
        ),
    );
    dict.insert(
        "SourceKitService",
        (
            "Swift/ObjC code intelligence (autocomplete, syntax highlighting)",
            "developer",
        ),
    );
    dict.insert("swift-frontend", ("Swift compiler frontend", "developer"));
    dict.insert("clang", ("C/C++/ObjC compiler (LLVM)", "developer"));
    dict.insert("lldb", ("LLDB debugger", "developer"));
    dict.insert("Simulator", ("iOS/watchOS/tvOS Simulator", "developer"));
    dict.insert("node", ("Node.js runtime", "developer"));
    dict.insert("ruby", ("Ruby interpreter", "developer"));
    dict.insert("python3", ("Python 3 interpreter", "developer"));
    dict.insert("python", ("Python interpreter", "developer"));
    dict.insert("java", ("Java Virtual Machine", "developer"));
    dict.insert(
        "cargo",
        ("Rust package manager and build tool", "developer"),
    );
    dict.insert("rustc", ("Rust compiler", "developer"));
    dict.insert(
        "rust-analyzer",
        ("Rust language server (IDE support)", "developer"),
    );
    dict.insert("gopls", ("Go language server (IDE support)", "developer"));

    // -- Common apps --
    dict.insert("Safari", ("Safari web browser", "app"));
    dict.insert("Mail", ("Apple Mail email client", "app"));
    dict.insert("Messages", ("iMessage and SMS messaging", "app"));
    dict.insert("FaceTime", ("FaceTime video and audio calls", "app"));
    dict.insert("Music", ("Apple Music player", "app"));
    dict.insert("Photos", ("Apple Photos library and editor", "app"));
    dict.insert("Preview", ("Image and PDF viewer", "app"));
    dict.insert("TextEdit", ("Basic text editor", "app"));
    dict.insert("Terminal", ("macOS Terminal emulator", "app"));
    dict.insert(
        "Activity Monitor",
        ("System process and resource monitor", "app"),
    );
    dict.insert("System Preferences", ("macOS System Preferences", "app"));
    dict.insert(
        "System Settings",
        ("macOS System Settings (Ventura+)", "app"),
    );
    dict.insert("Slack", ("Slack team messaging", "app"));
    dict.insert("Spotify", ("Spotify music streaming", "app"));
    dict.insert("zoom.us", ("Zoom video conferencing", "app"));
    dict.insert("Discord", ("Discord voice and text chat", "app"));
    dict.insert("Figma", ("Figma design tool", "app"));
    dict.insert("Notion", ("Notion workspace", "app"));
    dict.insert("1Password", ("1Password password manager", "app"));

    // -- Docker --
    dict.insert(
        "com.docker.backend",
        (
            "Docker Desktop backend -- manages containers and images",
            "developer",
        ),
    );
    dict.insert(
        "com.docker.vmnetd",
        ("Docker network virtualization daemon", "developer"),
    );
    dict.insert(
        "vpnkit-bridge",
        ("Docker VPN networking bridge", "developer"),
    );
    dict.insert(
        "qemu-system-aarch64",
        ("QEMU VM running Docker's Linux kernel (ARM)", "developer"),
    );
    dict.insert(
        "qemu-system-x86_64",
        ("QEMU VM running Docker's Linux kernel (x86)", "developer"),
    );

    // -- Browsers (helper processes) --
    dict.insert(
        "com.apple.WebKit.WebContent",
        ("Safari/WebKit page renderer -- one per tab/frame", "app"),
    );
    dict.insert(
        "com.apple.WebKit.Networking",
        (
            "Safari/WebKit network handler -- manages HTTP requests",
            "app",
        ),
    );
    dict.insert(
        "com.apple.WebKit.GPU",
        (
            "Safari/WebKit GPU process -- handles graphics rendering",
            "app",
        ),
    );
    dict.insert(
        "Google Chrome Helper",
        ("Chrome tab/extension renderer process", "app"),
    );
    dict.insert(
        "Google Chrome Helper (GPU)",
        ("Chrome GPU rendering process", "app"),
    );
    dict.insert(
        "Google Chrome Helper (Renderer)",
        ("Chrome page rendering process -- one per tab", "app"),
    );

    dict
}

// ---------------------------------------------------------------------------
// Known app bundle mappings
// ---------------------------------------------------------------------------
// Maps binary path substrings to app group names.
// When a process command contains one of these, it's grouped under the app.

pub fn get_app_bundle_mappings() -> Vec<(&'static str, &'static str, &'static str)> {
    // (path_contains, group_name, description)
    vec![
        ("Google Chrome", "Google Chrome", "Web browser by Google"),
        ("Firefox", "Firefox", "Web browser by Mozilla"),
        (
            "Brave Browser",
            "Brave Browser",
            "Privacy-focused web browser",
        ),
        (
            "Microsoft Edge",
            "Microsoft Edge",
            "Web browser by Microsoft",
        ),
        ("Arc", "Arc", "Web browser by The Browser Company"),
        ("Opera", "Opera", "Web browser by Opera Software"),
        ("Vivaldi", "Vivaldi", "Web browser by Vivaldi Technologies"),
        ("Safari", "Safari", "Apple's web browser"),
        ("com.apple.WebKit", "Safari", "Apple's web browser"),
        ("Slack", "Slack", "Team messaging and collaboration"),
        ("Discord", "Discord", "Voice, video, and text chat"),
        ("Spotify", "Spotify", "Music streaming"),
        ("zoom.us", "Zoom", "Video conferencing"),
        (
            "Microsoft Teams",
            "Microsoft Teams",
            "Team collaboration by Microsoft",
        ),
        (
            "Microsoft Outlook",
            "Microsoft Outlook",
            "Email client by Microsoft",
        ),
        (
            "Microsoft Word",
            "Microsoft Word",
            "Word processor by Microsoft",
        ),
        (
            "Microsoft Excel",
            "Microsoft Excel",
            "Spreadsheet by Microsoft",
        ),
        (
            "Microsoft PowerPoint",
            "Microsoft PowerPoint",
            "Presentations by Microsoft",
        ),
        (
            "Visual Studio Code",
            "Visual Studio Code",
            "Code editor by Microsoft",
        ),
        (
            "Code Helper",
            "Visual Studio Code",
            "Code editor by Microsoft",
        ),
        ("Cursor", "Cursor", "AI-powered code editor"),
        (
            "Electron",
            "Electron App",
            "Application built with Electron framework",
        ),
        ("Xcode", "Xcode", "Apple's IDE for macOS/iOS development"),
        ("Docker", "Docker", "Container platform"),
        ("com.docker", "Docker", "Container platform"),
        ("qemu-system", "Docker", "Container platform"),
        ("vpnkit", "Docker", "Container platform"),
        ("Figma", "Figma", "Design and prototyping tool"),
        ("Notion", "Notion", "All-in-one workspace"),
        ("1Password", "1Password", "Password manager"),
        ("iTerm2", "iTerm2", "Terminal emulator"),
        (
            "Alacritty",
            "Alacritty",
            "GPU-accelerated terminal emulator",
        ),
        ("kitty", "Kitty", "GPU-accelerated terminal emulator"),
        ("Warp", "Warp", "Modern terminal"),
        ("Obsidian", "Obsidian", "Markdown knowledge base"),
        ("Linear", "Linear", "Project management tool"),
        ("Postman", "Postman", "API development platform"),
        ("TablePlus", "TablePlus", "Database management tool"),
        ("IntelliJ", "IntelliJ IDEA", "Java/Kotlin IDE by JetBrains"),
        ("PyCharm", "PyCharm", "Python IDE by JetBrains"),
        ("WebStorm", "WebStorm", "JavaScript IDE by JetBrains"),
        ("GoLand", "GoLand", "Go IDE by JetBrains"),
        ("CLion", "CLion", "C/C++ IDE by JetBrains"),
        (
            "Android Studio",
            "Android Studio",
            "Android development IDE",
        ),
        ("Notion Mail", "Notion Mail", "Email client by Notion"),
        ("Affinity", "Affinity", "Professional creative suite"),
        ("Blender", "Blender", "3D modeling and rendering"),
        ("FreeCAD", "FreeCAD", "Parametric 3D CAD"),
        ("Negative _", "Negative _", "This app"),
    ]
}
