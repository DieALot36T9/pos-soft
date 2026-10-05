//! The scale profiles that ship with the app (rule HW-05). Each one follows the vendor's
//! published protocol (Reuse Map: no Rust crate covers scale protocols).
//! Shops can add their own profiles in settings.

use super::{ScaleMode, ScaleProfile, WeightUnit};

/// The ready-made profiles, as (id, profile). The id is fixed; the screen shows a name
/// for it from its text file, e.g. "generic_continuous" → "Generic continuous".
pub fn builtin_scale_profiles() -> Vec<(String, ScaleProfile)> {
    vec![
        ("generic_continuous".to_string(), generic_continuous()),
        ("cas".to_string(), cas_type()),
        ("toledo".to_string(), toledo_type()),
    ]
}

/// Any scale that keeps sending lines like "  1.234 kg". It has no stable flag,
/// so the weight counts as stable after 3 equal readings.
fn generic_continuous() -> ScaleProfile {
    ScaleProfile {
        mode: ScaleMode::Continuous,
        pattern: r"(?i)(?P<value>[-+]?\s*\d+(?:\.\d+)?)\s*(?P<unit>kg|lbs?|g)\b".to_string(),
        default_unit: WeightUnit::Kg,
        stable_marker: None,
        stable_repeats: 3,
    }
}

/// CAS scales in continuous mode send lines like "ST,GS,+  1.234kg":
/// ST = stable (US = unstable, OL = overload), GS = gross weight (NT = net), then the weight.
fn cas_type() -> ScaleProfile {
    ScaleProfile {
        mode: ScaleMode::Continuous,
        pattern: r"^(?:ST|US|OL),(?:GS|NT|TR),(?P<value>[-+]?\s*\d+(?:\.\d+)?)\s*(?P<unit>kg|lb|g)"
            .to_string(),
        default_unit: WeightUnit::Kg,
        stable_marker: Some("ST,".to_string()),
        stable_repeats: 1,
    }
}

/// Mettler-Toledo 8217 protocol: the app sends "W"; the scale answers STX "01.234" CR.
/// While the weight is moving it answers STX "?" and a status byte instead, which has
/// no number, so a weight answer is always a stable one.
fn toledo_type() -> ScaleProfile {
    ScaleProfile {
        mode: ScaleMode::Request {
            command: b"W".to_vec(),
        },
        pattern: r"^(?P<value>\d+\.\d+)$".to_string(),
        default_unit: WeightUnit::Kg,
        stable_marker: None,
        stable_repeats: 1,
    }
}
