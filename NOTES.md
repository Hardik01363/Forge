I'll write all my design decisions, bugs solved, problems encountered over here in this file to help in creating the README and Design doc after finishing the project. The only places where I use AI are for writing NOTES.ms, README.md and DESIGN.md. I handcode everything else, as, my main motive for this project is to learn as much as i can.

# Forge: Architecture, Design Decisions & Engineering Notes

Forge is a hand-crafted C-subset compiler written in Rust targeting x86-64, RISC-V, and AArch64 with an SSA-based middle end.

---

## 1. Stage 1: Rust & Machine Foundations

### 1.1 Lexer Architecture (`src/lexer.rs`)

The lexer is written entirely by hand without parser/lexer generators (no `logos`, `lex`, `pest`, etc.) to understand zero-overhead scanning, exact byte-span tracking, and compiler error recovery.

#### Key Architectural Choices & Strengths
- **Zero-Copy Borrowing:** `Lexer<'a>` borrows `&'a str` source directly without heap allocations for tokens (except token kinds that explicitly need ownership, e.g. identifiers).
- **Exact Spans:** Every `Token` and `LexError` tracks half-open byte ranges `Span { start, end }`, mapping 1:1 back to the source text for precise diagnostic printing.
- **Iterator Protocol with Resilient Recovery:** Implements `Iterator<Item = Result<Token, LexError>>`. On lexical errors (e.g. invalid suffixes, integer overflow, illegal characters), the lexer emits `Some(Err(LexError))` and advances its position past the erroneous sequence. The caller can continue collecting errors across the entire translation unit rather than aborting at the first failure.
- **Explicit EOF Token:** Emits a single `TokenKind::Eof` with a zero-length span at the end of input before returning `None`, simplifying parser peek and lookahead logic.
- **UTF-8 Defensive Decoding:** Character-level skipping uses `chars().next()` and `len_utf8()`. Invalid or unexpected input (e.g. emojis, arbitrary multi-byte UTF-8) will never panic or land `pos` on an unaligned byte boundary inside a multi-byte sequence.
- **C23 Standards Compliance:**
  - Strict suffix validation: rejects non-standard mixed-case suffixes like `lL` and `Ll` while accepting `ll`, `LL`, `uLL`, etc.
  - C23 digit separators (`'`): parsed cleanly across decimal (`1'000'000`), hex (`0xDE'AD`), octal (`0'77`), and binary (`0b10'11`). Rejects illegal placement (consecutive separators `1''0`, trailing separators `100'`, or immediate post-prefix placement `0x'12`).
  - Integer bases: Decimal, Octal (`0...`), Hex (`0x...`), and C23 Binary (`0b...`).
  - Floating-point constants: Decimal floats with fraction (`3.25`, `.5`), exponent (`1e3`, `2.5e-2`), and suffixes (`f`, `F`, `l`, `L`).
- **Comprehensive Test Suite:** 22 unit tests covering boundary conditions (`u64::MAX`, overflow, recovery, suffixes, UTF-8 safety, bases, floating-point precision, digit separators).

#### Known Gaps & Deliberate Deferrals
- **Hexadecimal Floating Constants:** C99/C23 hex floats (e.g. `0x1.fp3`) are currently unsupported.
- **Precision of `long double`:** Constants with suffix `L` or `l` are stored as 64-bit IEEE 754 floats (`f64`). x86-64 80-bit extended precision / 128-bit IEEE quad will be addressed when types are formalized in Stage 2.
- **ASCII-only Identifiers:** Identifiers follow `[a-zA-Z_][a-zA-Z0-9_]*`. C23 extended Unicode identifier syntax (UCNs / XID_Start / XID_Continue) is deferred.

---

## Expression AST Architecture (`src/ast.rs`)

### Design & Storage Strategy: Index-Based Arena
Rather than traditional recursive heap-allocated pointer trees (`Box<Expr>`), Forge uses an **Index-Based Arena** (`AstArena` with strongly-typed 32-bit `ExprId`).

- **Cache Locality & Amortized $O(1)$ Allocation:** All expression nodes live contiguously in a single `Vec<ExprNode>`. Allocating a node is an amortized $O(1)$ push without per-node heap fragmentation.
- **Compact Identifiers:** `ExprId(u32)` is a 4-byte `Copy` value, halving the memory footprint compared to 8-byte 64-bit pointers and eliminating lifetime parameters across AST consumers.
- **Type Safety via Newtype Pattern:** `ExprId` prevents accidental mixing of expression indices with statement IDs or basic block indices.
- **Ergonomics:** Implements `std::ops::Index<ExprId>`, allowing idiomatic array-like access (`arena[id]`) while maintaining safe indexing.
- **Source Mapping:** Every `ExprNode` maintains an exact `Span`, preserving full source provenance for diagnostics and error reporting.

### AST Output Format
Forge implements a Clang-style **Indented Tree Pretty-Printer** (`dump_tree`) displaying operator hierarchy, literal values, and operand relationships. This structure scales seamlessly to statement blocks, symbol resolutions, and explicit type casts in later stages.

---

## Pratt Expression Parser (`src/parser.rs`)

### Parsing Algorithm: Vaughan Pratt's Top-Down Operator Precedence
Expressions are parsed using a single, unified Pratt parsing loop parameterized by **Binding Power** `(left_bp, right_bp)` rather than cascading recursive descent functions:

- **Precedence Hierarchy:**
  - Prefix unary operators (`-`, `~`, `!`): Right binding power `9`.
  - Multiplicative operators (`*`, `/`, `%`): Binding power `(7, 8)`.
  - Additive operators (`+`, `-`): Binding power `(5, 6)`.
- **Associativity via Binding Asymmetry:** Left-associative binary operators enforce `left_bp < right_bp` (e.g. `(5, 6)` for `+` and `-`). In expressions like `a - b - c`, the right binding power of the first operator (`6`) exceeds the left binding power of the second (`5`), naturally forcing left-to-right grouping `(a - b) - c`.
- **Grouping:** Parenthesized sub-expressions `(expr)` reset the minimum binding power to `0` and parse recursively until matching `)`.
- **Lookahead & Diagnostics:** The parser maintains a 1-token lookahead buffer (`peek_token`), converting lexer errors cleanly via `From<LexError>` and emitting detailed `ParseError` diagnostics with offending spans.
- **Snapshot Testing:** Integrated with `insta` for regression-free inline snapshot tests verifying exact AST tree dumps across operator precedence, left-associativity, and nested unary operators.

---

## Target-Independent Intermediate Representation (TAC IR) (`src/ir.rs`)

### Design: Three-Address Code (TAC)
Forge uses a target-independent Three-Address Code intermediate representation. Instructions operate on at most two source operands and write to a single destination operand (`dst = op(src1, src2)`).

- **Operand Representation (`Val`):** Operands are either immediate integer constants (`Val::Constant(i64)`) or named variables/temporaries (`Val::Var(String)`).
- **Infinite Virtual Register Model:** Expressions are linearized by generating fresh, uniquely indexed temporary variables (`tmp.0`, `tmp.1`, etc.) via `var_counter` ($2^{64}-1$ capacity). Temporaries are deliberately not reused at this stage, avoiding artificial anti-dependencies and keeping the IR in a clean static-single-assignment form prior to the middle-end optimizer.
- **Instruction Set:** Supports `Return(Val)`, `Unary { op, src, dst }`, `Binary { op, left, right, dst }`, and `Copy { src, dst }`.
- **Multi-Function Program Container:** `IrProgram` models translation units as `Vec<IrFunction>`, ensuring zero architectural refactoring when progressing from single-function to multi-function programs.

---

## x86-64 Target Architecture & Code Generation (`src/codegen.rs`)

### Two-Step Code Generation Pipeline
To enable future machine-level peephole optimizations, instruction scheduling, and graph-coloring register allocation, Forge avoids direct text emission from IR. Instead, it operates in two distinct stages:

$$\text{TAC IR} \xrightarrow{\text{Instruction Selection}} \text{x86-64 AST} \xrightarrow{\text{Emitter}} \text{GAS / AT\&T Assembly}$$

- **Target-Specific Instruction AST:** Defines explicit target registers (`X86Reg::AX`), operands (`X86Operand::Imm`, `X86Operand::Reg`), and machine instructions (`X86Instruction::Mov`, `X86Instruction::Ret`).
- **System V AMD64 ABI Adherence:**
  - 32-bit integer return values are placed in `%eax`.
  - Emits `.globl <name>` symbol directives for function exports visible to `ld`.
  - Appends `.section .note.GNU-stack,"",@progbits` to mark the stack non-executable, adhering to modern Linux security standards.
- **Assembly Syntax:** Emits standard GNU Assembler (GAS) AT&T syntax (operands formatted as `$imm`, `%reg`).

---

## Compiler Driver & Toolchain Integration (`src/main.rs`)

The compiler executable provides a full driver pipeline adhering to standard compiler interface specifications:
- **Phase Flags:** `--lex` (token dump), `--parse` (AST verification), `--codegen` (IR/x86 AST verification), and `-S` (emit assembly text to `.s`).
- **End-to-End Native Compilation:** Compiles C source files directly to native ELF executables by coordinating with the host toolchain (`gcc` assembler and linker).
- **Test Suite Conformance:** Passes 100% of Nora Sandler's official test suite for minimal C translation units (including line `//` and block `/* ... */` comment stripping).

---

## Unary Operators & Runtime Stack Frame Architecture (`src/codegen.rs`)

### In-Place x86 Unary Instructions & Branchless Logical NOT
- **Negation and Complement:** Lowered to in-place x86 instructions `negl` (two's complement) and `notl` (one's complement bitwise NOT). The compiler first copies the source operand into the destination (`movl src, dst`), then applies the in-place operation.
- **Branchless Logical NOT (`!`):** Synthesized without jump labels using standard branchless condition code idioms:
  ```nasm
  cmpl    $0, <dst>
  sete    %al
  movzbl  %al, <dst>

---

## Binary Operators & x86 Hardware Constraints (`src/codegen.rs`)

### Two-Address Arithmetic Lowering (`+`, `-`, `*`)
- Three-Address Code binary operations (`dst = left OP right`) are lowered into x86's two-address format:
  1. `movl <left>, <dst>`
  2. `<op>l <right>, <dst>` (where `<op>` is `addl`, `subl`, or `imull`).
- **Memory Fixups:**
  - `addl` and `subl` forbid both operands in memory (`addl -4(%rbp), -8(%rbp)`). Fixed by staging the source operand into `%r10d`.
  - `imull` destination **cannot be a memory operand** in two-address form. Fixed by staging `<dst>` into scratch register `%r11d`, multiplying, and writing the product back to `<dst>`.

### Hardware Division & Modulo Quirks (`/`, `%`)
- In x86-64, signed division is executed exclusively via `idivl`:
  1. The 32-bit dividend is moved to `%eax`.
  2. Sign-extended into the 64-bit pair `%edx:%eax` using `cdq` (Convert Doubleword to Quadword).
  3. `idivl <divisor>` produces the quotient in `%eax` and the remainder in `%edx`.
  4. The divisor **cannot be an immediate constant** (`idivl $3` is illegal). Fixed by staging immediate constants into `%r10d`.
