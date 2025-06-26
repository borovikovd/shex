# Shex Shell Interpreter

A modern, safe implementation of a POSIX-compliant shell interpreter written in Rust, featuring enhanced safety, precise error reporting, and planned support for JSON literals and block scoping.

## Project Status

**Early Development** - Core functionality working, actively developed

- ✅ Complete POSIX tokenization and parsing
- ✅ Variable assignment and parameter expansion  
- ✅ Logical operators (`&&`, `||`) and command sequences
- ✅ Comprehensive test suite and CLI interface
- ✅ Precise error reporting with line/column information

## Quick Start

```bash
# Clone and build
git clone https://github.com/borovikovd/shex.git
cd shex
cargo build -p shex-cli       # Build CLI (required first)
cargo test --workspace        # Run all tests

# Run commands
./target/debug/shex-cli -c "echo hello world"

# Run script files  
./target/debug/shex-cli script.sh

# Build release version
cargo build --release -p shex-cli
./target/release/shex-cli -c "echo 'Production ready!'"
```

## Features

### Current Features
- **POSIX Tokenization**: Complete token set including all operators and keywords
- **Simple Commands**: `echo hello world`
- **Variable Assignment**: `name=value echo hello $name`
- **Parameter Expansion**: `echo ${var:-default}`
- **Logical Operators**: `true && echo success || echo failure`
- **Error Reporting**: Precise line/column error locations
- **Safety**: Built-in protection against common shell vulnerabilities

### Upcoming Features
- **Complete POSIX Grammar**: Pipelines, redirections, control flow
- **Block Scoping**: `let` and `const` with lexical scoping
- **JSON Support**: Native JSON literals and property access
- **Try/Catch**: Structured error handling
- **Performance**: Target <50ms for 10k lines

## Architecture

```
shex/
├── crates/
│   ├── shex-lexer/     # POSIX tokenization with logos
│   ├── shex-parser/    # LALRPOP-based parser
│   ├── shex-ast/       # Abstract syntax tree
│   ├── shex-interpreter/ # Command execution
│   └── shex-cli/       # Command-line interface
└── tests/              # Integration and E2E tests
```

## Examples

```bash
# Basic commands
echo "Hello, World!"

# Variable assignment and expansion
name=Alice
echo "Hello, $name!"

# Parameter expansion with defaults
echo "Hello, ${USER:-Anonymous}!"

# Logical operators
test -f config.json && echo "Config found" || echo "Config missing"

# Command sequences
make clean; make build; make test
```

## Testing

```bash
# Run all tests
cargo test --workspace

# Run specific test types
cargo test --test integration  # Integration tests
cargo test --test e2e          # End-to-end tests

# Run package-specific tests
cargo test -p shex-lexer      # Lexer tests
cargo test -p shex-parser     # Parser tests
cargo test -p shex-interpreter # Interpreter tests
```

## Development

### Prerequisites
- Rust 1.70+ with Cargo
- LALRPOP for parser generation

### Building
```bash
# Clean build and test (recommended after clone)
cargo clean
cargo build -p shex-cli       # Build CLI binary (required for e2e tests)
cargo test --workspace        # Run all tests

# Development commands
cargo build                   # Debug build
cargo build --release         # Release build
cargo clippy --workspace      # Lint code
cargo fmt --all              # Format code
```

### Important Notes
- **E2E Tests**: The CLI binary (`target/debug/shex-cli`) must be built before running e2e tests
- **Clean Builds**: After `cargo clean`, run `cargo build -p shex-cli` before testing
- **Test Order**: Build CLI first, then run `cargo test --workspace` for complete test suite

### Project Philosophy
- **Ship working code early**: Incremental development with working features
- **Test-driven development**: Comprehensive test coverage at all levels
- **Error handling first**: Precise error reporting from day one
- **Safety by default**: Built-in protections against shell vulnerabilities

## Error Reporting

Shex provides precise error locations with helpful messages:

```
Shex:script.sh:3:15: ERR_UNDEF_VAR: undefined_variable not found
```

All errors include:
- File name and line/column position
- Stable error codes (e.g., `ERR_UNDEF_VAR`)
- Helpful context and suggestions

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Write tests for your changes
4. Ensure all tests pass (`cargo test --workspace`)
5. Run clippy and fmt (`cargo clippy --workspace && cargo fmt --all`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## Design Goals

- **POSIX Compliance**: Full compatibility with POSIX shell specification
- **Memory Safety**: Rust's ownership system prevents common vulnerabilities
- **Performance**: Target <50ms execution time for 10k line scripts
- **Ergonomics**: Modern shell features while maintaining compatibility
- **Testability**: Comprehensive test coverage with clear separation of concerns

## Roadmap

- [x] **Core Foundation**: Basic shell with error reporting
- [x] **Parser Infrastructure**: LALRPOP integration and robust architecture  
- [x] **POSIX Tokenization**: Complete token set implementation
- [x] **Quality Assurance**: Test infrastructure and code quality
- [ ] **POSIX Grammar**: Complete grammar (pipelines, redirections, control flow)
- [ ] **Safety Features**: Enhanced validation and security checks
- [ ] **Block Scoping**: Modern scoping with let/const
- [ ] **JSON Integration**: Native JSON literals and operations
- [ ] **Error Handling**: Try/catch structured error handling
- [ ] **Performance**: Optimization and polish

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [LALRPOP](https://github.com/lalrpop/lalrpop) parser generator
- Tokenization powered by [Logos](https://github.com/maciejhirsz/logos)
- Inspired by modern shell implementations and POSIX standards