# BizClaw GitHub Actions Setup

This document describes the CI/CD pipeline configured for BizClaw project.

## Workflows Overview

### 1. CI Pipeline (`ci.yml`)

**Triggers:**
- Every push to `main` and `develop`
- Every pull request to `main` and `develop`

**Jobs:**

1. **Format Check** - Ensures code follows Rust formatting standards
2. **Clippy Lint** - Runs Clippy linter with strict warnings
3. **Unit Tests** - Runs all tests on Ubuntu, macOS, and Windows
4. **Build** - Compiles release binaries for multiple platforms:
   - x86_64-unknown-linux-gnu
   - aarch64-unknown-linux-gnu
   - x86_64-apple-darwin
   - aarch64-apple-darwin
   - x86_64-pc-windows-gnu
5. **Documentation** - Generates and validates documentation
6. **Security Audit** - Runs `cargo audit` for vulnerability scanning
7. **Dependency Check** - Enforces dependency policies with `cargo deny`
8. **Integration Tests** - Runs module-specific integration tests

**Environment Variables:**
- `CARGO_TERM_COLOR: always`
- `RUST_BACKTRACE: 1`
- `DATABASE_URL: postgres://bizclaw:test@localhost:5432/bizclaw_test` (test job only)

**Artifacts:**
- Build artifacts uploaded for each platform
- Coverage reports saved

### 2. Docker Pipeline (`docker.yml`)

**Triggers:**
- Push to `main` or `develop`
- New version tags (`v*`)
- Pull requests to `main` or `develop`

**Jobs:**

1. **Build Main Image** - Multi-platform Docker build for main binary
2. **Build Platform Image** - Builds platform-specific Docker image
3. **Build Standalone Image** - Builds standalone Docker image
4. **Push Images** - Pushes to both Docker Hub and GitHub Container Registry
5. **Security Scan** - Trivy vulnerability scanning

**Features:**
- Multi-architecture support (amd64, arm64)
- Docker layer caching for faster builds
- Automatic versioning based on git tags
- Security scanning with Trivy
- SARIF format for GitHub Security tab integration

**Push Conditions:**
- Only pushes on `main` branch or version tags
- Does NOT push on pull requests (prevents credential exposure)

### 3. Release Pipeline (`release.yml`)

**Triggers:**
- Version tags matching pattern `v*.*.*`

**Jobs:**

1. **Create Release** - Creates GitHub release with changelog
2. **Build Binaries** - Cross-platform release builds
3. **Build Docker** - Docker images for release
4. **Publish Crates** - Publishes to crates.io (requires CARGO_TOKEN)
5. **Generate SBOM** - Creates Software Bill of Materials
6. **Notify Success** - Completion notification

**Artifacts:**
- Platform-specific binaries
- Docker images
- SBOM in SPDX format

### 4. Security Pipeline (`security.yml`)

**Triggers:**
- Weekly schedule (Sunday midnight)
- Every push to `main`
- Changes to Cargo files

**Jobs:**

1. **Cargo Audit** - Scans for known vulnerabilities
2. **Cargo Deny** - Enforces dependency policies
3. **Dependency Review** - GitHub's dependency review
4. **CodeQL Analysis** - Static security analysis
5. **Container Scan** - Trivy scanning for Docker images
6. **Secret Scan** - GitLeaks for secrets in code
7. **Security Hardening** - Custom checks for security best practices
8. **Outdated Dependencies** - Checks for outdated dependencies

**Features:**
- Scheduled weekly for proactive security monitoring
- Immediate scan on dependency changes
- SARIF format for GitHub Security tab

### 5. Stale PR Management (`stale.yml`)

**Triggers:**
- Weekly schedule (Sunday midnight)

**Features:**
- Marks issues/PRs as stale after 30 days
- Closes after 30 more days of inactivity
- Configurable messages in Vietnamese

## PR Automation

### Labeler Configuration (`.github/labeler.yml`)

Automatically labels PRs based on file changes:
- Module-specific labels (bizclaw-core, bizclaw-ecommerce, etc.)
- Type labels (bug, feature, enhancement, etc.)
- Area labels (docker, ci-cd, documentation, tests, etc.)

### Size Labels

Automatically adds size labels based on PR diff:
- `size/xs`: < 50 lines
- `size/s`: 50-200 lines
- `size/m`: 200-500 lines
- `size/l`: 500-1000 lines
- `size/xl`: > 1000 lines

### PR Template

Standardized PR template with:
- Vietnamese descriptions
- Checklist for compliance
- Module affected tracking
- Testing requirements

## Required Secrets

### GitHub Secrets

Configure these in GitHub → Settings → Secrets:

- `DOCKERHUB_USERNAME` - Docker Hub username
- `DOCKERHUB_TOKEN` - Docker Hub access token
- `CARGO_TOKEN` - crates.io API token (for publishing)
- `GITHUB_TOKEN` - Automatically provided by GitHub Actions

### Docker Hub Token

Generate at: https://hub.docker.com/settings/security

Required permissions:
- Create/manage repositories
- Push images

## Configuration Files

### `.cargo/audit.toml`
- Configures `cargo audit`
- Ignores specific advisories

### `.cargo/deny.toml`
- Configures `cargo deny`
- Enforces license policies
- Bans specific dependencies

### `.github/CODEOWNERS`
- Automatic reviewer assignment
- Module ownership

### `.github/dependabot.yml`
- Automated dependency updates
- Weekly schedule for all ecosystems

## Best Practices

1. **Never commit secrets** - Use GitHub Secrets
2. **Use minimal permissions** - Only request needed scopes
3. **Enable branch protection** - Require status checks before merge
4. **Keep workflows fast** - Use caching and parallel jobs
5. **Monitor costs** - GitHub Actions minutes are limited
6. **Regular reviews** - Review workflow changes in PRs

## Troubleshooting

### Workflow Not Running

1. Check branch protection rules
2. Verify secrets are configured
3. Check workflow syntax with act (local testing)

### Build Failures

1. Check cargo-audit results
2. Verify dependency compatibility
3. Run locally: `cargo build --all-features`

### Docker Build Issues

1. Verify Docker Hub credentials
2. Check build context size
3. Ensure multi-stage build is optimized

### Test Failures

1. Run locally: `cargo test --workspace`
2. Check environment variables
3. Verify database is available

## Cost Optimization

1. **Use caching** - All workflows use cargo cache
2. **Parallel jobs** - Independent jobs run in parallel
3. **Smart triggers** - Workflows only run when needed
4. **Artifact retention** - Auto-cleanup after 7 days

## Monitoring

- **Actions tab** - View workflow runs
- **Security tab** - View security findings
- **Insights** - Repository analytics

## Support

For workflow issues:
- GitHub Actions Documentation
- Check workflow logs
- Review GitHub Community

For security issues:
- Create private security advisory
- Email: security@bizclaw.com
