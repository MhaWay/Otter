# Contributing to Otter

Thank you for your interest in contributing to Otter! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- Basic understanding of cryptography concepts (helpful but not required)
- Familiarity with async Rust and libp2p (for networking contributions)

### Setting Up Development Environment

1. Fork and clone the repository:
```bash
git clone https://github.com/your-username/Otter.git
cd Otter
```

2. Build the project:
```bash
cargo build
```

3. Run tests:
```bash
cargo test
```

4. Build documentation:
```bash
cargo doc --no-deps --open
```

## Project Structure

```
Otter/
├── crates/
│   ├── otter-identity/     # Identity & key management
│   ├── otter-crypto/       # Encryption primitives
│   ├── otter-network/      # P2P networking
│   ├── otter-protocol/     # Protocol versioning
│   ├── otter-messaging/    # Message protocol
│   ├── otter-storage/      # Data persistence
│   ├── otter-voice/        # Voice communication
│   └── otter-cli/          # CLI client
├── Cargo.toml              # Workspace configuration
├── README.md               # Project overview
├── LICENSE.md              # Project license
└── CONTRIBUTING.md         # This file
```

## How to Contribute

### Accepting the CLA

**Before your first contribution**, you must sign the Contributor License Agreement (CLA):

1. Read the [CLA document](CLA.md)
2. Add this statement to your first pull request:
   ```
   I hereby accept the terms of the Otter CLA.
   
   Signed: [Your Full Name]
   Date: [YYYY-MM-DD]
   GitHub Username: @[your-username]
   Email: [your-email]
   ```

**Why?** The CLA ensures:
- The project owner can license your contributions under any terms
- You retain copyright but grant necessary rights
- The project can evolve without legal complications
- Commercial development is possible while keeping the codebase secure

### Contribution Guidelines

### Reporting Bugs

When reporting bugs, please include:

- Otter version (from `Cargo.toml`)
- Operating system and version
- Rust version (`rustc --version`)
- Steps to reproduce
- Expected vs. actual behavior
- Relevant logs (with `-vv` flag for verbose output)

Example:
```
**Environment:**
- Otter version: 0.1.0
- OS: Ubuntu 22.04
- Rust: 1.70.0

**Steps to Reproduce:**
1. Run `otter start`
2. Type `/peers`
3. Observe error

**Expected:** List of connected peers
**Actual:** Crash with error message
```

### Suggesting Features

Feature requests are welcome! Please include:

- Clear description of the feature
- Use case / motivation
- Potential implementation approach (if you have ideas)
- Impact on existing functionality

### Pull Requests

1. **Sign the CLA** (required for first contribution)
2. **Create an issue first** for significant changes
3. **Fork the repository** and create a feature branch
4. **Make focused commits** with clear messages
5. **Add tests** for new functionality
6. **Update documentation** as needed
7. **Ensure all tests pass** before submitting
8. **Follow the code style** (run `cargo fmt`)
9. **Run clippy** for linting (`cargo clippy`)

**Note**: The project owner reserves the right to accept, modify, or reject any contribution. All accepted contributions become part of the official codebase under the project's proprietary license.

Example workflow:
```bash
# Create feature branch
git checkout -b feature/add-group-chat

# Make changes and commit
git add .
git commit -m "Add group chat support to messaging layer"

# Run checks
cargo test
cargo fmt
cargo clippy

# Push and create PR
git push origin feature/add-group-chat
```

## Coding Standards

### Rust Style

Follow the official Rust style guide:

```bash
# Format code
cargo fmt

# Check for common mistakes
cargo clippy -- -D warnings
```

### Documentation

- Add doc comments (`///`) for public APIs
- Include examples in doc comments where helpful
- Update `README.md` and other docs for user-facing changes
- Keep `ARCHITECTURE.md` current for structural changes

Example:
```rust
/// Encrypts a message using ChaCha20-Poly1305
///
/// # Arguments
///
/// * `plaintext` - The message to encrypt
/// * `associated_data` - Optional authenticated metadata
///
/// # Examples
///
/// ```
/// let session = CryptoSession::new(&alice, &bob_public)?;
/// let encrypted = session.encrypt(b"Hello", None)?;
/// ```
///
/// # Errors
///
/// Returns `CryptoError::EncryptionFailed` if encryption fails
pub fn encrypt(
    &self,
    plaintext: &[u8],
    associated_data: Option<&[u8]>,
) -> Result<EncryptedMessage, CryptoError> {
    // implementation
}
```

### Testing

- Write unit tests for new functions
- Add integration tests for new features
- Aim for good coverage of critical paths
- Test error cases, not just happy paths

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encryption_decryption() {
        let alice = Identity::generate().unwrap();
        let bob = Identity::generate().unwrap();
        
        let bob_public = PublicIdentity::from_identity(&bob);
        let session = CryptoSession::new(&alice, &bob_public).unwrap();
        
        let plaintext = b"Secret message";
        let encrypted = session.encrypt(plaintext, None).unwrap();
        let decrypted = session.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted.as_slice());
    }
    
    #[test]
    fn test_invalid_decryption() {
        // Test error handling
    }
}
```

### Security

Security is paramount for Otter. When contributing:

- **Never commit secrets** or private keys
- **Use secure random** generation (e.g., `OsRng`)
- **Follow cryptographic best practices**
- **Handle sensitive data carefully** (zero on drop when possible)
- **Validate all inputs** from network and users
- **Document security assumptions**

For security-sensitive changes, please:
1. Explain the security model
2. Reference relevant standards (e.g., NaCl, RFC)
3. Consider asking for security review

### Performance

- Profile before optimizing
- Add benchmarks for performance-critical code
- Document performance characteristics
- Avoid premature optimization

Example benchmark:
```rust
#[bench]
fn bench_encryption(b: &mut Bencher) {
    let session = setup_session();
    let plaintext = vec![0u8; 1024];
    
    b.iter(|| {
        session.encrypt(&plaintext, None)
    });
}
```

## Areas for Contribution

### High Priority

1. **WebRTC Integration**: Add voice/video chat support
2. **Perfect Forward Secrecy**: Implement ephemeral key exchange
3. **Group Chat**: Multi-party encrypted messaging
4. **File Transfer**: Encrypted file sharing
5. **Mobile Support**: iOS and Android clients

### Medium Priority

1. **Persistent Storage**: Encrypted message history
2. **Contact Management**: Peer address book
3. **NAT Traversal**: Improve connectivity
4. **Performance**: Optimize crypto and networking
5. **Testing**: Expand test coverage

### Good First Issues

1. **Documentation**: Improve examples and guides
2. **Error Messages**: Make errors more helpful
3. **CLI UX**: Improve command-line interface
4. **Logging**: Better log messages and filtering
5. **Configuration**: Add config file support

## Development Tips

### Running Tests in Watch Mode

```bash
cargo install cargo-watch
cargo watch -x test
```

### Debugging

Enable verbose logging:
```bash
RUST_LOG=otter=debug,libp2p=debug cargo run -p otter-cli -- start
```

### Testing Locally

Run multiple peers:
```bash
# Terminal 1
.\target\release\otter.exe --nickname Alice

# Terminal 2  
.\target\release\otter.exe --nickname Bob --port 9001
```

### Benchmarking

```bash
cargo bench
```

### Code Coverage

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## Review Process

Pull requests are reviewed for:

1. **Correctness**: Does it work as intended?
2. **Security**: Are there security implications?
3. **Quality**: Is the code well-written and tested?
4. **Documentation**: Is it documented appropriately?
5. **Style**: Does it follow project conventions?

Expect:
- Constructive feedback
- Requests for changes
- Discussion of approaches
- Merge when requirements are met

## Communication

- **GitHub Issues**: Bug reports and feature requests
- **Pull Requests**: Code contributions and discussions
- **Commit Messages**: Clear descriptions of changes

## License

By contributing to Otter, you agree to the terms of the [Contributor License Agreement](CLA.md).

Your contributions will be incorporated into the project under the proprietary license, 
but you retain copyright to your original work. The project owner gains the right to 
use, modify, and distribute your contributions under any license, including commercial licenses.

## Code of Conduct

Be respectful, inclusive, and professional. We're all here to build something useful together.

**Security First**: Never compromise on security or privacy. All contributions must maintain 
the project's high standards for cryptographic implementation and user protection.

## Questions?

For licensing questions, commercial inquiries, or permissions:
- Email: info@ggally.net
- Website: https://ggally.net
- Open an issue for technical questions

Thank you for contributing to Otter! 🦦
