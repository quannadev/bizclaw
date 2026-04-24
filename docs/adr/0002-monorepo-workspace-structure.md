# ADR-0002: Monorepo Workspace Structure

**Status**: Accepted  
**Date**: 2024-01-20  
**Deciders**: BizClaw Team

## Context

BizClaw consists of multiple interconnected components:
- Core framework
- Agent engine
- Tools and integrations
- Channel adapters
- Web dashboard

We needed to decide on the repository structure to balance:
- Code sharing and dependency management
- Independent deployment capabilities
- Build performance
- Developer experience

## Decision

**Use Cargo workspace monorepo with 34 crates organized by domain.**

```
bizclaw/
├── Cargo.toml (workspace)
├── crates/
│   ├── bizclaw-core/      # Core traits and types
│   ├── bizclaw-agent/     # Agent pipeline
│   ├── bizclaw-tools/     # Tool implementations
│   ├── bizclaw-channels/  # Communication channels
│   ├── bizclaw-browser/   # Browser automation
│   └── ... (34 total crates)
├── src/                   # Binary entry points
└── docs/                  # Documentation
```

## Rationale

### Benefits

1. **Shared Dependencies**: Single source of truth for versions
2. **Cross-crate Refactoring**: IDE support for navigating related code
3. **Unified Version**: All components use same Rust/toolchain version
4. **CI/CD Simplification**: Single pipeline for all code
5. **Atomic Changes**: Update related components in one PR

### Trade-offs

- Repository size (135K+ lines)
- Slower `cargo check` for unrelated crates
- Need discipline to maintain crate boundaries

## Crate Organization

| Category | Crates | Purpose |
|----------|--------|---------|
| Core | bizclaw-core | Traits, types, config |
| AI/ML | bizclaw-brain, bizclaw-providers, bizclaw-memory | LLM, routing |
| Agent | bizclaw-agent, bizclaw-hands, bizclaw-hai | Agent execution |
| Tools | bizclaw-tools, bizclaw-browser, bizclaw-scheduler | Tool ecosystem |
| Channels | bizclaw-channels, bizclaw-social | Integrations |
| Platform | bizclaw-gateway, bizclaw-platform | Web, auth, multi-tenancy |
| Security | bizclaw-security, bizclaw-redteam | Vault, approval, testing |
| Utilities | bizclaw-tracing, bizclaw-evaluator, etc. | Cross-cutting concerns |

## Consequences

### Positive

- Clear ownership boundaries
- Independent versioning when needed
- Easy to find related code
- Shared workspace dependencies

### Negative

- Larger repository
- Build cache sharing across crates
- Must be careful with circular dependencies

## Alternatives Considered

| Structure | Pros | Cons |
|-----------|------|------|
| Separate repos | Independent versioning | Hard to sync changes |
| Single mega-crate | Simple | Loses granularity |
| Category repos | Balance | Complex CI/CD |

## Implementation Notes

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

All crates use workspace dependencies to ensure consistency.
