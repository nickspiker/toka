# Rune/Nautilus System Architecture

## Components

**Rune** - Capability-bounded stack VM
- Executes VSF-encoded bytecode
- ~100 instructions (stack, arithmetic, drawing, I/O, control flow)
- No linear memory, only handles
- Spirix-native arithmetic

**Capsule** - Signed executable bundle
- Contains: Rune bytecode + BLAKE3 hash + TOKEN signature
- Immutable, content-addressed
- Declares required capabilities

**Nautilus** - Browser/compositor
- Parses VSF pages
- Executes Rune capsules
- Renders via unified compositor (viewport-relative coords)
- Backends: Canvas (Portal), GPU (native), e-ink

**FGTW** - Discovery/attestation network
- 42-node Byzantine consensus
- Maps handles → IP addresses + routes
- Stores attestations (handle + route + capsule_hash + timestamp)
- Latest timestamp wins, TOKEN signature required

**Photon** - P2P transport
- Peer discovery via FGTW
- Direct UDP connections
- TOKEN authentication
- VSF wire format

**Portal** - Web compatibility shim
- Nautilus-core compiled to WASM
- Runs in any browser
- Proxies Photon via fgtw.org backend
- SSR on Cloudflare Workers

## Data Flow

**Publishing:**
```
Developer writes Rust
  → rustc --target ferros-runic
  → Rune bytecode (VSF)
  → BLAKE3 hash, TOKEN sign
  → Capsule
  → Publish attestation to FGTW
  → Serve from peer via Photon
```

**Resolution:**
```
User enters photon://fractaldecoder/calculator
  → Query FGTW for handle
  → FGTW returns: peer IPs + latest capsule hash
  → Fetch page VSF from peer (Photon)
  → Page references capsule by hash
  → Fetch capsule, verify signature
  → Nautilus executes Rune with capabilities
  → Compositor renders to viewport
```

**Execution:**
```
Capsule loaded
  → Verify BLAKE3(bytecode) == declared hash
  → Verify ed25519(signature, hash, pubkey)
  → Grant declared capabilities
  → Rune VM interprets instructions
  → Handle I/O capability-checked
  → Drawing ops use viewport fractions
  → Backend rasterizes (Canvas/GPU/e-ink)
```

## Security Model

- Capsules cryptographically verified before execution
- Capabilities granted explicitly, checked per instruction
- No pointers, no linear memory, no buffer overflows
- Handle updates require TOKEN signature
- FGTW consensus prevents single-node attacks
- Content-addressing prevents tampering

## Coordinates

All layout uses viewport fractions (0.0-1.0):
- Same page scales to phone/desktop/projector
- Font size as area fraction: `sqrt(size% * viewport_area)` = pixels
- No media queries needed

## Tech Stack

- VSF - serialization
- Spirix - arithmetic
- TOKEN - identity
- BLAKE3 - content addressing
- ed25519 - signatures
- Rune - execution
- Photon - transport
- FGTW - discovery
- Nautilus - rendering