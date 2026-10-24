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

## [0.10.0] - 2026-10-23

Baseline entry matching the version already in `Cargo.toml` when changelog tracking started.
