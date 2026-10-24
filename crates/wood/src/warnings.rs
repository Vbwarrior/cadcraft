//! WoodCraft Phase 7: the warnings engine (`plans/Wood_Workflow_Scope.md` "Warnings").
//! Pure, stateless checks: given a part's properties (and, where needed, live settings or its
//! outline's physical size), return the warnings that currently apply. A warning is computed
//! fresh every time, never stored — "appears when found, disappears when fixed." Each warning
//! carries a stable `id` so the same real-world problem gets the same id across repeated
//! computation, letting a caller track "the user already acknowledged this one" (session-only
//! state, kept outside this crate — see `crates/engine/src/lib.rs`'s
//! `Session::acknowledged_wood_warnings`).

use crate::data::{JoineryEntry, Part};
use crate::settings_data::WoodSettings;

/// Which of the three kinds a warning is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarningKind {
    Fastener,
    JointFit,
    BuildSize,
}

/// One current warning for one part.
#[derive(Clone, Debug, PartialEq)]
pub struct Warning {
    /// Stable across repeated computation for the same underlying problem, so a session can
    /// track which warnings the user has already acknowledged.
    pub id: String,
    pub kind: WarningKind,
    pub part_guid: String,
    pub message: String,
}

fn fastener_warning(part_guid: &str, feature: &str, depth: f64, thickness: f64) -> Warning {
    Warning {
        id: format!("fastener:{part_guid}:{feature}"),
        kind: WarningKind::Fastener,
        part_guid: part_guid.to_string(),
        message: format!("{feature} goes {:.3} past the part's thickness ({thickness:.3}); it would come through the other side", depth - thickness),
    }
}

/// Fastener warnings: a hole deep enough, or a nail long enough, to come out the other side of
/// the part (`Wood_Workflow_Scope.md`: "a fastener that goes through a part"). `Hole-Through` is
/// deliberately exempt — a through hole is supposed to go all the way through.
pub fn fastener_warnings(part_guid: &str, part: &Part) -> Vec<Warning> {
    let mut out = Vec::new();
    let thickness = part.thickness;

    let hole_groups: [(&str, &[crate::data::HoleEntry]); 6] = [
        ("Hole-Blind", &part.hole_blind),
        ("Hole-Counterbore", &part.hole_counterbore),
        ("Hole-Countersink", &part.hole_countersink),
        ("Hole-Pocket-Screw", &part.hole_pocket_screw),
        ("Hole-Shelf-Pin", &part.hole_shelf_pin),
        ("Hole-Dowel", &part.hole_dowel),
    ];
    for (name, entries) in hole_groups {
        for (i, entry) in entries.iter().enumerate() {
            if let Some(depth) = entry.depth
                && depth > thickness
            {
                out.push(fastener_warning(part_guid, &format!("{name} #{i}"), depth, thickness));
            }
        }
    }
    for (i, nail) in part.nail.iter().enumerate() {
        if nail.length > thickness {
            out.push(fastener_warning(part_guid, &format!("Nail #{i}"), nail.length, thickness));
        }
    }
    out
}

/// The gap a joinery entry's own `Tolerance` field is compared against: `minimum_joint_spacing`
/// is the floor, and `joint_tolerance_by_type` (keyed by the part's `Joint type` joint code, if
/// it has one) is the extra room allowed above that floor
/// (`Wood_Workflow_Scope.md`: "Tolerance limit per joint type: extra gap above the minimum").
fn joint_fit_warning(
    part_guid: &str,
    kind_name: &str,
    i: usize,
    entry: &JoineryEntry,
    settings: &WoodSettings,
    joint_code: Option<&str>,
) -> Option<Warning> {
    let tolerance = entry.tolerance?;
    let minimum = settings.minimum_joint_spacing;
    if tolerance < minimum {
        return Some(Warning {
            id: format!("jointfit:{part_guid}:{kind_name}:{i}"),
            kind: WarningKind::JointFit,
            part_guid: part_guid.to_string(),
            message: format!("{kind_name} #{i}'s tolerance ({tolerance:.3}) is tighter than the minimum joint spacing ({minimum:.3})"),
        });
    }
    let extra = joint_code.and_then(|code| settings.joint_tolerance_by_type.get(code)).copied();
    if let Some(extra) = extra {
        let max = minimum + extra;
        if tolerance > max {
            return Some(Warning {
                id: format!("jointfit:{part_guid}:{kind_name}:{i}"),
                kind: WarningKind::JointFit,
                part_guid: part_guid.to_string(),
                message: format!("{kind_name} #{i}'s tolerance ({tolerance:.3}) is looser than this joint type allows ({max:.3})"),
            });
        }
    }
    None
}

/// Joint fit warnings: a Mortise or Tenon entry whose own `Tolerance` sits outside the allowed
/// range built from `minimumJointSpacing` and (when the part names a joint type) that type's
/// extra tolerance. Biscuit is exempt (the sheet gives it no `Tolerance` field).
pub fn joint_fit_warnings(part_guid: &str, part: &Part, settings: &WoodSettings) -> Vec<Warning> {
    let joint_code = part.joint_type.first().map(|j| j.joint_code.as_str());
    let mut out = Vec::new();
    for (i, m) in part.mortise.iter().enumerate() {
        out.extend(joint_fit_warning(part_guid, "Mortise", i, m, settings, joint_code));
    }
    for (i, t) in part.tenon.iter().enumerate() {
        out.extend(joint_fit_warning(part_guid, "Tenon", i, t, settings, joint_code));
    }
    out
}

/// Build-size warnings: the part's three physical dimensions, sorted smallest to largest, each
/// checked against the matching build-size limit. Skipped entirely until all three limits are
/// set (`Wood_Workflow_Scope.md`: "All three must fit. Warning only, never a hard stop").
pub fn build_size_warnings(part_guid: &str, dimensions: (f64, f64, f64), settings: &WoodSettings) -> Vec<Warning> {
    if !settings.build_size_limits.is_active() {
        return Vec::new();
    }
    let mut sorted = [dimensions.0, dimensions.1, dimensions.2];
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let limits = [
        ("smallest", settings.build_size_limits.smallest),
        ("middle", settings.build_size_limits.middle),
        ("largest", settings.build_size_limits.largest),
    ];
    let mut out = Vec::new();
    for (i, (label, limit)) in limits.iter().enumerate() {
        let Some(limit) = limit else { continue };
        if sorted[i] > *limit {
            out.push(Warning {
                id: format!("buildsize:{part_guid}:{label}"),
                kind: WarningKind::BuildSize,
                part_guid: part_guid.to_string(),
                message: format!("the part's {label} dimension ({:.3}) is larger than the {label} build-size limit ({limit:.3})", sorted[i]),
            });
        }
    }
    out
}

/// Every current warning for one part: fastener and joint fit always run; build size only when
/// `dimensions` is given (it needs the part's outline, which only the engine, not this crate,
/// can read from the drawing).
pub fn part_warnings(part_guid: &str, part: &Part, settings: &WoodSettings, dimensions: Option<(f64, f64, f64)>) -> Vec<Warning> {
    let mut out = fastener_warnings(part_guid, part);
    out.extend(joint_fit_warnings(part_guid, part, settings));
    if let Some(dims) = dimensions {
        out.extend(build_size_warnings(part_guid, dims, settings));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Face, HoleEntry};

    fn part_with_thickness(t: f64) -> Part {
        Part::new("Test", t)
    }

    #[test]
    fn a_hole_deeper_than_the_part_is_a_fastener_warning() {
        let mut part = part_with_thickness(0.75);
        part.hole_blind.push(HoleEntry { face: Face::A, dia: 0.25, depth: Some(1.0), extra1: None, extra2: None, at: vec![] });
        let warnings = fastener_warnings("p1", &part);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].kind, WarningKind::Fastener);
    }

    #[test]
    fn a_hole_within_thickness_has_no_warning() {
        let mut part = part_with_thickness(0.75);
        part.hole_blind.push(HoleEntry { face: Face::A, dia: 0.25, depth: Some(0.5), extra1: None, extra2: None, at: vec![] });
        assert!(fastener_warnings("p1", &part).is_empty());
    }

    #[test]
    fn a_through_hole_never_warns_regardless_of_depth() {
        // Hole-Through has no depth field at all, so it can never trigger this check.
        let part = part_with_thickness(0.75);
        assert!(fastener_warnings("p1", &part).is_empty());
    }

    #[test]
    fn a_long_nail_is_a_fastener_warning() {
        let mut part = part_with_thickness(0.75);
        part.nail.push(crate::data::NailEntry { face_edge: "A".into(), diameter: 0.1, length: 1.5, at: vec![] });
        let warnings = fastener_warnings("p1", &part);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn warning_ids_are_stable_across_repeated_computation() {
        let mut part = part_with_thickness(0.75);
        part.hole_blind.push(HoleEntry { face: Face::A, dia: 0.25, depth: Some(1.0), extra1: None, extra2: None, at: vec![] });
        let a = fastener_warnings("p1", &part);
        let b = fastener_warnings("p1", &part);
        assert_eq!(a[0].id, b[0].id);
    }

    #[test]
    fn a_tolerance_tighter_than_minimum_spacing_warns() {
        let mut part = part_with_thickness(0.75);
        part.mortise.push(JoineryEntry {
            face_edge: "Left".into(),
            length: 1.5,
            width: 0.375,
            depth: Some(0.75),
            tolerance: Some(0.001),
            at: vec![],
        });
        let settings = WoodSettings { minimum_joint_spacing: 0.01, ..WoodSettings::default() };
        let warnings = joint_fit_warnings("p1", &part, &settings);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].kind, WarningKind::JointFit);
    }

    #[test]
    fn a_tolerance_at_or_above_minimum_with_no_type_tolerance_does_not_warn() {
        let mut part = part_with_thickness(0.75);
        part.mortise.push(JoineryEntry { face_edge: "Left".into(), length: 1.5, width: 0.375, depth: Some(0.75), tolerance: Some(0.02), at: vec![] });
        let settings = WoodSettings { minimum_joint_spacing: 0.01, ..WoodSettings::default() };
        assert!(joint_fit_warnings("p1", &part, &settings).is_empty());
    }

    #[test]
    fn a_tolerance_looser_than_the_joint_types_allowance_warns() {
        let mut part = part_with_thickness(0.75);
        part.mortise.push(JoineryEntry { face_edge: "Left".into(), length: 1.5, width: 0.375, depth: Some(0.75), tolerance: Some(0.05), at: vec![] });
        part.joint_type.push(crate::data::JointTypeEntry { joint_code: "Tenon".into(), mating_part_id: "leg".into(), tolerance: None });
        let mut settings = WoodSettings { minimum_joint_spacing: 0.01, ..WoodSettings::default() };
        settings.joint_tolerance_by_type.insert("Tenon".into(), 0.02);
        let warnings = joint_fit_warnings("p1", &part, &settings);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn biscuit_is_exempt_from_joint_fit() {
        // JoineryEntry's tolerance field exists for Biscuit structurally, but the command layer
        // never lets it be set (group 4 refuses depth/tolerance for Biscuit); joint_fit_warnings
        // itself only looks at Mortise and Tenon.
        let part = part_with_thickness(0.75);
        assert!(joint_fit_warnings("p1", &part, &WoodSettings::default()).is_empty());
    }

    #[test]
    fn build_size_is_skipped_until_all_three_limits_are_set() {
        let settings = WoodSettings::default();
        assert!(build_size_warnings("p1", (1.0, 2.0, 3.0), &settings).is_empty());
    }

    #[test]
    fn build_size_warns_on_whichever_sorted_dimension_exceeds_its_limit() {
        let mut settings = WoodSettings::default();
        settings.build_size_limits.smallest = Some(5.0);
        settings.build_size_limits.middle = Some(10.0);
        settings.build_size_limits.largest = Some(20.0);
        let warnings = build_size_warnings("p1", (1.0, 2.0, 25.0), &settings);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].kind, WarningKind::BuildSize);
    }

    #[test]
    fn build_size_fits_within_all_three_limits_has_no_warning() {
        let mut settings = WoodSettings::default();
        settings.build_size_limits.smallest = Some(5.0);
        settings.build_size_limits.middle = Some(10.0);
        settings.build_size_limits.largest = Some(20.0);
        assert!(build_size_warnings("p1", (1.0, 2.0, 3.0), &settings).is_empty());
    }
}
