# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for the BizClaw project.

## What is an ADR?

An ADR is a document capturing an important architectural decision made along with its context and consequences.

## Index

| Number | Title | Status | Date |
|--------|-------|--------|------|
| [0001](0001-rust-as-primary-language.md) | Rust as Primary Language | Accepted | 2024-01-15 |
| [0002](0002-monorepo-workspace-structure.md) | Monorepo Workspace Structure | Accepted | 2024-01-20 |
| [0003](0003-tokio-async-runtime.md) | Tokio Async Runtime | Accepted | 2024-02-01 |
| [0004](0004-cdp-browser-automation.md) | CDP-Based Browser Automation | Accepted | 2024-11-15 |

## Creating New ADRs

1. Copy the template:

```bash
cp docs/adr/TEMPLATE.md docs/adr/XXXX-new-decision.md
```

2. Fill in the sections:
   - **Context**: What problem are we solving?
   - **Decision**: What are we doing?
   - **Rationale**: Why this decision?
   - **Consequences**: What are the trade-offs?
   - **Alternatives**: What else was considered?

3. Submit as PR with title: `docs: ADR-XXXX <title>`

## Template

See [TEMPLATE.md](TEMPLATE.md) for the standard ADR format.

## Maintenance

- Review ADRs when making significant architectural changes
- Update status if decision is superseded
- Add consequences as they become known
