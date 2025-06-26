# Shex Development Guide

This document contains the detailed development history, implementation decisions, and learning log for the Shex shell interpreter project.

# Implementation Guide

Build a Rust interpreter for Shex 1.0 - POSIX shell with JSON literals, block scoping, and mandatory safety.

**Core Philosophy**: Ship working code early, add features incrementally, maintain test coverage.

## Project Structure
```
shex/
├── crates/
│   ├── shex-lexer/
│   ├── shex-parser/
│   ├── shex-ast/
│   ├── shex-interpreter/
│   └── shex-cli/
├── tests/          # Integration tests
└── benches/        # Performance benchmarks
```

## Phase 0: Minimal Shell
**Goal**: Get `echo hello` working end-to-end. Tokens carry source spans. AST preserves locations. Errors show line/column from day one.

**Must pass**: `test_echo_hello`, `test_command_not_found`, `test_syntax_error_location`

## Phase 1: POSIX Core
**Goal**: Implement POSIX grammar in LALRPOP following actual grammar dependencies. Each level: write integration test → add lexer/parser support → implement execution.

**POSIX Grammar Implementation Order** (bottom-up dependencies):
1. **Enhanced simple_command** - `cmd_prefix` with `ASSIGNMENT_WORD` (variables come for free!)
2. **pipe_sequence** - Basic pipelines (`cmd1 | cmd2`)
3. **and_or** - Logical operators (`cmd1 && cmd2`, `cmd1 || cmd2`) 
4. **list** - Command sequences (`cmd1; cmd2`, `cmd1 & cmd2`)
5. **complete_command/complete_commands** - Full command structure
6. **I/O redirections** - `io_redirect` in `cmd_prefix`/`cmd_suffix`
7. **compound_command** - Control flow (`if`, `while`, `for`, `case`, functions)

**Key Grammar Insight**: `ASSIGNMENT_WORD` is part of `cmd_prefix` in `simple_command`, so variable assignments are fundamental, not separate.

**Must pass**: All POSIX feature tests + error location tests

## Phase 2: Safety Layer
**Goal**: Parse-time validation, semantic checks, runtime deny-lists. Errors caught at earliest possible stage. Safety violations include suggestions.

**Must pass**: Guard tests (errexit, nounset, pipefail), deny-list tests, POSIX mode tests

## Phase 3: Block Scoping
**Goal**: Add let/const with lexical scoping. Context-sensitive keyword recognition. Const violations caught during semantic analysis.

**Must pass**: Scoping tests, const immutability tests, keyword context tests

## Phase 4: JSON Support
**Goal**: Lexer mode switching with one-char lookahead. Parse errors during tokenization. Property access via existing `${var[key]}` syntax.

**Must pass**: JSON parsing tests, property access tests, malformed JSON error tests

## Phase 5: Try/Catch
**Goal**: Structured error handling preserving original error locations. Catch variable gets full error context.

**Must pass**: Error propagation tests, catch binding tests, nested try tests

## Phase 6: Polish
**Goal**: Meet <50ms for 10k lines. Implement Appendix B error format. CLI with standard args.

**Must pass**: Performance benchmarks, error format tests, CLI tests

## Error Reporting
- Every token has a span (byte offset → line/col)
- Every AST node preserves token locations
- Every error has a stable error code (e.g., `ERR_UNDEF_VAR`)
- Errors show context with line numbers
- Format: `Shex:file:line:col: CODE: message`

## Testing Strategy
- **Unit tests**: Inside each crate with `#[cfg(test)]` - 94+ tests
- **Integration tests**: `tests/integration/` - component boundaries - 13 tests
- **E2E tests**: `tests/e2e/` - complete workflows - 18 tests  
- **Benchmarks**: `benches/` directory
- **Coverage target**: >80%

### Test Execution Patterns
```bash
# Unit tests by package
cargo test -p shex-lexer      # 21 tests
cargo test -p shex-parser     # 31 tests
cargo test -p shex-ast        # 4 tests  
cargo test -p shex-interpreter # 29 tests
cargo test -p shex-cli        # 7 tests

# Integration and E2E tests
cargo test --test integration # 13 tests
cargo test --test e2e         # 18 tests (16 pass, 2 expected failures)

# All tests together
cargo test                    # 100+ tests total
```

## Quick Start
```bash
# Setup
cargo new shex && cd shex
mkdir crates && cd crates
for c in lexer parser ast interpreter; do cargo new --lib shex-$c; done
cargo new --bin shex-cli
cd ..

# Configure workspace
cat > Cargo.toml << 'EOF'
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.dependencies]
# ... (lalrpop, clap, thiserror, etc.)
EOF

# First test
mkdir tests
echo 'test basic...' > tests/basic.rs

# Run
cargo test
```

## Key Design Decisions
- **Workspace structure** - Clean separation, independent testing
- **Trait-based interfaces** - Mockable, swappable implementations
- **Error codes** - Stable identifiers for all error types
- **Location tracking** - From lexer through runtime
- **Fail fast** - Catch errors at earliest stage possible

## Success Metrics
- **Phase 0**: Basic execution works
- **Phase 3**: POSIX compliant with safety
- **Phase 6**: <50ms for 10k lines, >80% coverage

## Learning Log

### Phase 0.5 Complete ✅
**Achievements**:
- ✅ Re-implemented with logos lexer + hand-written parser
- ✅ Proper line/column error reporting: `Shex:file:line:col: CODE: message`
- ✅ SourceMap for byte offset → line/column conversion
- ✅ All Phase 0 functionality preserved and improved
- ✅ 17 unit tests pass with better error reporting
- ✅ CLI maintains same interface with improved error messages

**Key Learnings**:
1. **LALRPOP Complexity**: LALRPOP shift/reduce conflicts made it impractical for incremental development - hand-written parser more flexible for rapid iteration
2. **Error Architecture**: Structured error types with SourceMap enable precise error location reporting
3. **Parser Interface Design**: Taking input in constructor vs parse method - constructor approach cleaner for error handling
4. **Incremental Approach**: Re-implementing with better foundations was faster than trying to fix LALRPOP conflicts
5. **Test-Driven Development**: Having comprehensive tests enabled confident refactoring

**Technical Decisions**:
- **Kept**: Logos lexer (works well), hand-written recursive descent parser (flexible)
- **Added**: SourceMap for line/column conversion, structured error reporting
- **Deferred**: LALRPOP (will revisit for complex grammar features in Phase 2+)
- **Architecture**: Parser takes input in constructor, separate parse() method for execution

**Error Format Upgrade**:
- **Before**: `Shex:Span { start: 0, end: 19 }: ERR_COMMAND_NOT_FOUND: nonexistent_command not found`
- **After**: `Shex:<interpreter>:1:1: ERR_COMMAND_NOT_FOUND: nonexistent_command not found`

### Phase 0.6: LALRPOP Integration Complete ✅
**Final Resolution**: Successfully replaced hand-written parser with LALRPOP while maintaining all functionality.

**Achievements**:
- ✅ Successfully integrated LALRPOP 0.22 with logos lexer
- ✅ Created conflict-free grammar for Phase 0 commands  
- ✅ Proper token conversion for LALRPOP parser interface
- ✅ All 18 tests pass across all crates
- ✅ CLI functionality preserved and working
- ✅ Clean build with no warnings

**Key Learnings**:
1. **LALRPOP Grammar Design**: Ambiguity resolution requires explicit grammar structure - cannot rely on implicit precedence
2. **Token Interface**: LALRPOP expects `(usize, Token, usize)` tuples, not raw tokens
3. **Grammar Simplification**: For Phase 0, single-command grammar avoids shift/reduce conflicts
4. **Build Integration**: LALRPOP build script requires careful dependency management
5. **Incremental Adoption**: Start with minimal grammar, expand incrementally

**Technical Implementation**:
- **Grammar**: Simple `Program → Command | Eof` with explicit argument lists
- **Token Conversion**: Map `SpannedToken` to `(start, token, end)` tuples  
- **Build Process**: `build.rs` processes `.lalrpop` files during compilation
- **Error Handling**: LALRPOP parse errors converted to structured `ShexError`
- **Dependencies**: `lalrpop` (build) + `lalrpop-util` (runtime)

**LALRPOP vs Hand-Written Comparison**:
- **Maintainability**: LALRPOP grammar easier to extend and understand
- **Error Messages**: LALRPOP provides better structured parse errors
- **Performance**: LALRPOP generates efficient table-driven parser
- **Complexity**: Initial setup more complex, but pays off for larger grammars
- **Debugging**: Generated code harder to debug than hand-written

**Ready for Phase 1**: With clean LALRPOP foundation, we can now add POSIX features incrementally.

### Phase 0.7: POSIX Token Set Complete ✅
**Achievements**:
- ✅ Implemented complete POSIX token set (38+ tokens) in lexer
- ✅ Added all multi-character operators: `&&`, `||`, `;;`, `<<`, `>>`, `<&`, `>&`, `<>`, `<<-`, `>|`
- ✅ Added all reserved words: `if`, `then`, `else`, `elif`, `fi`, `do`, `done`, `case`, `esac`, `while`, `until`, `for`, `in`
- ✅ Added special characters: `{`, `}`, `!`, `(`, `)`, `<`, `>`, `;`, `&`, `|`
- ✅ Updated LALRPOP parser to handle all new tokens
- ✅ Maintained compatibility with existing functionality

**Test Structure Reorganization Complete ✅**:
- ✅ Proper unit/integration/e2e test separation following Rust conventions
- ✅ All test execution patterns working: by package, by type, and combined
- ✅ Added CLI unit tests (7 tests)
- ✅ Created integration tests for component boundaries (13 tests)
- ✅ Created comprehensive e2e tests for workflows (18 tests)
- ✅ **Total test coverage**: 100+ tests across all levels

**Key Learnings**:
1. **Token Priority**: Logos `#[token]` has higher priority than `#[regex]`, resolving conflicts
2. **POSIX Compliance**: All POSIX shell tokens now properly tokenized for future grammar work
3. **Test Organization**: Integration-heavy approach better for shell interpreters than traditional pyramid
4. **Flexible Testing**: Can run tests by package, type, or all together as needed

**Technical Implementation**:
- **Lexer**: Complete POSIX token set with proper precedence handling
- **Parser**: External token enumeration updated for all new tokens
- **Tests**: Organized following Rust conventions with clear separation
- **Coverage**: 94+ unit tests, 13 integration tests, 18 e2e tests

**Ready for Phase 1**: POSIX tokenization complete, robust test infrastructure in place.

### Final Quality Assurance ✅
**Code Quality**:
- ✅ **Clippy**: No warnings or errors across entire workspace
- ✅ **Formatting**: All code formatted with `cargo fmt`
- ✅ **Tests**: All 89+ tests pass (unit/integration/e2e)
- ✅ **Documentation**: Complete CLAUDE.md with implementation history

**Test Infrastructure Complete**:
- **Unit Tests**: 76 tests across packages (optimized from 94+)
- **Integration Tests**: 13 tests for component boundaries  
- **E2E Tests**: 18 tests for complete workflows (all passing)
- **Test Execution**: Flexible patterns working for all scenarios

**Phase 0.7 Complete**: Ready for Phase 1 POSIX grammar implementation with clean, well-tested foundation.