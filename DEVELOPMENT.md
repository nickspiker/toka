# Toka Development Guide

Quick reference for the Toka graphics engine development workflow.

## Development Scripts

### `./build.sh` - Full build pipeline
Comprehensive build script that:
1. ✅ Builds Toka in release mode
2. ✅ Generates capsules (box.vsf)
3. ✅ Verifies capsules with vsfinfo (if available)
4. ✅ Starts web server (if not running)
5. ✅ Displays ready-to-use URLs

```bash
./build.sh
```

**Output**:
```
╔══════════════════════════════════════════════════════════════╗
║              Toka Development Build Pipeline                ║
╚══════════════════════════════════════════════════════════════╝

[1/5] Building Toka...
✓ Toka built successfully

[2/5] Generating capsules...
✓ Generated box.vsf (1234 bytes)

[3/5] Verifying capsule...
[capsule info displayed]

[4/5] Managing web server...
✓ Web server started (PID: 12345)

[5/5] Build complete!
╔══════════════════════════════════════════════════════════════╗
║                     Ready to Test                           ║
╚══════════════════════════════════════════════════════════════╝

Web Server:  http://localhost:8000
Capsule:     /path/to/www/capsules/box.vsf
```

### `./dev.sh` - Quick rebuild cycle
Convenience wrapper for rapid iteration:
- Stops existing server
- Runs full build pipeline
- Optionally opens browser

```bash
# Rebuild only
./dev.sh

# Rebuild and open browser
./dev.sh --open
./dev.sh -o
```

### `./stop.sh` - Stop web server
Cleanly stops the development server:

```bash
./stop.sh
```

## Manual Workflow

If you prefer manual control:

### 1. Build Toka
```bash
cargo build --release --example box
```

### 2. Generate capsule
```bash
./target/release/examples/box
```

This creates `www/capsules/box.vsf`

### 3. Verify capsule (optional)
```bash
vsfinfo www/capsules/box.vsf
```

**Note**: Requires vsfinfo built with spirix feature:
```bash
cd /mnt/Octopus/Code/vsf
cargo build --release --bin vsfinfo --features "text,spirix"
cp target/release/vsfinfo ~/bin/
```

### 4. Start web server
```bash
cd www
./serve.sh
```

Or manually:
```bash
cd www
basic-http-server . -a 127.0.0.1:8000 -x
```

### 5. Test in browser
```
http://localhost:8000
```

## Architecture

### Toka Pipeline
```
┌─────────────┐
│ Rust Code   │  examples/box.rs
└──────┬──────┘
       │ cargo build
       ▼
┌─────────────┐
│ Toka Binary │  target/release/examples/box
└──────┬──────┘
       │ execute
       ▼
┌─────────────┐
│ VSF Capsule │  www/capsules/box.vsf
└──────┬──────┘
       │
       ├─────── vsfinfo (verify)
       │
       ▼
┌─────────────┐
│ Web Browser │  http://localhost:8000
└─────────────┘
```

### VSF Capsule Structure
Toka's innovation: **Everything is a VSF type**

```rust
[(field "main": {ps}, row(transform, children), {rl}, {hl})]
```

- `{ps}` = Push opcode (VSF type)
- `row(...)` = Transform group ro* type (VSF type)
- `rob(...)` = Box renderable object (VSF type)
- `roc(...)` = Circle renderable object (VSF type)
- `{rl}` = Render loom opcode (VSF type)
- `{hl}` = Halt opcode (VSF type)

**No intermediate representation** - opcodes and scene graph are unified!

### Renderer Flow
```
VSF Bytecode → VM Execute → Direct Canvas Rendering
```

1. **VM** ([src/vm.rs](src/vm.rs)): Executes opcodes, builds scene graph
2. **Renderer** ([src/renderer.rs](src/renderer.rs)): Directly renders ro* types to Canvas
3. **No Loom**: Intermediate representation removed (was [src/loom.rs](src/loom.rs) - deleted)

## Common Issues

### vsfinfo parse error
**Error**: `Invalid usize size marker: 111` (byte 0x6F = 'o')

**Cause**: vsfinfo built without `spirix` feature

**Fix**:
```bash
cd /mnt/Octopus/Code/vsf
cargo build --release --bin vsfinfo --features "text,spirix"
cp target/release/vsfinfo ~/bin/
```

### Web server port conflict
**Error**: Port 8000 already in use

**Fix**:
```bash
./stop.sh
# Or manually find and kill
lsof -ti:8000 | xargs kill
```

### Stale capsule
If browser shows old capsule after rebuild:

1. Check capsule timestamp:
   ```bash
   ls -lh www/capsules/box.vsf
   ```

2. Hard refresh browser:
   - **Chrome**: Ctrl+Shift+R (Linux/Windows) or Cmd+Shift+R (macOS)
   - **Firefox**: Ctrl+F5 (Linux/Windows) or Cmd+Shift+R (macOS)

3. Or clear cache and rebuild:
   ```bash
   rm www/capsules/*.vsf
   ./build.sh
   ```

## Testing Changes

### After modifying Rust code
```bash
./dev.sh --open
```

### After modifying examples/box.rs
```bash
./build.sh  # Rebuilds and regenerates capsule
```

### After modifying www/ files (HTML, JS, CSS)
Just refresh browser - no rebuild needed!

## Performance Monitoring

### View server logs
```bash
tail -f /tmp/toka-server.log
```

### Capsule size
```bash
du -h www/capsules/*.vsf
```

### Build time
```bash
time ./build.sh
```

## Files Modified by Scripts

**build.sh** creates/updates:
- `/tmp/toka-server.pid` - Server process ID
- `/tmp/toka-server.log` - Server output log
- `www/capsules/*.vsf` - Generated capsules

**stop.sh** removes:
- `/tmp/toka-server.pid`

**dev.sh** is a wrapper - modifies nothing directly

## Integration with VSF

Toka uses [VSF](../vsf) for capsule format:
- **Encoding**: [src/builder.rs](src/builder.rs) → VSF writer
- **Decoding**: VSF parser → [src/vm.rs](src/vm.rs)
- **Rendering**: [src/renderer.rs](src/renderer.rs) → Canvas

Cross-project dependencies:
```
toka/ ─depends on─> vsf/ ─depends on─> spirix/
```

## Related Documentation

- **VSF Format**: [/mnt/Octopus/Code/vsf/README.md](../vsf/README.md)
- **Spirix Types**: [/mnt/Octopus/Code/spirix/README.md](../spirix/README.md)
- **Toka Architecture**: [README.md](README.md)
- **WebGPU Port**: [/mnt/Octopus/Code/spirix/gpu/webgpu/README.md](../spirix/gpu/webgpu/README.md)
