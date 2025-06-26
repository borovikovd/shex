# Shex 1.0 — Extended POSIX Shell Specification

*POSIX shell with principled modern extensions and mandatory safety*

---

## Table of Contents

1. [Preface](#preface)
2. [Overview](#overview)
3. [Design Principles](#design-principles)
4. [Syntax & Grammar](#syntax--grammar)
   1. [Source Grammar Family](#41--source-grammar-family)
   2. [Canonical Tokens](#42--canonical-tokens)
   3. [Lexical Rules (1‑18)](#43--lexical-rules-1-18)
   4. [Operator Precedence](#44--operator-precedence)
   5. [Complete Grammar (POSIX + Shex)](#45--complete-grammar-posix--shex)
   6. [Formal JSON Grammar](#46--formal-json-grammar)
   7. [Arithmetic Expression Grammar](#47--arithmetic-expression-grammar)
5. [Safety Model](#safety-model)
6. [Language Details](#language-details)
7. [Error Handling & Recovery](#error-handling--recovery)
8. [Reserved‑Word Contexts](#reservedword-contexts)
9. [Safe‑Subset‑of‑sh (S‑SoS)](#s-sos)
10. [Command‑line Interface](#cli)
11. [Implementation Notes](#implementation-notes)
12. [Appendix A — POSIX‑Compatibility Feature List](#appendix-a--posixcompatibility-feature-list)
13. [Appendix B — Reference Error Messages](#appendix-b--reference-error-messages)
14. [Appendix C — JSON Mode Examples](#appendix-c--json-mode-examples)

---

## Preface

The POSIX shell is still the universal glue of Unix. **Shex 1.0** is a *strict superset* of the normative POSIX grammar (SUS Issue 7, §2.10). It adds block‑scoped variables, native JSON literals, structured error handling, and a mandatory safety layer—*without* breaking existing scripts that rely only on the POSIX core.

> **Compatibility Promise**   *Every script that parses under the normative POSIX grammar (SUS Issue 7, §2.10) ****and uses only the features enumerated in Appendix A**** parses unchanged in Shex.*  Additional run‑time guards may still raise safety errors; see the [Safety Model](#safety-model).

---

## Overview

- **POSIX foundation** – original grammar kept verbatim.
- **Safe by default** – `errexit`, `nounset`, `pipefail`, deny‑list, and parse‑time validation are always on.
- **Block‑scoped variables** – `let` / `const` (à la TypeScript) coexist with classic globals.
- **Native JSON** – RFC 7159 objects & arrays as first‑class literals.
- **Structured error handling** – familiar `try / catch / end` block.
- **Backwards portability** – `--posix` flag enforces a pure POSIX subset.

---

## Design Principles

1. **POSIX Grammar Foundation** – Original POSIX productions are preserved; Shex extensions are integrated as additional alternatives to avoid ambiguities.
2. **Fail Fast & Loud** – Undefined variables, pipeline failures, and dangerous operations abort the script.
3. **Single Canonical Syntax** – One spelling per construct; no dialect forks.
4. **Modern Scoping** – `let`/`const` introduce block‑local variables; globals remain unchanged.
5. **JSON as Data, Not Text** – Literals are parsed, stored, and manipulated as structured values.
6. **Optional Portability Layer (S‑SoS)** – `--posix` guarantees a script will run under `/bin/sh`.

---

## Syntax & Grammar

### 4.1  Source Grammar Family

The POSIX shell grammar is **LALR(1)**; Shex inherits this property.  Any LR‑compatible parser generator (e.g. yacc/bison) suffices.

### 4.2  Canonical Tokens

```yacc
/* -------------------------------------------------------
   POSIX Tokens (from SUS Issue 7)
   ------------------------------------------------------- */
%token  WORD
%token  ASSIGNMENT_WORD
%token  NAME
%token  NEWLINE
%token  IO_NUMBER

/* The following are the operators (see XBD Operator)
   containing more than one character. */
%token  AND_IF    OR_IF    DSEMI
/*      '&&'      '||'     ';;'    */

%token  DLESS  DGREAT  LESSAND  GREATAND  LESSGREAT  DLESSDASH
/*      '<<'   '>>'    '<&'     '>&'      '<>'       '<<-'   */

%token  CLOBBER
/*      '>|'   */

/* The following are the reserved words. */
%token  If    Then    Else    Elif    Fi    Do    Done
/*      'if'  'then'  'else'  'elif'  'fi'  'do'  'done'   */

%token  Case    Esac    While    Until    For
/*      'case'  'esac'  'while'  'until'  'for'   */

/* These are reserved words, not operator tokens, and are
   recognized when reserved words are recognized. */
%token  Lbrace    Rbrace    Bang
/*      '{'       '}'       '!'   */

%token  In
/*      'in'   */

/* -------------------------------------------------------
   Shex Extensions
   ------------------------------------------------------- */
%token  Let Const Try Catch End           /* reserved only in specific positions */
%token  TRUE FALSE NULL                    /* JSON literals */
%token  NUMBER STRING                      /* shell words */
%token  ARITHMETIC_EXPANSION               /* $(( … ))  */
%token  JSON_STRING JSON_NUMBER            /* RFC 7159 tokens, emitted in JSON‑mode */
```

### 4.3  Lexical Rules (1‑18)

The table extends the POSIX **token‑recognition algorithm** (SUS §2.10.2). Rules 1‑9 replicate POSIX verbatim; rules 10‑18 are Shex‑specific.

| #      | Situational Trigger                                                         | Returned Token                                                                              |
| ------ | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| **1**  | Reserved‑word position (start of simple command, after `;` or `<newline>`)  | Specific reserved‑word token; else `WORD`                                                   |
| **2**  | Immediately after a redirection operator (`<`, `>`, `<<`, etc.)             | `WORD` to be used later as a *filename*                                                     |
| **3**  | Word directly following a here‑document operator (`<<`)                     | `WORD` used as here‑doc delimiter (after quote removal)                                     |
| **4**  | Token exactly `esac` inside a `case` pattern list                           | Reserved‑word `Esac`; otherwise `WORD`                                                      |
| **5**  | First word after the `for` keyword                                          | `NAME` if syntactically valid; else `WORD`                                                  |
| **6**  | Third word in `for`/`case` (`in`, `do`)                                     | Reserved‑word `In` or `Do`; otherwise `WORD`                                                |
| **7a** | First word of a simple command and it does **not** contain `=`              | Apply rule 1                                                                                |
| **7b** | Unquoted word *containing* a single `=` with a valid identifier on the left | `ASSIGNMENT_WORD`; else rule 1                                                              |
| **8**  | Word before `()` in a function definition                                   | `NAME` (or reserved) as function name; else rule 7                                          |
| **9**  | Any token inside a function body while parsing                              | Returned verbatim as `WORD` (no assignments)                                                |
| **10** | *Command position* word exactly `let` or `const`                            | Reserved‑word `Let` / `Const`                                                               |
| **11** | *Command position* word exactly `try` or `end`                              | Reserved‑word `Try` / `End`                                                                 |
| **12** | Word exactly `catch` appearing **immediately after** a `try` compound list  | Reserved‑word `Catch`                                                                       |
| **13** | Word containing `[` that is not preceded by whitespace                      | Treat the entire word including brackets as a single `WORD` token (for array access like `arr[0]`) |
| **14** | Unquoted `{` **immediately followed by** `"` or another `{` or `[`, or unquoted `[` at start of word | Enter **JSON mode**; lexer tracks brace/bracket depth until the matching `}`/`]` at depth 0. JSON mode ends at the matching closer |
| **15** | Token inside JSON‑mode matching RFC 7159 string literal                     | `JSON_STRING` (shell expansion is **disabled**)                                             |
| **16** | Token inside JSON‑mode matching RFC 7159 number                             | `JSON_NUMBER`                                                                               |
| **17** | Token inside JSON‑mode exactly `true`, `false`, or `null`                   | `TRUE`, `FALSE`, `NULL`                                                                     |
| **18** | When encountering the sequence `$((` | Scan until matching `))` and emit entire construct as single `ARITHMETIC_EXPANSION` token. The arithmetic expression grammar (4.7) defines the internal syntax |

**Reserved‑Word Context** – The keywords in rules 10–12 are reserved *only* in the circumstances stated; elsewhere they are ordinary `WORD`s. `End` is reserved **only** when closing a `try` block.

**JSON Mode Disambiguation** – JSON mode is entered only when:
- An unquoted `{` is immediately followed by `"` (start of JSON string), or
- An unquoted `{` is immediately followed by another `{` or `[` (nested JSON), or  
- An unquoted `[` appears at the start of a word (not part of existing word)

This distinguishes JSON literals from POSIX constructs. For example:
- `["item"]` enters JSON mode (array literal)
- `var[0]` does NOT enter JSON mode (part of word)
- `$var[0]` does NOT enter JSON mode (part of expansion)
- `{ echo hi; }` remains a POSIX brace group (space after `{`)

### 4.4  Operator Precedence

Within arithmetic expressions `$((...))`, the following precedence applies:

| Precedence (High → Low) | Operators | Context                |
| ----------------------- | --------- | ---------------------- |
| 1                       | `()`      | Grouping               |
| 2                       | `*`, `/`, `%` | Multiplication, division, modulo |
| 3                       | `+`, `-`  | Addition, subtraction  |

Property access `obj[key]` has highest precedence in expression contexts outside arithmetic expansion.

### 4.5  Complete Grammar (POSIX + Shex)

The grammar below shows the complete POSIX shell grammar with Shex extensions integrated. The original POSIX productions are preserved, with Shex alternatives added to `compound_command` and `cmd_suffix` to ensure natural integration without ambiguities.

```yacc
/* -------------------------------------------------------
   The Grammar
   ------------------------------------------------------- */
%start program
%%
program          : linebreak complete_commands linebreak
                 | linebreak
                 ;
complete_commands: complete_commands newline_list complete_command
                 |                                complete_command
                 ;
complete_command : list separator_op
                 | list
                 ;
list             : list separator_op and_or
                 |                   and_or
                 ;
and_or           :                         pipeline
                 | and_or AND_IF linebreak pipeline
                 | and_or OR_IF  linebreak pipeline
                 ;
pipeline         :      pipe_sequence
                 | Bang pipe_sequence
                 ;
pipe_sequence    :                             command
                 | pipe_sequence '|' linebreak command
                 ;
command          : simple_command
                 | compound_command
                 | compound_command redirect_list
                 | function_definition
                 ;

/* ... other productions ... */

/* -------------------------------------------------------
   Shex Extensions
   ------------------------------------------------------- */
let_clause       : Let ASSIGNMENT_WORD             /* Shex new */
                 ;

const_clause     : Const ASSIGNMENT_WORD           /* Shex new */
                 ;

try_clause       : Try compound_list Catch NAME compound_list End  /* Shex new */
                 ;
compound_command : brace_group
                 | subshell
                 | for_clause
                 | case_clause
                 | if_clause
                 | while_clause
                 | until_clause
                 | let_clause                       /* Shex new */
                 | const_clause                     /* Shex new */
                 | try_clause                       /* Shex new */
                 ;
subshell         : '(' compound_list ')'
                 ;
compound_list    : linebreak term
                 | linebreak term separator
                 ;
term             : term separator and_or
                 |                and_or
                 ;
for_clause       : For name                                      do_group
                 | For name                       sequential_sep do_group
                 | For name linebreak in          sequential_sep do_group
                 | For name linebreak in wordlist sequential_sep do_group
                 ;
name             : NAME                     /* Apply rule 5 */
                 ;
in               : In                       /* Apply rule 6 */
                 ;
wordlist         : wordlist WORD
                 |          WORD
                 ;
case_clause      : Case WORD linebreak in linebreak case_list    Esac
                 | Case WORD linebreak in linebreak case_list_ns Esac
                 | Case WORD linebreak in linebreak              Esac
                 ;
case_list_ns     : case_list case_item_ns
                 |           case_item_ns
                 ;
case_list        : case_list case_item
                 |           case_item
                 ;
case_item_ns     :     pattern ')' linebreak
                 |     pattern ')' compound_list
                 | '(' pattern ')' linebreak
                 | '(' pattern ')' compound_list
                 ;
case_item        :     pattern ')' linebreak     DSEMI linebreak
                 |     pattern ')' compound_list DSEMI linebreak
                 | '(' pattern ')' linebreak     DSEMI linebreak
                 | '(' pattern ')' compound_list DSEMI linebreak
                 ;
pattern          :             WORD         /* Apply rule 4 */
                 | pattern '|' WORD         /* Do not apply rule 4 */
                 ;
if_clause        : If compound_list Then compound_list else_part Fi
                 | If compound_list Then compound_list           Fi
                 ;
else_part        : Elif compound_list Then compound_list
                 | Elif compound_list Then compound_list else_part
                 | Else compound_list
                 ;
while_clause     : While compound_list do_group
                 ;
until_clause     : Until compound_list do_group
                 ;
function_definition : fname '(' ')' linebreak function_body
                 ;
function_body    : compound_command                /* Apply rule 9 */
                 | compound_command redirect_list  /* Apply rule 9 */
                 ;
fname            : NAME                            /* Apply rule 8 */
                 ;
brace_group      : Lbrace compound_list Rbrace
                 ;
do_group         : Do compound_list Done           /* Apply rule 6 */
                 ;
simple_command   : cmd_prefix cmd_word cmd_suffix
                 | cmd_prefix cmd_word
                 | cmd_prefix
                 | cmd_name cmd_suffix
                 | cmd_name
                 ;
cmd_name         : WORD                   /* Apply rule 7a */
                 ;
cmd_word         : WORD                   /* Apply rule 7b */
                 ;
cmd_prefix       :            io_redirect
                 | cmd_prefix io_redirect
                 |            ASSIGNMENT_WORD
                 | cmd_prefix ASSIGNMENT_WORD
                 ;
cmd_suffix       :            io_redirect
                 | cmd_suffix io_redirect
                 |            WORD
                 | cmd_suffix WORD
                 |            json_literal          /* Shex new */
                 | cmd_suffix json_literal          /* Shex new */
                 ;
redirect_list    :               io_redirect
                 | redirect_list io_redirect
                 ;
io_redirect      :           io_file
                 | IO_NUMBER io_file
                 |           io_here
                 | IO_NUMBER io_here
                 ;
io_file          : '<'       filename
                 | LESSAND   filename
                 | '>'       filename
                 | GREATAND  filename
                 | DGREAT    filename
                 | LESSGREAT filename
                 | CLOBBER   filename
                 ;
filename         : WORD                      /* Apply rule 2 */
                 ;
io_here          : DLESS     here_end
                 | DLESSDASH here_end
                 ;
here_end         : WORD                      /* Apply rule 3 */
                 ;
newline_list     :              NEWLINE
                 | newline_list NEWLINE
                 ;
linebreak        : newline_list
                 | /* empty */
                 ;
separator_op     : '&'
                 | ';'
                 ;
separator        : separator_op linebreak
                 | newline_list
                 ;
sequential_sep   : ';' linebreak
                 | newline_list
                 ;

/* -------------------------------------------------------
   Shex Extensions
   ------------------------------------------------------- */
/* Note: Shex declarations and try statements are integrated
   into simple_command to avoid grammar ambiguities */

parameter_list   : NAME                            /* Shex new */
                 | parameter_list ',' NAME
                 ;

/* JSON Literals - can appear as cmd_suffix */
json_literal     : json_object | json_array ;             /* Shex new - Apply rule 14 */
json_object      : '{' json_members '}' | '{' '}' ;       /* Tokens from rules 15-17 */
json_members     : json_member | json_members ',' json_member ;
json_member      : JSON_STRING ':' json_value ;           /* Apply rule 15 */
json_array       : '[' json_elements ']' | '[' ']' ;      /* Apply rule 14 */
json_elements    : json_value | json_elements ',' json_value ;
json_value       : JSON_STRING                             /* Apply rule 15 */
                 | JSON_NUMBER                             /* Apply rule 16 */
                 | json_object 
                 | json_array 
                 | TRUE                                    /* Apply rule 17 */
                 | FALSE                                   /* Apply rule 17 */
                 | NULL ;                                  /* Apply rule 17 */

/* Arithmetic (within $((...))) - internal grammar only */
/* Note: This grammar is not part of the main parser; it defines
   the syntax inside ARITHMETIC_EXPANSION tokens which are
   produced by lexical rule 18 */
%%
```

### 4.6  Formal JSON Grammar

```yacc
/* JSON Grammar - used when lexer is in JSON mode */
json_value       : json_string
                 | json_number
                 | json_object
                 | json_array
                 | json_boolean
                 | json_null
                 ;
json_object      : '{' '}'
                 | '{' json_members '}'
                 ;
json_members     : json_member
                 | json_members ',' json_member
                 ;
json_member      : JSON_STRING ':' json_value
                 ;
json_array       : '[' ']'
                 | '[' json_elements ']'
                 ;
json_elements    : json_value
                 | json_elements ',' json_value
                 ;
json_string      : JSON_STRING              /* no shell expansion */
                 ;
json_number      : JSON_NUMBER
                 ;
json_boolean     : TRUE
                 | FALSE
                 ;
json_null        : NULL
                 ;
```

### 4.7  Arithmetic Expression Grammar

The arithmetic expression grammar applies **only** within `$((...))` contexts. The `ARITHMETIC_EXPANSION` token represents the entire `$((...))` construct, and the following grammar defines what can appear inside:

```yacc
/* Grammar for content inside $((...)) */
arithmetic_expression     : additive_expression ;
additive_expression      : multiplicative_expression
                         | additive_expression '+' multiplicative_expression
                         | additive_expression '-' multiplicative_expression
                         ;
multiplicative_expression : unary_expression
                         | multiplicative_expression '*' unary_expression
                         | multiplicative_expression '/' unary_expression
                         | multiplicative_expression '%' unary_expression
                         ;
unary_expression         : arith_primary
                         | '-' unary_expression
                         | '+' unary_expression
                         ;
arith_primary            : NUMBER
                         | NAME
                         | '(' arithmetic_expression ')'
                         ;
```

Note: Arithmetic expressions cannot appear outside `$((...))`. The `ARITHMETIC_EXPANSION` token in the main grammar represents the entire arithmetic expansion including the `$((...))` delimiters.

---

## Safety Model

| Mode        | Parse‑time                                         | Run‑time                                                       |
| ----------- | -------------------------------------------------- | -------------------------------------------------------------- |
| `--posix`   | Rejects any Shex‑only syntax; enforces pure POSIX. | Runs with mandatory guards (`errexit`, `nounset`, `pipefail`). |
| **default** | Accepts all POSIX + Shex syntax.                   | Deny‑list enforcement, path validation, plus mandatory guards. |

### 5.1  Safety Enforcement Order

1. **Parse‑time** – syntax validation.
2. **Semantic pass** – scope & type checks.
3. **Runtime** – deny‑list → whitelist → shell guards.

### 5.2  Runtime Deny‑list (configurable)

```
Commands  : rm dd mkfs fdisk shutdown reboot halt su sudo passwd chown chmod …
Paths     : / /etc /bin /sbin /usr/* /dev /proc /sys /root /var/{log,run,lib}
Patterns  : ".." components, NUL bytes, control chars ≤ 0x1F
```

Use `--allow=<pattern>` or `~/.shex_allowlist` to override specific entries.

### 5.3  Always‑on Guards (non‑configurable)

| Guard                    | Effect                                                                |
| ------------------------ | --------------------------------------------------------------------- |
| `-e`                     | Exit on non‑zero status.                                              |
| `-u`                     | Error on undefined variable.                                          |
| `pipefail`               | Pipeline fails if any segment fails.                                  |
| **Strict interpolation** | `$varX` where `X ∈ [A‑Za‑z0‑9_]` ⇒ compile‑time error; use `${var}X`. |

---

## Language Details

### 6.1  Variables

- **POSIX globals** – Classic shell variables with dynamic scope
- **`let`** – Mutable block‑scoped variable: `let var=value`
- **`const`** – Immutable block‑scoped variable: `const var=value`

Variable declarations use the standard `ASSIGNMENT_WORD` token, maintaining consistency with POSIX.

### 6.2  JSON Literals

JSON literals never undergo shell expansion inside `{ }` / `[ ]`. Variable references or command substitutions within JSON mode result in parse errors.

### 6.3  Error Handling

The `try … catch err … end` construct provides structured error handling. Uncaught errors abort the script.

### 6.4  Functions

Functions use standard POSIX `name() { … }` syntax only.

### 6.5  Property Access

Property access for JSON objects uses standard shell array syntax. The shell already tokenizes `arr[0]` as a single `WORD` when there's no whitespace before `[`. This existing behavior is leveraged for JSON property access:

```sh
config={"port": 8080, "host": "localhost"}
echo ${config[port]}      # Accesses JSON property
echo ${config["host"]}    # String key access
```

No grammar extension is needed; this reuses existing shell mechanisms.

---

## Error Handling & Recovery

On parse error the implementation **must**:

1. Print `Shex:<file>:<line>:<col>: <class>: <message>`.
2. Suggest a POSIX alternative when relevant.
3. Abort; never reinterpret invalid input.

---

## Reserved‑Word Contexts

| Keyword        | Reserved when …                                   |
| -------------- | ------------------------------------------------- |
| `let`, `const` | First unquoted word of simple command             |
| `try`          | First unquoted word of simple command             |
| `catch`        | Immediately after a `try` compound list           |
| `end`          | Terminates a preceding `try` block                |

Outside these positions the words are ordinary `WORD`s.

---

## S‑SoS

Scripts run with `--posix` are guaranteed to execute under `/bin/sh`:

```sh
#!/bin/sh
exec shex --posix "$0" "$@"
```

---

## CLI

```sh
shex script.sh           # default (safe‑by‑default)
shex --posix script.sh   # enforce POSIX subset
shex -c 'echo hi'        # one‑liner
```

---

## Implementation Notes

### 11.1  Parser Speed

A reference implementation parses 10 k‑lines in <50 ms on 2025 hardware.

### 11.2  Error Style

All diagnostics start with `Shex:`.

### 11.3  Extension Points

Experimental keywords must be prefixed `x_` and gated by `--experimental`.

### 11.4  Test Suite

A conformance harness ships in `tests/` and covers every Appendix‑A rule.

### 11.5  JSON Mode Boundary Conditions

JSON mode disambiguation rules:

```sh
# ✅ Enters JSON mode
{"key": "value"}              # { immediately followed by "
["a", "b", "c"]               # [ starts JSON array at word boundary
{{nested}}                    # { immediately followed by {

# ❌ Does NOT enter JSON mode  
{ echo hi; }                  # Space after { - POSIX brace group
{foo}                         # No " after { - POSIX brace group
${var}                        # $ before { - Parameter expansion
var[0]                        # [ is part of existing word
$var[0]                       # [ is part of parameter expansion
arr[0]                        # [ is part of word - not JSON

# Parse errors
{"key": "val"}extra           # Unexpected token after JSON
```

The lexer uses one character of lookahead after `{` to determine whether to enter JSON mode.

---

## Appendix A — POSIX‑Compatibility Feature List

Scripts relying **only** on the features below run unchanged under Shex:

- Basic commands, parameter & arithmetic expansion.
- Control structures: `if`, `while`, `until`, `for`, `case`.
- POSIX function definitions.
- Redirections & pipelines.
- No use of Shex‑only keywords or JSON literals.

---

## Appendix B — Reference Error Messages

| Code                   | Condition                 | Example Diagnostic                                             |
| ---------------------- | ------------------------- | -------------------------------------------------------------- |
| `ERR_UNDEF_VAR`        | Undefined variable        | `Shex: main.sh:8: nounset: $bar is not set`                    |
| `ERR_DANGEROUS_CMD`    | Command matches deny‑list | `Shex: rm -rf / blocked by safety policy`                      |
| `ERR_PATH_TRAVERSAL`   | Path contains `..`        | `Shex: ../../etc/passwd: path traversal blocked`               |
| `ERR_JSON_SYNTAX`      | Malformed JSON            | `Shex: line 15: malformed JSON (unexpected token after value)` |
| `ERR_AMBIGUOUS_INTERP` | Ambiguous "\$varX"        | `Shex: line 22: ambiguous interpolation; use "${var}X"`        |
| `ERR_JSON_EXPANSION`   | Shell syntax in JSON      | `Shex: line 10: shell expansion not allowed in JSON mode`      |

---

## Appendix C — JSON Mode Examples

### C.1  Valid JSON Literals

```sh
# ✅ Valid JSON literals
config={"debug": true, "port": 8080}
items=["apple", "banana", "cherry"]
nested={"user": {"name": "alice", "id": 42}}

# ✅ Valid - JSON in shell string (not JSON mode)
json_text='{"home": "/tmp"}'
echo "$json_text"
```

### C.2  Invalid JSON Mode Usage

```sh
# ❌ Invalid - shell expansion in JSON mode
bad={"home": "$HOME"}        # Parse error: $ not allowed
bad={"count": $(wc -l)}      # Parse error: $( not allowed
bad={"path": `pwd`}          # Parse error: ` not allowed

# ❌ Invalid - extra tokens after JSON
bad={"key": "value"}extra    # Parse error: unexpected token
bad=["a", "b"]suffix         # Parse error: unexpected token
```

### C.3  Property Access

```sh
# ✅ Valid - uses existing shell array syntax
config={"port": 8080}
echo ${config[port]}         # Shell tokenizes config[port] as one WORD

# ✅ Valid - string keys
data={"user-name": "alice"}
echo ${data["user-name"]}    # Quotes needed for non-identifier keys

# ❌ Invalid - spaces break the WORD token
echo ${config [port]}        # Space makes this two tokens
```

### C.4  JSON Whitespace and Formatting

```sh
# ✅ All valid - JSON allows flexible whitespace
compact={"a":1,"b":2}
formatted={
    "name": "example",
    "values": [1, 2, 3]
}
mixed={ "key"   :    "value"  }
```