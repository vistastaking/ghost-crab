# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.7](https://github.com/stakelens/ghost-crab/compare/ghost-crab-macros-v0.1.6...ghost-crab-macros-v0.1.7) - 2024-07-26

### Other
- Use ".expect" instead of ".unwrap()" inside ghost-crab-macros

## [0.1.6](https://github.com/stakelens/ghost-crab/compare/ghost-crab-macros-v0.1.5...ghost-crab-macros-v0.1.6) - 2024-07-26

### Added
- Load "event_handler" config dynamically at runtime
- load block handler config dynamically at runtime

### Other
- Refactor indexer and add errors

## [0.1.5](https://github.com/stakelens/ghost-crab/compare/ghost-crab-macros-v0.1.4...ghost-crab-macros-v0.1.5) - 2024-07-25

### Added
- Add requests per second to the "config.json"

## [0.1.4](https://github.com/stakelens/ghost-crab/compare/ghost-crab-macros-v0.1.3...ghost-crab-macros-v0.1.4) - 2024-07-23

### Added
- Add config::load

### Fixed
- Fix macro invalid address error

## [0.1.3](https://github.com/stakelens/ghost-crab/compare/ghost-crab-macros-v0.1.2...ghost-crab-macros-v0.1.3) - 2024-07-19

### Other
- Adds step, network, start_block and execution_mode to "block_handler"
- Adds "network" to handler
- Adds execution_mode to handler
- Adds "ghost_crab_common" to share the config struct
- Adds "template" proc-macro and renames "handle" to "event_handler"

## [0.1.2](https://github.com/stakelens/ghost-crab/compare/ghost-crab-macros-v0.1.1...ghost-crab-macros-v0.1.2) - 2024-07-13

### Other
- add clippy/rustfmt

## [0.1.1](https://github.com/vistastaking/ghost-crab/compare/ghost-crab-macros-v0.1.0...ghost-crab-macros-v0.1.1) - 2024-06-27

### Fixed
- macros imports
