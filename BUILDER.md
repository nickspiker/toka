# Toka Bytecode Builder

Type-safe Rust DSL for building Toka programs with compile-time checking.

## Overview

The builder provides a chainable API where each method corresponds to a Toka opcode. Method names use the same two-letter mnemonics as the bytecode (e.g., `.po()` for `push_one`, `.ad()` for `add`).

## Quick Example

```rust
use toka::builder::Program;
use toka::vm::VM;

// Compute 2 + 3 = 5
let bytecode = Program::new()
    .po()   // push_one → [1]
    .dp()   // dup → [1, 1]
    .ad()   // add → [2]
    .po()   // push_one → [2, 1]
    .po()   // push_one → [2, 1, 1]
    .ad()   // add → [2, 2]
    .po()   // push_one → [2, 2, 1]
    .ad()   // add → [2, 3]
    .ad()   // add → [5]
    .hl()   // halt
    .build();

let mut vm = VM::new(bytecode);
vm.run().unwrap();
```

## Method Reference

### Stack Operations
- `.pz()` - push zero (S44)
- `.po()` - push one (S44)
- `.pp()` - pop
- `.dp()` - duplicate top
- `.sw()` - swap top two

### Arithmetic
- `.ad()` - add
- `.sb()` - subtract
- `.ml()` - multiply
- `.dv()` - divide
- `.ng()` - negate
- `.ab()` - absolute value
- `.sq()` - square root
- `.pw()` - power

### Comparison
- `.eq()` - equal
- `.ne()` - not equal
- `.lo()` - less than (lt)
- `.le()` - less than or equal
- `.gt()` - greater than
- `.ge()` - greater than or equal

### Logic
- `.an()` - logical AND
- `.or()` - logical OR
- `.nt()` - logical NOT

### Drawing
- `.cr()` - clear canvas
- `.fr()` - fill rectangle
- `.sr()` - stroke rectangle
- `.fc()` - fill circle
- `.dt()` - draw text

### Colour Utilities
- `.cb()` - RGB colour (alpha=1.0)
- `.ca()` - RGBA colour
- `.ci()` - colour interpolation

### Control Flow
- `.hl()` - halt
- `.jm(offset)` - jump
- `.ji(offset)` - jump if truthy
- `.jz(offset)` - jump if zero

### Push Values (requires VSF decoder - not yet implemented in VM)
- `.ps_s44(value)` - push S44 float
- `.ps_u32(value)` - push u32
- `.ps_str(text)` - push UTF-8 string

## Benefits

1. **Type Safety**: Rust compiler catches errors at compile time
2. **Readable**: Method names match opcodes, easy to verify against bytecode
3. **IDE Support**: Autocomplete, documentation, refactoring all work
4. **Zero Overhead**: Direct bytecode emission, no runtime parsing
5. **Composable**: Build helper functions for common patterns

## Example: Colour Helper

```rust
fn red_colour() -> Program {
    Program::new()
        .po()  // r = 1.0
        .pz()  // g = 0.0
        .pz()  // b = 0.0
        .cb()  // rgb
}

// Use it
let bytecode = red_colour()
    .ps_s44(0.0)  // x (when push is implemented)
    .ps_s44(0.0)  // y
    .ps_s44(1.0)  // w
    .ps_s44(1.0)  // h
    .fr()         // fill_rect
    .hl()         // halt
    .build();
```

## Mnemonic Reference

The two-letter mnemonics follow consistent patterns:

| Pattern | Examples |
|---------|----------|
| `p*` - push | `po` (one), `pz` (zero), `ps` (stack) |
| `*d` - operations | `ad` (add), `ml` (multiply) |
| `*l` - loops/lerp | `lp` (lerp) |
| `c*` - colour/compare | `cb` (rgb), `cr` (clear) |
| `f*` - fill/float ops | `fr` (fill rect), `fl` (floor) |
| `s*` - stroke/stack | `sr` (stroke rect), `sw` (swap) |

See [OPCODES.md](OPCODES.md) for the complete opcode reference.

## Current Limitations

The `push` opcode (`.ps()` family) generates valid VSF bytecode but the VM hasn't implemented VSF value decoding yet. For now, use `.po()` (push_one) and `.pz()` (push_zero) which work perfectly.

## Running Examples

```bash
cargo run --example builder_demo
cargo test --lib builder
```
