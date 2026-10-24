//! WoodCraft Phase 5: reading a part's outline into a named, ordered list of edges
//! (`plans/WoodCraft_Implementation_Plan.md` Phase 5, `plans/Wood_Workflow_Scope.md`
//! "Edge names"). Pure geometry: no drawing mutation, no commands.
//!
//! An edge is named by the direction its outside faces (`Top`, `Bottom`, `Left`, `Right`).
//! Edges of the same kind get letters in clockwise order from the top-left (`Top A`, `Top B`).
//! An edge that is not exactly horizontal or vertical (within the straightness tolerance) is
//! named for the side it faces plus "Angle" (`Top Angle`); when a side has one flat and one
//! angled edge they are `Top` and `Top Angle A`; two angled edges on a side with no flat one
//! are `Top Angle A` and `Top Angle B`. A fillet or chamfer is already part of the outline
//! (CADCraft's own fillet/chamfer commands write it there): a fillet's rounded corner is a real
//! arc and is read as one, never flattened into short lines; a chamfer is already a straight
//! line and is read like any other edge.

use cadcraft_doc::geom::{Arc, PolyVertex, Polyline, Segment, Vec2};

/// Which of the four sides an edge's outside faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    fn label(self) -> &'static str {
        match self {
            Side::Top => "Top",
            Side::Bottom => "Bottom",
            Side::Left => "Left",
            Side::Right => "Right",
        }
    }
}

/// One edge of a part's outline, in order around the shape.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    /// `"Top"`, `"Top A"`, `"Top Angle"`, `"Top Angle A"`, and so on.
    pub name: String,
    pub side: Side,
    /// A line exactly horizontal or vertical (within the straightness tolerance). An arc is
    /// never flat, even one that happens to touch an axis-aligned tangent.
    pub flat: bool,
    pub start: Vec2,
    pub end: Vec2,
    /// `Some` only for an edge that is an arc (a fillet's rounded corner, or any other arc
    /// segment already in the outline).
    pub arc: Option<Arc>,
}

/// The outline could not be read into edges.
#[derive(Debug, thiserror::Error)]
pub enum EdgeError {
    #[error("the outline has fewer than three sides")]
    TooFew,
}

/// A whole circle has no sides to name; it is reported as a single, unnamed round outline.
pub fn circle_edges() -> Vec<Edge> {
    Vec::new()
}

/// The named, ordered edges of a closed polyline outline. `straightness_tolerance_deg` is the
/// setting (degrees) within which a line still counts as flat; `0.0` means only an edge that is
/// exactly horizontal or vertical counts (`plans/WoodCraft_Decisions.md`: "Rounding noise below
/// 1e-9 is ignored").
pub fn polyline_edges(vertices: &[PolyVertex], closed: bool, straightness_tolerance_deg: f64) -> Result<Vec<Edge>, EdgeError> {
    let pl = Polyline { vertices: vertices.to_vec(), closed };
    let segments = pl.segments();
    if segments.len() < 3 {
        return Err(EdgeError::TooFew);
    }

    // Outward-facing classification assumes a loop walked counter-clockwise (left of travel
    // faces outward); `area()` is CCW-positive, so a clockwise loop is walked in reverse first.
    let ccw = pl.area() >= 0.0;
    let ordered: Vec<Segment> = if ccw { segments } else { segments.into_iter().rev().map(|s| s.reversed()).collect() };

    let tol = straightness_tolerance_deg.to_radians().max(0.0);
    let mut edges: Vec<Edge> = ordered
        .iter()
        .map(|s| {
            let (side, flat) = classify(s, tol);
            Edge { name: String::new(), side, flat, start: s.start(), end: s.end(), arc: as_arc(s) }
        })
        .collect();

    name_edges(&mut edges);
    Ok(edges)
}

fn as_arc(s: &Segment) -> Option<Arc> {
    match s {
        Segment::Arc { arc, .. } => Some(*arc),
        Segment::Line(_) => None,
    }
}

/// Which side this segment's outside faces, and whether it is flat (a line within
/// `tol` radians of horizontal or vertical; an arc is never flat).
fn classify(s: &Segment, tol: f64) -> (Side, bool) {
    let outward = outward_direction(s);
    let side = nearest_side(outward);
    let flat = match s {
        Segment::Line(l) => {
            let dir = l.dir();
            if dir == Vec2::ZERO {
                false
            } else {
                let horizontal = angle_between(dir, Vec2::X).min(angle_between(dir, -Vec2::X));
                let vertical = angle_between(dir, Vec2::Y).min(angle_between(dir, -Vec2::Y));
                horizontal <= tol || vertical <= tol
            }
        }
        Segment::Arc { .. } => false,
    };
    (side, flat)
}

/// The unsigned angle (radians, 0..=pi/2 for axis comparisons) between two vectors.
fn angle_between(a: Vec2, b: Vec2) -> f64 {
    let (a, b) = (a.normalized(), b.normalized());
    a.dot(b).clamp(-1.0, 1.0).acos()
}

/// The segment's own outward-facing normal. The loop was normalized to CCW above; walking a
/// CCW polygon, the interior is to the left of the direction of travel (`dir.perp()`, a 90°
/// counter-clockwise turn), so the outside is the opposite way.
fn outward_direction(s: &Segment) -> Vec2 {
    match s {
        Segment::Line(l) => -l.dir().perp(),
        // Outward from an arc's own center is outward for a convex (bulging-out) corner; a
        // concave (bulging-in) corner's outward direction is the reverse.
        Segment::Arc { arc, ccw } => {
            let out = (s.mid() - arc.center).normalized();
            if *ccw { out } else { -out }
        }
    }
}

/// The closest of the four axis directions to `dir`.
fn nearest_side(dir: Vec2) -> Side {
    if dir.x.abs() >= dir.y.abs() {
        if dir.x >= 0.0 { Side::Right } else { Side::Left }
    } else if dir.y >= 0.0 {
        Side::Top
    } else {
        Side::Bottom
    }
}

/// Fills in each edge's `name`, in place, by the scope doc's rule: edges on the same side get
/// letters in clockwise order from the top-left; a flat edge on a side is plain (`Top`) unless
/// an angled edge shares that side, in which case the flat one still gets no letter and the
/// angled one(s) get letters (`Top`, `Top Angle A`); with no flat edge on a side, every angled
/// edge there gets a letter (`Top Angle A`, `Top Angle B`).
fn name_edges(edges: &mut [Edge]) {
    for side in [Side::Top, Side::Right, Side::Bottom, Side::Left] {
        let idx: Vec<usize> = edges.iter().enumerate().filter(|(_, e)| e.side == side).map(|(i, _)| i).collect();
        let flats: Vec<usize> = idx.iter().copied().filter(|&i| edges[i].flat).collect();
        let angled: Vec<usize> = idx.iter().copied().filter(|&i| !edges[i].flat).collect();

        // A single flat edge on this side needs no letter; more than one still gets lettered,
        // since "edges of the same kind get letters" applies whenever there is more than one.
        if flats.len() == 1 {
            edges[flats[0]].name = side.label().to_string();
        } else {
            for (n, &i) in flats.iter().enumerate() {
                edges[i].name = format!("{} {}", side.label(), letter(n));
            }
        }
        if angled.len() == 1 && flats.is_empty() {
            edges[angled[0]].name = format!("{} Angle", side.label());
        } else {
            for (n, &i) in angled.iter().enumerate() {
                edges[i].name = format!("{} Angle {}", side.label(), letter(n));
            }
        }
    }
}

fn letter(n: usize) -> char {
    (b'A' + (n.min(25) as u8)) as char
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: f64, h: f64) -> Vec<PolyVertex> {
        vec![
            PolyVertex::new(Vec2::new(0.0, 0.0)),
            PolyVertex::new(Vec2::new(w, 0.0)),
            PolyVertex::new(Vec2::new(w, h)),
            PolyVertex::new(Vec2::new(0.0, h)),
        ]
    }

    #[test]
    fn a_rectangle_has_four_plain_named_edges() {
        let edges = polyline_edges(&rect(10.0, 4.0), true, 0.0).unwrap();
        assert_eq!(edges.len(), 4);
        let names: Vec<&str> = edges.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Bottom", "Right", "Top", "Left"]);
        assert!(edges.iter().all(|e| e.flat && e.arc.is_none()));
    }

    #[test]
    fn a_rectangle_drawn_clockwise_names_the_same() {
        let mut v = rect(10.0, 4.0);
        v.reverse();
        let edges = polyline_edges(&v, true, 0.0).unwrap();
        let mut names: Vec<&str> = edges.iter().map(|e| e.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, ["Bottom", "Left", "Right", "Top"]);
    }

    #[test]
    fn an_l_shape_names_the_repeated_sides_with_letters() {
        // An L: notch taken out of the top-right corner.
        let v = vec![
            PolyVertex::new(Vec2::new(0.0, 0.0)),
            PolyVertex::new(Vec2::new(10.0, 0.0)),
            PolyVertex::new(Vec2::new(10.0, 4.0)),
            PolyVertex::new(Vec2::new(6.0, 4.0)),
            PolyVertex::new(Vec2::new(6.0, 8.0)),
            PolyVertex::new(Vec2::new(0.0, 8.0)),
        ];
        let edges = polyline_edges(&v, true, 0.0).unwrap();
        assert_eq!(edges.len(), 6);
        let tops: Vec<&str> = edges.iter().filter(|e| e.side == Side::Top).map(|e| e.name.as_str()).collect();
        assert_eq!(tops.len(), 2);
        assert!(tops.contains(&"Top A") && tops.contains(&"Top B"));
    }

    #[test]
    fn one_angled_edge_on_a_side_is_named_angle_without_a_letter() {
        // A right trapezoid: the right edge leans in, nothing else does.
        let v = vec![
            PolyVertex::new(Vec2::new(0.0, 0.0)),
            PolyVertex::new(Vec2::new(10.0, 0.0)),
            PolyVertex::new(Vec2::new(8.0, 4.0)),
            PolyVertex::new(Vec2::new(0.0, 4.0)),
        ];
        let edges = polyline_edges(&v, true, 0.0).unwrap();
        let right = edges.iter().find(|e| e.side == Side::Right).unwrap();
        assert!(!right.flat);
        assert_eq!(right.name, "Right Angle");
    }

    #[test]
    fn two_angled_edges_on_a_side_get_letters() {
        // A triangle-ish shape: two angled edges both read as facing Top, no flat edge there.
        let v = vec![
            PolyVertex::new(Vec2::new(0.0, 0.0)),
            PolyVertex::new(Vec2::new(10.0, 0.0)),
            PolyVertex::new(Vec2::new(8.0, 6.0)),
            PolyVertex::new(Vec2::new(5.0, 8.0)),
            PolyVertex::new(Vec2::new(2.0, 6.0)),
        ];
        let edges = polyline_edges(&v, true, 0.0).unwrap();
        let tops: Vec<&Edge> = edges.iter().filter(|e| e.side == Side::Top).collect();
        assert_eq!(tops.len(), 2);
        assert!(tops.iter().all(|e| !e.flat));
        let mut names: Vec<&str> = tops.iter().map(|e| e.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, ["Top Angle A", "Top Angle B"]);
    }

    #[test]
    fn a_flat_and_an_angled_edge_on_the_same_side() {
        // A rectangle with its top-right corner cut off diagonally: most of the top stays flat,
        // and one short angled edge takes over the corner, still facing Top.
        let v = vec![
            PolyVertex::new(Vec2::new(0.0, 0.0)),
            PolyVertex::new(Vec2::new(10.0, 0.0)),
            PolyVertex::new(Vec2::new(10.0, 3.0)),
            PolyVertex::new(Vec2::new(8.0, 4.0)),
            PolyVertex::new(Vec2::new(0.0, 4.0)),
        ];
        let edges = polyline_edges(&v, true, 0.0).unwrap();
        let tops: Vec<&Edge> = edges.iter().filter(|e| e.side == Side::Top).collect();
        assert_eq!(tops.len(), 2, "expected one flat Top and one angled Top edge");
        assert_eq!(tops.iter().filter(|e| e.flat).count(), 1);
        let flat = tops.iter().find(|e| e.flat).unwrap();
        let angled = tops.iter().find(|e| !e.flat).unwrap();
        assert_eq!(flat.name, "Top");
        assert_eq!(angled.name, "Top Angle A");
    }

    #[test]
    fn a_fillet_arc_is_read_as_an_arc_not_flattened() {
        // A rectangle with one bulged (rounded) corner: an arc vertex between two straight runs.
        let mut v = rect(10.0, 4.0);
        v[1].bulge = cadcraft_doc::geom::arc_to_bulge(std::f64::consts::FRAC_PI_2);
        let edges = polyline_edges(&v, true, 0.0).unwrap();
        assert_eq!(edges.len(), 4);
        assert!(edges.iter().any(|e| e.arc.is_some()), "the bulged vertex must read back as an arc edge");
        assert!(edges.iter().filter(|e| e.arc.is_some()).all(|e| !e.flat), "an arc is never flat");
    }

    #[test]
    fn a_concave_fillet_at_an_inside_corner_is_still_read_as_an_arc() {
        // An L-shape (as in an_l_shape_names_the_repeated_sides_with_letters) with its one
        // inside (concave) corner rounded instead of square.
        let mut v = vec![
            PolyVertex::new(Vec2::new(0.0, 0.0)),
            PolyVertex::new(Vec2::new(10.0, 0.0)),
            PolyVertex::new(Vec2::new(10.0, 4.0)),
            PolyVertex::new(Vec2::new(6.0, 4.0)),
            PolyVertex::new(Vec2::new(6.0, 8.0)),
            PolyVertex::new(Vec2::new(0.0, 8.0)),
        ];
        // The concave corner at (6, 4): bulge the turn from "Top A" into "Right B" inward.
        v[3].bulge = cadcraft_doc::geom::arc_to_bulge(-std::f64::consts::FRAC_PI_2);
        let edges = polyline_edges(&v, true, 0.0).unwrap();
        assert_eq!(edges.len(), 6);
        let arcs: Vec<&Edge> = edges.iter().filter(|e| e.arc.is_some()).collect();
        assert_eq!(arcs.len(), 1);
        assert!(!arcs[0].flat, "an arc is never flat");
    }

    #[test]
    fn straightness_tolerance_lets_a_near_flat_edge_count_as_flat() {
        // A right edge leaning by a shade under one degree.
        let v = vec![
            PolyVertex::new(Vec2::new(0.0, 0.0)),
            PolyVertex::new(Vec2::new(10.0, 0.0)),
            PolyVertex::new(Vec2::new(10.05, 4.0)),
            PolyVertex::new(Vec2::new(0.0, 4.0)),
        ];
        let strict = polyline_edges(&v, true, 0.0).unwrap();
        assert!(!strict.iter().find(|e| e.side == Side::Right).unwrap().flat);
        let lenient = polyline_edges(&v, true, 2.0).unwrap();
        assert!(lenient.iter().find(|e| e.side == Side::Right).unwrap().flat);
    }

    #[test]
    fn too_few_segments_is_refused() {
        let v = vec![PolyVertex::new(Vec2::new(0.0, 0.0)), PolyVertex::new(Vec2::new(10.0, 0.0))];
        assert!(matches!(polyline_edges(&v, true, 0.0), Err(EdgeError::TooFew)));
    }
}
