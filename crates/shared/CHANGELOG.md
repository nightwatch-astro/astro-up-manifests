# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/nightwatch-astro/astro-up-manifests/releases/tag/astro-up-shared-v0.1.0) - 2026-03-29

### Bug Fixes

- address drift findings from sync.analyze and sync.conflicts
- resolve clippy warnings and cleanup dead code (Step 12)
- *(checker)* integrate rate limiter and add URL validation

### Features

- *(shared)* implement shared crate with manifest types, validation, templates, and version parsing
- *(setup)* add shared crate and update workspace for manifest pipeline

### Style

- apply cargo fmt formatting
