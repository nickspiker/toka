# Toka

<div align="center">
  <h3>Capability-Bounded Stack VM for Secure Distributed Computing</h3>
</div>

**Status:** v0.0 - Early Development

## Overview

Toka is a stack-based virtual machine designed for executing signed, capability-bounded bytecode in distributed systems. It provides deterministic execution, cryptographic verification, and zero platform-specific behavior.

**Key Features:**
- **Spirix-Native Arithmetic** - Two's complement floating point (no IEEE-754)
- **Capability-Based Security** - Fine-grained permission system
- **Cryptographic Verification** - BLAKE3 hashes, ed25519 signatures
- **Deterministic Execution** - Same bytecode, same results, everywhere
- **VSF Bytecode** - Compact, self-describing binary format
- **Handle-Only Memory** - No pointers, no buffer overflows
- **Viewport Graphics** - Resolution-independent rendering (0.0-1.0 coords)

## Architecture

### Execution Model

Toka is a **stack machine** with:
- **Value Stack** - All operands pushed/popped from stack
- **Local Variables** - Function-local storage slots
- **Handles** - Capability-checked references to resources
- **No Linear Memory** - Eliminates entire classes of vulnerabilities

### Data Flow

```
VSF Capsule (signed bytecode)
  ↓ Verify BLAKE3 hash
  ↓ Verify ed25519 signature
  ↓ Parse bytecode
  ↓ Grant declared capabilities
Rune VM Execution
  ↓ Stack operations
  ↓ Spirix arithmetic
  ↓ Capability-checked I/O
Canvas Rendering (viewport coords)
  ↓ Browser: Canvas 2D API
  ↓ Native: Spirix GPU kernels
Pixels on Screen
```

### Capsule Structure

A **capsule** is an immutable, signed executable bundle:

```
VSF File:
<
  z3◖6◗ Version
  y3◖5◗ Backward compat
  eu◖timestamp◗ Created
  hp3◖31◗ BLAKE3 hash ◖32 bytes◗
  ge◖signature◗ Ed25519 signature
  n3◖sections◗ Section count
>

[bytecode
  (main:{ps}s44◖0.5◗{ps}s44◖0.3◗{fc}{ht})
  (render:{cc}u5◖0xFFFFFFFF◗{fr}{ht})
]

[metadata
  (name:l◖"MyApp"◗)
  (version:z◖1◗)
]
```

**Security Model:**
1. Bytecode content-addressed by BLAKE3 hash
2. ed25519 signature proves authorship
3. Capabilities declared explicitly
4. Verification happens once on load
5. Execution cannot escape capability bounds

## Type System

### Value Types

Rune supports these stack value types:

| Type | Description | Size | Range |
|------|-------------|------|-------|
| `s44` | Spirix F4E4 (default numeric) | 32-bit | ±2^±32768, 65K fraction values |
| `s53` | Spirix F5E3 (extended precision) | 40-bit | ±2^±128, 4B fraction values |
| `u3-u7` | Unsigned integers | 8-256 bit | 0 to 2^(2^N)-1 |
| `i3-i7` | Signed integers | 8-256 bit | -2^(2^N-1) to 2^(2^N-1)-1 |
| `l` | ASCII label/string | Variable | Metadata, names |
| `x` | Huffman text (Unicode) | Variable | Compressed strings |
| `Color` | RGBA color | 32-bit | 0xRRGGBBAA |
| `Bool` | Boolean | 1 bit | true/false |
| `Handle` | Capability reference | 64-bit | Opaque ID |

**No IEEE-754 floats.** Spirix provides deterministic, platform-independent arithmetic without IEEE edge cases (±0, NaN fingerprinting, denormal branches).

**No `usize`/`isize`.** Explicit integer sizes (u3-u7) ensure same bytecode produces identical results on 16/32/64-bit platforms.

### Why Spirix S44?

**S44 (ScalarF4E4)** = 16-bit fraction + 16-bit exponent = 32 bits total

**Advantages:**
- **Single-load aligned** - 32-bit reads, cache-friendly
- **Precision** - 65,536 distinct fraction values (perfect for viewport coords)
- **Dynamic range** - ±2^±32768 (tiny to cosmic)
- **Deterministic** - Same operations, same bit patterns, everywhere
- **GPU-native** - Hardware Spirix units use F4E4 as primary format
- **Compact** - 44% smaller than f64, faster than IEEE f32 in practice

**No IEEE nonsense:**
```rust
// IEEE f32 edge cases that waste cycles:
-0.0 == 0.0  // true, but different bit patterns
NaN != NaN   // breaks transitivity
0.1 + 0.2 != 0.3  // rounding errors

// Spirix S44 is clean:
Zero is Zero (one bit pattern)
Undefined states are deterministic ([℘ ⬇/⬇] always same bits)
Math that works like math (a × b = 0 iff a = 0 or b = 0)
```

## Instruction Set

### Bytecode Format

**Opcodes:** Two lowercase ASCII characters in braces
**Operands:** VSF-encoded values with ◖◗ notation

```
{op}              → Opcode with no operands (4 bytes)
{op}type◖value◗   → Opcode with VSF operand (variable length)

Example:
{ps}s44◖0.5◗      → Push S44 scalar (0.5)
{ad}              → Add top two stack values
{ht}              → Halt execution
```

### Instruction Categories

**Stack Manipulation (6 ops)**
```
{ps} value    - Push constant to stack
{po}          - Pop (discard top)
{du}          - Duplicate top value
{sw}          - Swap top two values
{rt}          - Rotate top three (a b c → b c a)
```

**Local Variables (4 ops)**
```
{la} count    - Allocate N local slots
{lg} index    - Push local[index] to stack
{ls} index    - Pop stack to local[index]
{lt} index    - Copy top to local[index] without popping
```

**Arithmetic (8 ops - all Spirix)**
```
{ad}          - Add (pop b, a; push a + b)
{sb}          - Subtract (pop b, a; push a - b)
{ml}          - Multiply (pop b, a; push a * b)
{dv}          - Divide (pop b, a; push a / b)
{md}          - Modulo (pop b, a; push a % b)
{ng}          - Negate (pop a; push -a)
{mn}          - Min (pop b, a; push min(a,b))
{mx}          - Max (pop b, a; push max(a,b))
```

**Drawing (8 ops - viewport relative 0.0-1.0)**
```
{cc}          - Clear canvas (pop: color)
{fr}          - Fill rect (pop: color, h%, w%, y%, x%)
{fc}          - Fill circle (pop: color, r%, cy%, cx%)
{dl}          - Draw line (pop: width%, color, y2%, x2%, y1%, x1%)
{dt}          - Draw text (pop: size%, y%, x%, string)
{sc}          - Set color (pop: color)
{sr}          - Stroke rect (pop: width%, color, h%, w%, y%, x%)
{sl}          - Stroke circle (pop: width%, color, r%, cy%, cx%)
```

**Control Flow (4 ops)**
```
{br} target   - Branch to instruction index
{bi} target   - Branch if true (pop condition)
{ht}          - Halt execution
{np}          - No operation
```

**Debug (2 ops)**
```
{dp}          - Debug print (pop value, print to console)
{ds}          - Debug stack (print entire stack state)
```

### Example Bytecode

**Draw red circle:**
```
{ps}s44◖0.5◗         # Push x (center)
{ps}s44◖0.3◗         # Push y (center)
{ps}s44◖0.2◗         # Push radius
{ps}u5◖0xFF0000FF◗   # Push red color
{fc}                 # Fill circle
{ht}                 # Halt
```

**Add two numbers:**
```
{ps}s44◖100.0◗       # Push 100
{ps}s44◖42.0◗        # Push 42
{ad}                 # Add (result: 142)
{dp}                 # Debug print
{ht}                 # Halt
```

**Loop with locals:**
```
{la}u3◖2◗            # Allocate 2 locals (counter, sum)
{ps}s44◖0◗{ls}u3◖0◗  # local[0] = 0 (counter)
{ps}s44◖0◗{ls}u3◖1◗  # local[1] = 0 (sum)

# Loop start (instruction 0)
{lg}u3◖0◗            # Push counter
{ps}s44◖10◗          # Push limit
{lt}                 # counter < 10?
{bi}u4◖exit◗         # Branch if false to exit

{lg}u3◖1◗            # Push sum
{lg}u3◖0◗            # Push counter
{ad}                 # sum += counter
{ls}u3◖1◗            # Store sum

{lg}u3◖0◗            # Push counter
{ps}s44◖1◗           # Push 1
{ad}                 # counter + 1
{ls}u3◖0◗            # Store counter

{br}u4◖0◗            # Loop back

# Exit
{lg}u3◖1◗            # Push final sum
{dp}                 # Print it
{ht}                 # Halt
```

## Viewport Graphics

All drawing uses **viewport-relative coordinates** (0.0-1.0):

```
(0.0, 0.0) ────────────────── (1.0, 0.0)
    │                              │
    │                              │
    │         VIEWPORT             │
    │                              │
    │                              │
(0.0, 1.0) ────────────────── (1.0, 1.0)
```

**Benefits:**
- Same layout scales to phone/tablet/desktop/projector
- No media queries needed
- No absolute pixel calculations
- Resolution-independent by design

**Font Size Calculation:**
```rust
viewport_area = width_px × height_px
font_size_px = sqrt(size_fraction × viewport_area)
```

Example: `size=0.01` (1% of viewport) on 1920×1080 screen:
```
area = 1920 × 1080 = 2,073,600
font_px = sqrt(0.01 × 2,073,600) = sqrt(20,736) ≈ 144px
```

## Capability System

### Handle-Based I/O

All external resources accessed via **handles** (opaque u64 IDs):

```rust
// Capability declaration in capsule metadata:
[capabilities
  (canvas_draw:true)
  (file_read:false)
  (network_access:false)
]

// Runtime checks:
{rh}u4◖canvas_handle◗   // read_handle - checks canvas_draw capability
{wh}u4◖file_handle◗     // write_handle - DENIED (file_write not granted)
```

**Handle types:**
- `canvas` - Drawing operations
- `file` - File I/O (read/write)
- `network` - Photon transport
- `buffer` - Memory allocation
- `font` - Font loading

**Security properties:**
- Capabilities granted at capsule load (based on signature/hash)
- Cannot be escalated at runtime
- Handle operations fail if capability not granted
- No raw pointers - impossible to bypass

### No Linear Memory

Rune has **no linear memory model**. Only handles.

**This eliminates:**
- Buffer overflows
- Use-after-free
- Double-free
- Null pointer dereferences
- Memory corruption attacks

**Storage options:**
- Stack (automatic, bounded)
- Locals (function-scoped)
- Handles (capability-checked)

## Platform Support

### v0.0 - Portal (WASM)

**Target:** Browser WASM (Chrome, Firefox, Safari)

**Features:**
- VSF bytecode parsing
- Spirix S44 arithmetic (CPU, no GPU)
- Canvas 2D rendering (software)
- Stack VM execution
- Capability stubs

**Build:**
```bash
wasm-pack build --target web --out-dir www/pkg
```

**Usage:**
```javascript
import init, { RuneVM } from './pkg/rune.js';

await init();
const canvas = document.getElementById('canvas');
const bytecode = new Uint8Array([/* VSF capsule */]);
const vm = new RuneVM(canvas, bytecode);

// Run 1000 instructions per frame
function animate() {
    if (vm.run(1000)) {
        requestAnimationFrame(animate);
    }
}
animate();
```

### v0.x+ - Nautilus (Native)

**Target:** Native browser with GPU stack

**Features:**
- Hardware Spirix GPU kernels (HIP/CUDA)
- Direct frame buffer access
- Zero-copy rendering
- Full capability enforcement
- FGTW network integration
- Photon transport

**Performance:**
- S44 operations → GPU ALU (single cycle)
- Viewport coords → GPU transform (parallel)
- Canvas ops → Frame buffer (DMA)

**No IEEE-754 anywhere in the pipeline.**

## Development

### Project Structure

```
rune/
├── src/
│   ├── lib.rs          # WASM entry point, public API
│   ├── main.rs         # Native CLI runner (testing)
│   ├── value.rs        # Value type system
│   ├── stack.rs        # Stack implementation
│   ├── instruction.rs  # Instruction enum + decoder
│   ├── vm.rs           # VM executor
│   ├── canvas.rs       # Canvas backend (WASM + native)
│   ├── bytecode.rs     # VSF bytecode parser
│   └── error.rs        # Error types
├── www/
│   ├── index.html      # WASM test harness
│   └── pkg/            # wasm-pack output
├── Cargo.toml
├── OPCODES.md          # Full instruction reference
├── SCAFFOLD.md         # Architecture overview
└── README.md           # This file
```

### Dependencies

```toml
[dependencies]
vsf = { path = "../vsf" }          # VSF serialization
spirix = { path = "../spirix" }    # Spirix arithmetic

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["CanvasRenderingContext2d", "HtmlCanvasElement"] }
```

### Build Commands

```bash
# Native build
cargo build --release
cargo run

# WASM build
wasm-pack build --target web --out-dir www/pkg

# Test
cargo test

# Serve WASM locally
python3 -m http.server 8000
# Open http://localhost:8000/www/
```

### Testing Strategy

**Unit Tests:**
- Stack operations (push, pop, dup, swap)
- Instruction decoding
- Value type conversions
- Spirix arithmetic edge cases

**Integration Tests:**
- Native CLI runner with sample bytecode
- Verify stdout matches expected canvas ops
- Round-trip VSF encoding/decoding

**WASM Tests:**
- Visual verification in browser
- Known bytecode → expected shapes/colors
- Performance benchmarks

## Comparison with Other VMs

| Feature | WASM | JVM | Rune |
|---------|------|-----|------|
| Arithmetic | IEEE-754 f32/f64 | IEEE-754 float/double | Spirix S44 (deterministic) |
| Memory Model | Linear memory | Garbage collected heap | Handle-only (no pointers) |
| Security | Sandboxed | Security manager | Capability-based |
| Verification | SHA-256 (optional) | JAR signing (optional) | BLAKE3 + ed25519 (mandatory) |
| Determinism | Platform-dependent NaNs | GC timing varies | Fully deterministic |
| Graphics | WebGL/Canvas via JS | Java2D/JavaFX | Native viewport coords |
| Size overhead | Moderate | Large (JVM runtime) | Minimal (VM + bytecode) |

## Design Philosophy

### Determinism First

**Same bytecode must produce identical results everywhere:**
- No platform-specific types (`usize`, `isize`)
- No IEEE-754 (NaN bit patterns vary by CPU)
- No timing dependencies (no GC pauses)
- No undefined behavior (all edge cases specified)

**Implications:**
- Capsule hash = content identity (reproducible builds)
- Signature verifies behavior (not just bits)
- No fingerprinting attacks (no platform leaks)

### Security Through Simplicity

**Less mechanism = less attack surface:**
- No linear memory → no buffer overflows
- No pointers → no use-after-free
- No type coercion → no confusion attacks
- No dynamic loading → no code injection

**Capabilities instead of ACLs:**
- Unforgeable (cryptographic signatures)
- Delegatable (pass handle to other capsule)
- Revocable (handle invalidation)
- No ambient authority (must be explicitly granted)

### Performance Through Design

**Not "fast despite safety" but "fast because of safety":**
- Spirix has fewer branches than IEEE (no denormal checks)
- Handles eliminate pointer aliasing (optimizer freedom)
- Stack machine is instruction cache-friendly
- VSF bytecode is compact (better I-cache utilization)
- GPU pipeline has no legacy IEEE baggage

## Roadmap

**v0.0 (Current)** - Portal MVP
- [x] Architecture design
- [x] Opcode specification
- [ ] VSF bytecode format
- [ ] Stack VM implementation
- [ ] Canvas 2D backend (WASM)
- [ ] Basic instruction set (~30 ops)
- [ ] Demo: Colorful shapes in browser

**v0.0** - Expanded Instructions
- [ ] Comparison operators
- [ ] Function calls (call/return)
- [ ] More arithmetic (trig, sqrt, pow)
- [ ] More drawing (ellipse, paths, transforms)
- [ ] VSF capsule parsing

**v0.2** - Security & I/O
- [ ] BLAKE3 hash verification
- [ ] ed25519 signature checking
- [ ] Capability enforcement
- [ ] Arrays and strings
- [ ] Memory buffers (via handles)

**v0.3** - Full Instruction Set
- [ ] All 153 instructions
- [ ] Error handling (try/catch)
- [ ] Cryptography ops
- [ ] Time operations
- [ ] Module system (import/export)

**v0.x+** - Nautilus Integration
- [ ] Native browser implementation
- [ ] Spirix GPU backend (HIP kernels)
- [ ] FGTW network integration
- [ ] Photon transport
- [ ] Hardware capability enforcement

## Related Projects

- **VSF** - Versatile Storage Format (bytecode encoding)
- **Spirix** - Two's complement floating point arithmetic
- **TOKEN** - Cryptographic identity system
- **FGTW** - 42-node Byzantine consensus network
- **Photon** - P2P transport protocol
- **Nautilus** - Native browser/compositor

## License

Custom open-source (see LICENSE):
- Free for any purpose (including commercial use)
- Modify and distribute freely
- Patent grant included
- Cannot sell Rune itself as standalone product

Hardware implementation rights reserved - contact for licensing.

## Author

Nick Spiker <nick@verichrome.cc>

## Acknowledgments

- RISC-V for instruction set design principles
- WASM for capability-based security model
- Spirix for deterministic arithmetic foundation
- The realization that IEEE-754 is actually slower in practice

---

**Status:** Early development. Not production-ready. API will change.

**Contribute:** Issues and PRs welcome at https://github.com/fractaldecoder/rune (when public)

**Learn More:** https://holdmyoscilloscope.com/rune/
