I'll write all my design decisions, bugs solved, problems encountered over here in this file to help in creating the README and Design doc after finishing the project.


Some gaps in Forge due to complexity reasons:
Hex floating-point literals are not supported.
Long double (L suffix) is stored as f64, so precision is lost.
Identifiers are ASCII-only (C23 allows Unicode), and this can be upgraded later in one place in src/lexer.rs.
