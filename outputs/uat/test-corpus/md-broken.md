# Broken Markdown Fixture

This fixture is intentionally malformed and must not crash the renderer.

Dangling bracket: [

Unclosed table:

| A | B |
|---|---|
| one | two

Unclosed code fence:

```rust
fn broken() {
    println!("missing fence");

Broken mermaid:

```mermaid
nope
