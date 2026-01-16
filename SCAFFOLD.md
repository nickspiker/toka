# Toka/Nautilus System Architecture

## Components

**Toka** - Capability-bounded stack VM
- Executes VSF-encoded bytecode
- ~100 instructions (stack, arithmetic, drawing, I/O, control flow)
- No linear memory, only handles
- Spirix-native arithmetic

**Capsule** - Signed executable bundle
- Contains: Toka bytecode + VSF serialization
- Immutable, content-addressed
- Declares required capabilities

**Nautilus** - Browser/compositor
- Parses VSF pages
- Executes Toka capsules
- Renders via unified compositor (viewport-relative coords)
- Backends: Canvas (Portal), GPU (native), e-ink, print, etc.

**FGTW** - Discovery/attestation network
- 42-node Byzantine consensus
- Maps handles → IP addresses + routes
- Stores attestations (handle + route + capsule_hash + timestamp)
- Latest timestamp wins, TOKEN signature required

**PT** - Photon Transpart
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
  → Toka bytecode (VSF)
  → BLAKE3 hash, TOKEN sign
  → Capsule
  → Publish attestation to FGTW
  → Serve from peer via Photon
```

**Resolution:**
```
User enters "free the pizza!"
  → The plaintext handle is encoded as VSF text and hashed
  → A 1 second memory hard proof is computed
  → Query FGTW for handle proof
  → FGTW returns: peer IPs that host the latest capsule for "free the pizza!"
  → Fetch page VSF from peer (Photon Transport)
  → Page references capsule by hash
  → Fetch capsule, verify signature
  → Nautilus executes Toka with capabilities
  → Compositor renders to viewport
```

**Execution:**
```
Capsule loaded
  → Verify BLAKE3(bytecode, VSF hb type or signature for integrity) == declared hash
  → Verify ed25519(signature, hash, pubkey)
  → Grant declared capabilities
  → Toka VM interprets instructions
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
- User is notified if handle changed ownership since last visit

## Coordinates

All layout uses viewport fractions (0.0-1.0):
- Same page scales to phone/desktop/projector
- Font size as area fraction: `sqrt(size% * viewport_area)` = pixels
- No media queries needed

## Tech Stack

- Toka - execution
- VSF - serialization
- Spirix - arithmetic
- Photon Transport - delivery with VSF integrity
- TOKEN - identity
- FGTW - discovery
- Nautilus - rendering
- Photon handle proof style - content addressing