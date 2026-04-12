# Contributing to Aether

Thank you for your interest in contributing to Aether! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- Rust 1.70 or higher
- Git
- (Optional) Podman for local container testing
- (Optional) Kubernetes cluster access for K8s testing

### Development Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/ssahani/aether
   cd aether
   ```

2. **Build the project**
   ```bash
   cargo build
   ```

3. **Run tests**
   ```bash
   cargo test
   ```

4. **Run with examples**
   ```bash
   cargo run -- validate --spec workload.yaml
   ```

## Development Workflow

### Making Changes

1. **Create a branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**
   - Write code
   - Add tests
   - Update documentation

3. **Run checks**
   ```bash
   make lint      # Format and clippy
   make test      # All tests
   make ci        # Full CI checks
   ```

4. **Commit your changes**
   ```bash
   git add .
   git commit -m "feat: your feature description"
   ```

   Follow [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat:` - New feature
   - `fix:` - Bug fix
   - `docs:` - Documentation changes
   - `test:` - Test additions/changes
   - `refactor:` - Code refactoring
   - `chore:` - Maintenance tasks

5. **Push and create PR**
   ```bash
   git push origin feature/your-feature-name
   ```

### Code Style

- **Format:** Run `cargo fmt` before committing
- **Linting:** Run `cargo clippy` and fix warnings
- **Documentation:** Add doc comments for public APIs
- **Tests:** Write tests for new functionality

### Testing Guidelines

**Unit Tests**
- Place in the same file as the code
- Use `#[cfg(test)]` module
- Test individual functions/methods

**Integration Tests**
- Place in `tests/` directory
- Test end-to-end workflows
- Use `tempfile` for temporary state

**Example:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        let input = "test";

        // Act
        let result = my_function(input);

        // Assert
        assert_eq!(result, "expected");
    }
}
```

### Documentation

- **Public APIs:** Always document with `///` comments
- **Modules:** Add module-level documentation
- **Examples:** Include code examples in doc comments
- **Guides:** Update relevant `.md` files

**Example:**
```rust
/// Validates a workload specification
///
/// # Arguments
///
/// * `spec` - The workload specification to validate
///
/// # Returns
///
/// Returns `Ok(())` if valid, or an error describing the validation failure.
///
/// # Examples
///
/// ```
/// use aether::spec::Workload;
///
/// let workload = Workload::from_file("workload.yaml")?;
/// validate(&workload)?;
/// ```
pub fn validate(spec: &Workload) -> Result<()> {
    // Implementation
}
```

## Areas for Contribution

### Easy Contributions

- **Documentation improvements**
  - Fix typos
  - Add examples
  - Clarify explanations

- **Tests**
  - Add integration tests
  - Improve test coverage
  - Add edge case tests

- **Examples**
  - Create more workload examples
  - Add tutorial content

### Medium Contributions

- **TUI Enhancements**
  - Additional screens
  - Better navigation
  - More statistics

- **CLI Improvements**
  - Additional flags/options
  - Better error messages
  - Shell completions

- **Runtime Features**
  - ConfigMap/Secret support
  - More health probe types
  - Resource autoscaling hints

### Advanced Contributions

- **New Runtimes**
  - Docker runtime adapter
  - Nomad runtime adapter
  - Other orchestrators

- **Migration Strategies**
  - Canary deployments
  - Traffic splitting
  - Custom strategies

- **Performance**
  - Optimization
  - Caching
  - Parallel operations

- **Observability**
  - Metrics collection
  - Distributed tracing
  - Log aggregation

## Pull Request Process

1. **Ensure all checks pass**
   ```bash
   make ci
   ```

2. **Update documentation**
   - Update README if needed
   - Add/update doc comments
   - Update relevant guides

3. **Write a clear PR description**
   - What does this change?
   - Why is it needed?
   - How was it tested?

4. **Link related issues**
   - Reference issue numbers
   - Use "Fixes #123" to auto-close

5. **Wait for review**
   - Address feedback
   - Make requested changes
   - Re-run tests

6. **Squash commits if requested**
   ```bash
   git rebase -i main
   ```

## Code Review Guidelines

### For Reviewers

- Be constructive and respectful
- Ask questions, don't make demands
- Approve when satisfied
- Suggest improvements, don't block on style

### For Authors

- Be responsive to feedback
- Don't take criticism personally
- Ask for clarification if needed
- Make changes promptly

## Architecture Guidelines

### Project Structure

```
aether/
├── src/
│   ├── main.rs           # CLI entrypoint
│   ├── cli.rs            # CLI framework (commands, args)
│   ├── commands.rs       # Command handler implementations
│   ├── lib.rs            # Library root
│   ├── spec.rs           # Workload specification
│   ├── runtime.rs        # Runtime trait + factory
│   ├── engine.rs         # Decision engine
│   ├── state.rs          # State management (with file locking)
│   ├── migration.rs      # Migration engine (3 strategies)
│   ├── compose.rs        # Multi-workload compose support
│   ├── plugin.rs         # Runtime plugin system
│   ├── health.rs         # Health history and uptime tracking
│   ├── output.rs         # Pretty terminal output
│   ├── adapters/         # Runtime adapters
│   │   ├── podman.rs
│   │   ├── kube.rs
│   │   ├── kubevirt.rs
│   │   └── metal.rs
│   └── ui/               # TUI components
│       ├── app.rs        # App state (incl. search/filter)
│       ├── dashboard.rs  # Dashboard screen
│       ├── logs.rs
│       ├── components.rs
│       └── events.rs     # Keyboard handling
└── tests/                # Integration tests
```

### Design Principles

1. **Trait-based abstraction**
   - All runtimes implement `Runtime` trait
   - Consistent interface across runtimes

2. **Async-first**
   - Use `async`/`await` for I/O operations
   - Leverage `tokio` runtime

3. **Error handling**
   - Use `anyhow::Result` for errors
   - Provide meaningful error messages
   - Use `tracing` for logging

4. **Type safety**
   - Leverage Rust's type system
   - Avoid `unwrap()` in production code
   - Use `Option` and `Result` properly

### Adding a New Runtime

1. **Create adapter file**
   ```bash
   touch src/adapters/myruntime.rs
   ```

2. **Implement Runtime trait**
   ```rust
   use crate::runtime::{Runtime, Instance, Status};
   use async_trait::async_trait;

   pub struct MyRuntime {
       // Runtime state
   }

   #[async_trait]
   impl Runtime for MyRuntime {
       async fn build(&self, spec: &Workload) -> Result<Image> {
           // Implement
       }

       async fn run(&self, image: &Image, spec: &Workload) -> Result<Instance> {
           // Implement
       }

       // ... other methods
   }
   ```

3. **Export from module**
   ```rust
   // In src/adapters/mod.rs
   pub mod myruntime;
   pub use myruntime::MyRuntime;
   ```

4. **Add to RuntimeKind enum**
   ```rust
   // In src/runtime.rs
   pub enum RuntimeKind {
       Podman,
       Kubernetes,
       KubeVirt,
       Metal3,
       MyRuntime,  // Add here
   }
   ```

5. **Integrate with CLI**
   ```rust
   // In src/main.rs
   match runtime_kind {
       RuntimeKind::MyRuntime => {
           let runtime = MyRuntime::new()?;
           runtime.run(&image, &workload).await?
       }
       // ... other runtimes
   }
   ```

6. **Add tests**
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[tokio::test]
       async fn test_myruntime_build() {
           // Test build
       }
   }
   ```

7. **Write documentation**
   - Create `MYRUNTIME.md` guide
   - Add examples
   - Update README

## Testing Best Practices

### Test Coverage

- Aim for >80% coverage
- Test happy paths
- Test error conditions
- Test edge cases

### Mock Dependencies

For integration tests, use mocks:

```rust
#[cfg(test)]
mod tests {
    // Mock external dependencies
    // Test in isolation
}
```

### Continuous Integration

All PRs must pass:
- `cargo test` - All tests
- `cargo fmt` - Formatting
- `cargo clippy` - Linting
- `cargo audit` - Security audit

## Reporting Issues

### Bug Reports

Include:
- Aether version (`aether --version`)
- Operating system
- Steps to reproduce
- Expected behavior
- Actual behavior
- Logs (with `-v` flag)

### Feature Requests

Include:
- Use case description
- Why it's needed
- Proposed solution
- Alternatives considered

## Community

### Communication

- GitHub Issues - Bug reports, feature requests
- GitHub Discussions - Questions, ideas
- Pull Requests - Code contributions

### Code of Conduct

- Be respectful
- Be inclusive
- Be collaborative
- Give constructive feedback

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (Proprietary (HyperSDK)).

## Questions?

If you have questions:
1. Check existing documentation
2. Search closed issues
3. Open a new issue with the "question" label

Thank you for contributing to Aether! 🚀
