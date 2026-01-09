# Contributing to Quicunnel

Thank you for your interest in contributing to Quicunnel! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful, inclusive, and constructive. We're all here to build great software together.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- A GitHub account

### Development Setup

1. Fork the repository on GitHub
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/quicunnel.git
   cd quicunnel
   ```
3. Install development dependencies:
   ```bash
   cargo install cargo-watch
   cargo install cargo-hack
   ```

## Development Workflow

### Making Changes

1. Create a new branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes and commit:
   ```bash
   git add .
   git commit -m "feat: add your feature"
   ```

3. Run tests:
   ```bash
   cargo test
   ```

4. Run formatting:
   ```bash
   cargo fmt
   ```

5. Run lints:
   ```bash
   cargo clippy -- -D warnings
   ```

### Commit Messages

Follow conventional commits:

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `style:` - Code style changes (formatting)
- `refactor:` - Code refactoring
- `test:` - Test changes
- `chore:` - Maintenance tasks

Examples:
```
feat: add stream timeout configuration
fix: prevent memory leak in reconnection logic
docs: update certificate generation guide
test: add integration tests for heartbeat
```

### Pull Requests

1. Push to your fork:
   ```bash
   git push origin feature/your-feature-name
   ```

2. Open a pull request on GitHub

3. Ensure PR includes:
   - Clear description of changes
   - Related issue numbers
   - Tests for new functionality
   - Documentation updates
   - Example updates (if applicable)

### PR Review Process

- At least one maintainer approval required
- All CI checks must pass
- Code coverage should not decrease
- No unresolved conversations

## Coding Standards

### Rust Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Document all public APIs with doc comments

### Documentation

```rust
/// Brief description.
///
/// Longer description with examples.
///
/// # Errors
///
/// - `ErrorVariant` - When something happens
///
/// # Examples
///
/// ```
/// use quicunnel::Tunnel;
/// ```
pub fn example_function() -> Result<()> {
    // ...
}
```

### Testing

- Unit tests in same file as code
- Integration tests in `tests/` directory
- Examples in `examples/` directory
- Aim for >80% code coverage

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        assert_eq!(2 + 2, 4);
    }
}
```

## Project Structure

```
quicunnel/
├── src/
│   ├── lib.rs           # Public API
│   ├── tunnel.rs        # Main tunnel implementation
│   ├── tls.rs           # TLS configuration
│   ├── endpoint.rs      # QUIC endpoint
│   ├── state.rs         # State machine
│   ├── heartbeat.rs     # Heartbeat service
│   ├── reconnect.rs     # Reconnection logic
│   ├── types.rs         # Type definitions
│   └── error.rs         # Error types
├── examples/            # Example programs
├── benches/             # Benchmarks
├── tests/               # Integration tests
├── docs/                # Additional documentation
└── CONTRIBUTING.md      # This file
```

## Areas Where We Need Help

- Windows platform testing
- Additional QUIC transport features
- Connection migration support
- HTTP/3 integration
- Performance optimization
- Documentation improvements
- More examples

## Questions?

- Open an issue for bugs or feature requests
- Start a discussion for questions
- Join our Discord/Slack (link in README)

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.
