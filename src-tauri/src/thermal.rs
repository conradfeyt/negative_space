// thermal.rs — Hardware Temperature Sensors for Negative _.
//
// Reads real per-sensor temperature data directly from the Apple SMC
// (System Management Controller) via the IOKit interface. No sudo required.
//
// DATA SOURCE:
//   The SMC exposes hundreds of 4-character keys. Temperature keys all start
//   with 'T' and return values in degrees Celsius. Two data types are used:
//     - `sp78`: signed 7.8 fixed-point (common on Intel Macs)
//     - `flt `: native f32 (common on Apple Silicon)
//   The `smc` crate handles both transparently via its `temperature()` method.
//
// KEY NAMING CONVENTIONS (Apple Silicon):
//   Tp*  = CPU P-core (performance) die temperatures
//   Te*  = CPU E-core (efficiency) die temperatures
//   Tg*  = GPU cluster temperatures
//   TB*  = Battery temperatures
//   TD*  = DRAM/memory controller temperatures
//   Ts*  = SSD/storage temperatures
//   TH*  = Heatpipe/heatsink temperatures
//   TW*  = Wireless module (Wi-Fi/Bluetooth)
//   TV*  = VRM (voltage regulator module) temperatures
//   Tf*  = Fan controller / airflow sensors
//   Ta*  = Ambient / case sensors
//   TN*  = NAND / flash storage
//
// FAN DATA:
//   Fan keys use the `F{n}*` namespace. The `smc` crate provides a Fan struct
//   with name, current/min/max RPM. We include fan data alongside temperatures
//   since they're intimately related (fans spin up to cool hot sensors).
//
// SENSOR CATEGORIZATION:
//   We group the raw 4-char keys into human-readable categories and provide
//   friendly display names. The frontend shows a curated summary (CPU avg,
//   GPU avg, hottest sensor) plus an expandable list of all sensors.

use four_char_code::FourCharCode;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Data structures returned to the frontend
// ---------------------------------------------------------------------------

/// A single temperature sensor reading.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThermalSensor {
    /// Raw SMC 4-character key, e.g. "Tp01", "Tg0V"
    pub key: String,
    /// Human-readable name, e.g. "CPU P-Core 1", "GPU Cluster 2"
    pub name: String,
    /// Category for grouping: "cpu", "gpu", "memory", "storage", "battery",
    /// "airflow", "ambient", "vrm", "wireless", "other"
    pub category: String,
    /// Temperature in degrees Celsius
    pub temp_celsius: f64,
}

/// A single fan reading.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FanReading {
    /// Fan index (0, 1, ...)
    pub id: u32,
    /// Fan name from SMC, e.g. "Left side", "Right side"
    pub name: String,
    /// Current speed in RPM
    pub current_rpm: f64,
    /// Minimum speed in RPM
    pub min_rpm: f64,
    /// Maximum speed in RPM
    pub max_rpm: f64,
    /// Current speed as percentage of range (0-100)
    pub percent: f64,
}

/// Category-level summary for the curated view.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CategorySummary {
    /// Category id: "cpu", "gpu", etc.
    pub category: String,
    /// Display label: "CPU", "GPU", etc.
    pub label: String,
    /// Average temperature across all sensors in this category
    pub avg_celsius: f64,
    /// Maximum temperature in this category
    pub max_celsius: f64,
    /// Number of sensors in this category
    pub sensor_count: u32,
}

/// The full thermal scan result.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThermalScanResult {
    /// All individual sensor readings, sorted by category then key
    pub sensors: Vec<ThermalSensor>,
    /// Per-category summaries (CPU, GPU, etc.), sorted by max temp descending
    pub summaries: Vec<CategorySummary>,
    /// Fan readings
    pub fans: Vec<FanReading>,
    /// The single hottest sensor — quick headline for the dashboard
    pub hottest_sensor: Option<ThermalSensor>,
    /// Overall system assessment based on the hottest reading
    pub assessment: String,
    /// Total number of sensors found
    pub sensor_count: u32,
    /// Chip model name from sysctl (e.g. "Apple M4 Pro")
    pub chip_name: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build a comprehensive list of known SMC temperature key names.
///
/// Apple doesn't publish these, but they're well-documented from empirical
/// observation across Mac models. We include both Apple Silicon keys (Tp*, Te*,
/// Tg*) and Intel-era keys (TC*, TG*, TN*) so the app works on any Mac.
///
/// Not all keys exist on every machine — we probe each one and skip KeyNotFound.
fn get_known_temperature_keys() -> Vec<String> {
    let mut keys = Vec::with_capacity(300);

    // --- CPU P-cores (Apple Silicon) ---
    // Keys: Tp01..Tp0z, Tp1a..Tp9z — typically 4-14 P-cores
    for prefix in &["Tp0", "Tp1", "Tp2", "Tp3"] {
        for suffix in &[
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "a", "b", "c", "d", "e", "f", "g",
            "h", "i", "j", "k", "l", "m", "n", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y",
            "z", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "P", "Q",
            "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
        ] {
            keys.push(format!("{}{}", prefix, suffix));
        }
    }

    // --- CPU E-cores (Apple Silicon) ---
    for prefix in &["Te0", "Te1", "Te2"] {
        for suffix in &[
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "a", "b", "c", "d", "e", "f", "g",
            "h", "i", "j", "k", "l", "m", "n", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y",
            "z", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J",
        ] {
            keys.push(format!("{}{}", prefix, suffix));
        }
    }

    // --- Intel CPU cores ---
    // TC0C..TC9C = individual cores, TC0P = proximity
    for i in 0..=9 {
        keys.push(format!("TC{}C", i));
        keys.push(format!("TC{}c", i));
    }
    keys.push("TC0P".to_string());
    keys.push("TCXC".to_string());
    keys.push("TCGC".to_string());
    keys.push("TCSA".to_string());
    keys.push("TCMX".to_string());

    // --- GPU (Apple Silicon) ---
    // Tg01..Tg9z — GPU cluster temperatures
    for prefix in &["Tg0", "Tg1", "Tg2"] {
        for suffix in &[
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "a", "b", "c", "d", "e", "f", "g",
            "h", "i", "j", "k", "l", "m", "n", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y",
            "z", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "P", "V",
            "W",
        ] {
            keys.push(format!("{}{}", prefix, suffix));
        }
    }

    // --- Intel GPU ---
    for i in 0..=3 {
        keys.push(format!("TG{}P", i));
        keys.push(format!("TG{}D", i));
        keys.push(format!("TG{}p", i));
    }

    // --- Battery ---
    keys.push("TB0T".to_string());
    keys.push("TB1T".to_string());
    keys.push("TB2T".to_string());
    keys.push("TB3T".to_string());
    keys.push("TBXT".to_string());

    // Battery gas gauge
    for suffix in &["a", "b", "c", "d", "e", "f", "1", "2", "3"] {
        keys.push(format!("TB0{}", suffix));
        keys.push(format!("TB1{}", suffix));
    }

    // --- Memory / DRAM ---
    for i in 0..=9 {
        keys.push(format!("TD0{}", i));
        keys.push(format!("TD1{}", i));
        keys.push(format!("TD2{}", i));
    }
    for suffix in &["a", "b", "c", "d", "e", "f"] {
        keys.push(format!("TD0{}", suffix));
        keys.push(format!("TD1{}", suffix));
        keys.push(format!("TD2{}", suffix));
    }
    // Tm keys for memory on some models
    for i in 0..=9 {
        keys.push(format!("Tm0{}", i));
        keys.push(format!("Tm1{}", i));
    }

    // --- Storage / SSD ---
    for i in 0..=9 {
        keys.push(format!("Ts0{}", i));
        keys.push(format!("Ts1{}", i));
    }
    keys.push("Ts0P".to_string());
    keys.push("Ts0p".to_string());
    // NAND
    for i in 0..=9 {
        keys.push(format!("TN0{}", i));
    }
    keys.push("TN0P".to_string());

    // --- Heatpipe / Heatsink ---
    for i in 0..=9 {
        keys.push(format!("TH0{}", i));
        keys.push(format!("TH1{}", i));
    }
    for suffix in &["a", "b", "c", "d", "x", "P"] {
        keys.push(format!("TH0{}", suffix));
    }

    // --- Fan controller / airflow ---
    for i in 0..=4 {
        keys.push(format!("TfC{}", i));
        keys.push(format!("Tf0{}", i));
    }

    // --- Ambient / case ---
    keys.push("TaLP".to_string());
    keys.push("TaRP".to_string());
    keys.push("TaLW".to_string());
    keys.push("TaRW".to_string());
    keys.push("Ta0P".to_string());
    keys.push("Ta1P".to_string());
    // Thunderbolt
    keys.push("ThLP".to_string());
    keys.push("ThRP".to_string());
    keys.push("ThLH".to_string());
    keys.push("ThRH".to_string());
    // Trackpad
    keys.push("TkBT".to_string()); // Trackpad base temp
                                   // Various ambient
    for suffix in &["0P", "1P", "2P", "0p"] {
        keys.push(format!("Ta{}", suffix));
    }

    // --- VRM / voltage regulators ---
    for i in 0..=9 {
        keys.push(format!("TV0{}", i));
        keys.push(format!("TV1{}", i));
    }
    for suffix in &["h0", "h1", "h2", "h3", "e0", "e1", "e2", "e3"] {
        keys.push(format!("TV{}", suffix));
    }

    // --- Wireless ---
    keys.push("TW0P".to_string());
    keys.push("TW0p".to_string());
    keys.push("TW1P".to_string());

    // --- Power supply ---
    keys.push("Tp0P".to_string()); // Might clash with CPU — we'll deduplicate by key
    keys.push("TPCD".to_string());

    // --- Charger ---
    keys.push("TCHC".to_string());

    // Deduplicate — some keys may appear in multiple generation patterns
    keys.sort();
    keys.dedup();

    keys
}

/// Read all temperature sensors and fans from the SMC.
///
/// IMPLEMENTATION NOTE: In a bundled .app, the SMC driver's GetKeyFromIndex
/// API (used by `smc_keys()`) returns NotPrivileged because macOS restricts
/// full key enumeration to signed/entitled processes. However, direct key
/// reads via GetKeyInfo + ReadKey work fine for known keys.
///
/// So instead of enumerating all keys, we probe a comprehensive list of
/// known Apple Silicon and Intel temperature key names. If a key exists on
/// this hardware, `temperature()` returns Ok(value); if not, we skip it.
/// This is the same approach used by iStat Menus and similar tools.
pub fn scan_thermal() -> Result<ThermalScanResult, String> {
    // Open a connection to the SMC. This calls IOServiceOpen internally.
    let smc = smc::SMC::new().map_err(|e| format!("Failed to connect to SMC: {:?}", e))?;

    // --- Probe known temperature keys ---
    // We try each key individually. Keys that don't exist on this hardware
    // will return KeyNotFound and be silently skipped.
    let known_keys = get_known_temperature_keys();
    let mut sensors: Vec<ThermalSensor> = Vec::new();

    for key_str in &known_keys {
        // Convert 4-char string to FourCharCode for SMC lookup.
        // RUST CONCEPT: `Into` trait converts &str → FourCharCode using the
        // From<&str> impl in the four_char_code crate. The string must be
        // exactly 4 bytes (ASCII).
        if key_str.len() != 4 {
            continue; // Safety: FourCharCode panics on non-4-byte strings
        }
        let fcc: FourCharCode = key_str.as_str().into();

        // Try to read the temperature — handles both sp78 and flt data types
        match smc.temperature(fcc) {
            Ok(temp) => {
                // Filter out garbage readings (below -10 or above 130 is suspect)
                if temp > -10.0 && temp < 130.0 && temp != 0.0 {
                    let (name, category) = classify_sensor(key_str);
                    sensors.push(ThermalSensor {
                        key: key_str.clone(),
                        name,
                        category,
                        temp_celsius: (temp * 10.0).round() / 10.0,
                    });
                }
            }
            Err(_) => {
                // Key doesn't exist on this hardware, or isn't a temperature — skip.
            }
        }
    }

    // Sort sensors: by category first, then by key name within category
    sensors.sort_by(|a, b| a.category.cmp(&b.category).then(a.key.cmp(&b.key)));

    // --- Build category summaries ---
    let summaries = build_summaries(&sensors);

    // --- Find the hottest sensor ---
    let hottest_sensor = sensors
        .iter()
        .max_by(|a, b| {
            a.temp_celsius
                .partial_cmp(&b.temp_celsius)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned();

    // --- Generate assessment ---
    let assessment = if let Some(ref hot) = hottest_sensor {
        let t = hot.temp_celsius;
        if t >= 100.0 {
            format!(
                "{} is critically hot at {}C. Thermal throttling likely active.",
                hot.name, t
            )
        } else if t >= 90.0 {
            format!(
                "{} is running very hot at {}C. Consider reducing workload.",
                hot.name, t
            )
        } else if t >= 80.0 {
            format!("{} is warm at {}C. Normal under heavy load.", hot.name, t)
        } else if t >= 60.0 {
            "Temperatures are normal. System is running well.".to_string()
        } else {
            "System is cool. All sensors within comfortable range.".to_string()
        }
    } else {
        "No temperature sensors found.".to_string()
    };

    // --- Read fans ---
    let fans = read_fans(&smc);

    let sensor_count = sensors.len() as u32;

    // --- Chip model name from sysctl ---
    // RUST CONCEPT: Command::output() runs the process and captures stdout.
    // We use the absolute path because bundled .app has minimal PATH.
    let chip_name = std::process::Command::new("/usr/sbin/sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Apple Silicon".to_string());

    Ok(ThermalScanResult {
        sensors,
        summaries,
        fans,
        hottest_sensor,
        assessment,
        sensor_count,
        chip_name,
    })
}

// ---------------------------------------------------------------------------
// Sensor classification
// ---------------------------------------------------------------------------

/// Map a raw SMC key to a human-readable name and category.
///
/// Apple doesn't publish an official key dictionary, so this is built from
/// empirical observation across Apple Silicon Macs (M1-M4). Intel keys follow
/// a slightly different naming scheme but the T* prefix convention holds.
fn classify_sensor(key: &str) -> (String, String) {
    let bytes = key.as_bytes();

    // Need at least 2 characters to classify
    if bytes.len() < 2 {
        return (key.to_string(), "other".to_string());
    }

    let prefix2 = &key[..2];
    let suffix = &key[2..];

    match prefix2 {
        // --- CPU temperatures ---
        "Tp" => {
            // P-core (performance) die temps. Suffixes like "01", "0i", "2G" etc.
            let core_id = suffix.trim_start_matches('0');
            let label = if core_id.is_empty() {
                "CPU P-Core".to_string()
            } else {
                format!("CPU P-Core {}", core_id)
            };
            (label, "cpu".to_string())
        }
        "Te" => {
            // E-core (efficiency) die temps
            let core_id = suffix.trim_start_matches('0');
            let label = if core_id.is_empty() {
                "CPU E-Core".to_string()
            } else {
                format!("CPU E-Core {}", core_id)
            };
            (label, "cpu".to_string())
        }
        "TC" => {
            // Intel-era CPU keys: TC0C = CPU core, TC1C = core 1, etc.
            // Also TCGC, TCXC on some models.
            if suffix.ends_with('C') || suffix.ends_with('c') {
                let id = suffix.trim_end_matches(|c| c == 'C' || c == 'c');
                if id == "0" || id.is_empty() {
                    ("CPU Core Average".to_string(), "cpu".to_string())
                } else {
                    (format!("CPU Core {}", id), "cpu".to_string())
                }
            } else {
                (format!("CPU {}", suffix), "cpu".to_string())
            }
        }

        // --- GPU temperatures ---
        "Tg" => {
            let cluster_id = suffix.trim_start_matches('0');
            let label = if cluster_id.is_empty() {
                "GPU".to_string()
            } else {
                format!("GPU Cluster {}", cluster_id)
            };
            (label, "gpu".to_string())
        }
        "TG" => {
            // Intel discrete GPU keys
            if suffix == "0P" || suffix == "0p" {
                ("GPU Proximity".to_string(), "gpu".to_string())
            } else {
                (format!("GPU {}", suffix), "gpu".to_string())
            }
        }

        // --- Battery temperatures ---
        "TB" => {
            if suffix == "0T" || suffix == "0t" {
                ("Battery".to_string(), "battery".to_string())
            } else if suffix.starts_with('1') || suffix.starts_with('2') {
                (format!("Battery Cell {}", suffix), "battery".to_string())
            } else {
                (format!("Battery {}", suffix), "battery".to_string())
            }
        }

        // --- Memory/DRAM temperatures ---
        "TD" | "Tm" => {
            let id = suffix.trim_start_matches('0');
            if id.is_empty() {
                ("Memory".to_string(), "memory".to_string())
            } else {
                (format!("Memory {}", id), "memory".to_string())
            }
        }

        // --- Storage temperatures ---
        "Ts" | "TN" => {
            let id = suffix.trim_start_matches('0');
            if id.is_empty() {
                ("SSD".to_string(), "storage".to_string())
            } else {
                (format!("SSD {}", id), "storage".to_string())
            }
        }

        // --- Heatpipe / heatsink ---
        "TH" => {
            let id = suffix.trim_start_matches('0');
            if id.is_empty() {
                ("Heatsink".to_string(), "airflow".to_string())
            } else {
                (format!("Heatsink {}", id), "airflow".to_string())
            }
        }

        // --- Fan controller / airflow ---
        "Tf" => {
            let id = suffix.trim_start_matches('0');
            (
                format!("Fan Controller {}", id).trim().to_string(),
                "airflow".to_string(),
            )
        }

        // --- Ambient / case sensors ---
        "Ta" => {
            if suffix.starts_with('L') || suffix.starts_with('l') {
                ("Ambient Left".to_string(), "ambient".to_string())
            } else if suffix.starts_with('R') || suffix.starts_with('r') {
                ("Ambient Right".to_string(), "ambient".to_string())
            } else if suffix.starts_with('P') || suffix.starts_with('p') {
                ("Ambient".to_string(), "ambient".to_string())
            } else {
                (format!("Ambient {}", suffix), "ambient".to_string())
            }
        }

        // --- VRM / voltage regulators ---
        "TV" => {
            let id = suffix.trim_start_matches('0');
            if id.is_empty() {
                ("VRM".to_string(), "vrm".to_string())
            } else {
                (format!("VRM {}", id), "vrm".to_string())
            }
        }

        // --- Wireless ---
        "TW" => {
            if suffix == "0P" || suffix == "0p" {
                ("Airport Proximity".to_string(), "wireless".to_string())
            } else {
                (format!("Wireless {}", suffix), "wireless".to_string())
            }
        }

        // --- Thunderbolt ---
        "Th" => {
            if suffix.starts_with('L') || suffix.starts_with('l') {
                ("Thunderbolt Left".to_string(), "ambient".to_string())
            } else if suffix.starts_with('R') || suffix.starts_with('r') {
                ("Thunderbolt Right".to_string(), "ambient".to_string())
            } else {
                (format!("Thunderbolt {}", suffix), "ambient".to_string())
            }
        }

        _ => {
            // Check for some specific well-known keys
            match key {
                "Ts0P" | "Ts0p" => ("SSD Proximity".to_string(), "storage".to_string()),
                "TaLP" | "TaRP" => ("Ambient".to_string(), "ambient".to_string()),
                _ => {
                    // Fallback: use the raw key as the name
                    (format!("Sensor {}", key), "other".to_string())
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Category summaries
// ---------------------------------------------------------------------------

/// Build per-category summaries from the sensor list.
fn build_summaries(sensors: &[ThermalSensor]) -> Vec<CategorySummary> {
    // RUST CONCEPT: We use a HashMap to group sensors by category, then
    // compute avg/max/count for each group.
    let mut groups: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();

    for s in sensors {
        groups
            .entry(s.category.clone())
            .or_default()
            .push(s.temp_celsius);
    }

    let mut summaries: Vec<CategorySummary> = groups
        .into_iter()
        .map(|(category, temps)| {
            let sum: f64 = temps.iter().sum();
            let count = temps.len() as f64;
            let max = temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            CategorySummary {
                label: category_label(&category),
                category,
                avg_celsius: (sum / count * 10.0).round() / 10.0,
                max_celsius: max,
                sensor_count: temps.len() as u32,
            }
        })
        .collect();

    // Sort by max temperature descending — hottest categories first
    summaries.sort_by(|a, b| {
        b.max_celsius
            .partial_cmp(&a.max_celsius)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    summaries
}

/// Map category id to display label.
fn category_label(category: &str) -> String {
    match category {
        "cpu" => "CPU".to_string(),
        "gpu" => "GPU".to_string(),
        "memory" => "Memory".to_string(),
        "storage" => "Storage".to_string(),
        "battery" => "Battery".to_string(),
        "airflow" => "Airflow".to_string(),
        "ambient" => "Ambient".to_string(),
        "vrm" => "Power / VRM".to_string(),
        "wireless" => "Wireless".to_string(),
        other => other.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Fan readings
// ---------------------------------------------------------------------------

/// Read all fan speeds from the SMC.
///
/// APPROACH: We first try the `smc` crate's `fans()` method, which reads
/// the `FNum` key (fan count) and then `F{n}ID` (fan descriptor) for each.
/// These are direct key reads — NOT index-based enumeration — so they
/// should work without privileges.
///
/// If `fans()` fails (some Apple Silicon Macs don't expose FNum the same
/// way), we fall back to probing known fan keys directly: F0Ac, F1Ac, etc.
fn read_fans(smc: &smc::SMC) -> Vec<FanReading> {
    // --- Attempt 1: Use the crate's built-in fans() API ---
    match smc.fans() {
        Ok(fans) if !fans.is_empty() => {
            eprintln!("[thermal] fans() returned {} fan(s)", fans.len());
            let results: Vec<FanReading> = fans
                .iter()
                .filter_map(|fan| {
                    let current = match fan.current_speed() {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("[thermal] Fan {} current_speed failed: {:?}", fan.id(), e);
                            return None;
                        }
                    };
                    let min = fan.min_speed().unwrap_or(0.0);
                    let max = fan.max_speed().unwrap_or(1.0);
                    let pct = if max > min {
                        ((current - min) / (max - min) * 100.0).clamp(0.0, 100.0)
                    } else {
                        0.0
                    };

                    eprintln!(
                        "[thermal] Fan {}: name='{}', current={:.1}, min={:.1}, max={:.1}",
                        fan.id(),
                        fan.name(),
                        current,
                        min,
                        max
                    );

                    Some(FanReading {
                        id: fan.id(),
                        name: fan.name().to_string(),
                        current_rpm: (current * 10.0).round() / 10.0,
                        min_rpm: (min * 10.0).round() / 10.0,
                        max_rpm: (max * 10.0).round() / 10.0,
                        percent: (pct * 10.0).round() / 10.0,
                    })
                })
                .collect();

            if !results.is_empty() {
                return results;
            }
            eprintln!("[thermal] fans() returned fans but all reads failed, trying fallback");
        }
        Ok(_) => {
            eprintln!("[thermal] fans() returned 0 fans, trying fallback");
        }
        Err(e) => {
            eprintln!("[thermal] fans() failed: {:?}, trying fallback", e);
        }
    }

    // --- Attempt 2: Probe known fan keys directly ---
    // Fan keys follow the pattern F{n}Ac (actual/current speed), F{n}Mn (min),
    // F{n}Mx (max). Most Macs have 0-3 fans. We probe up to 8 just in case.
    eprintln!("[thermal] Probing fan keys directly (F0Ac..F7Ac)");
    let mut results = Vec::new();

    for i in 0u32..8 {
        // Read current speed: F{i}Ac
        let ac_key: FourCharCode = format!("F{}Ac", i).as_str().into();
        let current = match smc.read_key::<f64>(ac_key) {
            Ok(v) => v,
            Err(_) => {
                // No more fans at this index
                if i == 0 {
                    eprintln!("[thermal] F0Ac not found — machine may have no fans");
                }
                break;
            }
        };

        // Read min speed: F{i}Mn
        let mn_key: FourCharCode = format!("F{}Mn", i).as_str().into();
        let min = smc.read_key::<f64>(mn_key).unwrap_or(0.0);

        // Read max speed: F{i}Mx
        let mx_key: FourCharCode = format!("F{}Mx", i).as_str().into();
        let max = smc.read_key::<f64>(mx_key).unwrap_or(1.0);

        let pct = if max > min {
            ((current - min) / (max - min) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        // Try to get fan name from F{i}ID, fall back to generic name
        let name = match i {
            0 => "Left Fan".to_string(),
            1 => "Right Fan".to_string(),
            _ => format!("Fan {}", i),
        };

        eprintln!(
            "[thermal] Fallback Fan {}: current={:.1}, min={:.1}, max={:.1}",
            i, current, min, max
        );

        results.push(FanReading {
            id: i,
            name,
            current_rpm: (current * 10.0).round() / 10.0,
            min_rpm: (min * 10.0).round() / 10.0,
            max_rpm: (max * 10.0).round() / 10.0,
            percent: (pct * 10.0).round() / 10.0,
        });
    }

    eprintln!("[thermal] Fallback found {} fan(s)", results.len());
    results
}
