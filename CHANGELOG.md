# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- WoodCraft baseline: a new `cadcraft-wood` crate holding the wood part data model, GUID
  generation, and the XRECORD storage used to keep a part's properties attached to its drawing
  geometry.
- A `woodpart` command that creates a wood part's geometry and its property data together, as one
  undoable step.
- WoodCraft project files: new, open and save commands for `.wood` project files, kept separate
  from the standard 2D file commands. Opening anything other than a `.wood` file as a WoodCraft
  project is refused with a clear message.
- Commands to list a drawing's wood parts, read one part's full properties, and look up a part's
  display name from its underlying block name. A part's display name is always shown in place of
  its internal identifier.
- Commands to change a wood part's Thickness, Grain direction and display name, each undoable on
  its own.
- A size limit on a drawing's stored wood part data, refused with a clear error rather than
  allowed to grow without bound.
- Commands to copy a wood part (as an independent new part), to place another shared instance of
  one, to delete instances (cleaning up a part's data once its last instance is gone), and to
  split shared instances back into independent parts, one at a time or all at once. Changing a
  part shared by more than one instance is refused until it is split.
- WoodCraft plan documents under `plans/`, and a `changes/` folder recording what each WoodCraft
  phase adds.
- A WoodCraft settings file, separate from any drawing, holding straightness tolerance, units,
  minimum joint spacing, per-joint-type tolerance, the three build-size limits, whether export is
  blocked while fastener warnings exist, and the git timer interval. Commands to read and change
  it.
- Common size presets (standard lumber sizes seeded in) and six managed lists (joint type, edge
  profile, glue, screw, hardware, end treatment), each with commands to add, rename and delete an
  entry.
- A command that reads a wood part's outline into its named edges (Top, Bottom, Left, Right,
  lettered when a side has more than one, and "Angle" for one that isn't flat), honoring the
  straightness tolerance setting. Fillet and chamfer corners already in the outline are read
  correctly, arcs staying arcs.
- A WoodCraft section in the README listing every command built so far, with its parameters and
  what it does.
- Commands to set and clear a wood part's orientation mark, and to add, delete and list its
  reference marks, each drawn as a small marker in the part's own geometry.
- Commands to add, delete and list a wood part's holes (Through, Blind, Counterbore,
  Countersink, Pocket-Screw, Shelf-Pin, Dowel), each drawn as a small marker. Holes with the
  same properties at different positions share one entry.
- Commands to add, delete and list a wood part's cuts (Dado, Groove, Rabbet, Slot, Notch,
  Pocket), each drawn as a small marker.
- Commands for a wood part's joinery (Mortise, Tenon, Biscuit, each drawn as a small marker),
  its Joint type, and its End treatments (checked against the WoodCraft settings list).
- Commands for a wood part's Edge profile (checked against the WoodCraft settings list), Bevel
  and Miter.
- Commands for a wood part's Glue, Screw, Nail and Dowel fasteners, and its Hinge, Drawer slide
  and Drawer Pull hardware (type checked against the WoodCraft settings lists), each drawn as a
  small marker. This completes all six WoodCraft property groups.
- A warnings engine: a running, computed list of fastener, joint fit and build-size warnings
  across a drawing's wood parts, with commands to read it and to acknowledge or un-acknowledge
  one warning for the current session.

## [0.10.0] - 2026-10-23

Baseline entry matching the version already in `Cargo.toml` when changelog tracking started.
