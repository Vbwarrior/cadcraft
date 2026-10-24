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
- WoodCraft plan documents under `plans/`, and a `changes/` folder recording what each WoodCraft
  phase adds.

## [0.10.0] - 2026-10-23

Baseline entry matching the version already in `Cargo.toml` when changelog tracking started.
