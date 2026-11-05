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
