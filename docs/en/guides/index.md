---
title: User Guide
description: Guides for parsing, serializing, editing, and integrating YAML with pyrs-yaml.
tags:
  - docs
status: new
---

## User Guide

The User Guide is organized into two sections:

### Core

Core YAML operations — parsing, serialization, round-trip preservation, in-place editing, and streaming.

- [Parsing YAML](parsing.md) — Parse strings, files, and multiple documents
- [Serialization](serialization.md) — Convert YAML documents to and from Python objects
- [PyYAML Compatibility](pyyaml-compat.md) — Drop-in replacement API
- [Round-Trip Preservation](round-trip.md) — Comments, anchors, tags, and formatting survive
- [In-Place Editing](editing.md) — Edit documents via JSONPath paths without losing formatting
- [Streaming Parse](streaming.md) — Incremental parsing with constant memory

### Integrations

Advanced features — custom schemas, plugin development, community plugins, configuration management, markdown frontmatter, i18n error messages, and NumPy ndarray support.

- [Custom Schemas](custom-schema.md) — Define custom YAML schemas for type resolution
- [Plugin Development](plugin-development.md) — Build custom tag handlers and node types
- [Community Plugins](community-plugins.md) — Built-in plugins for datetime, UUID, decimal, and more
- [Configuration Management](tutorial-config-management.md) — End-to-end walkthrough
- [Markdown Frontmatter](frontmatter.md) — Extract YAML frontmatter from markdown files
- [i18n Error Messages](i18n.md) — Localize error messages
- [NumPy ndarray](numpy.md) — Serialize numpy arrays to YAML
