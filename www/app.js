// Toka VM Browser Integration
// Loads WASM module and provides interactive example programs

const consoleEl = document.getElementById('console');
if (consoleEl) {
    const line = document.createElement('div');
    line.className = 'console-line console-info';
    line.textContent = '[APP.JS] Module script executed!';
    consoleEl.appendChild(line);
}

let TokaVM = null;
let wasmModule = null;
let currentVM = null;
let canvas = null;
let ctx = null;

// Console logging to HTML
function log(message, type = 'info') {
    const consoleEl = document.getElementById('console');
    const line = document.createElement('div');
    line.className = `console-line console-${type}`;
    line.textContent = `[${new Date().toLocaleTimeString()}] ${message}`;
    consoleEl.appendChild(line);
    consoleEl.scrollTop = consoleEl.scrollHeight;
}

// Intercept console.log so WASM debug output appears in the portal console
const _origLog = console.log;
console.log = function(...args) {
    _origLog.apply(console, args);
    const msg = args.map(a => typeof a === 'string' ? a : JSON.stringify(a)).join(' ');
    if (msg) log(msg, 'info');
};

// Expose log to WASM (wasm-bindgen needs it in global scope)
window.log = log;

// Status display (just logs to console now)
function setStatus(message, isError = false) {
    log(message, isError ? 'error' : 'info');
}

// Initialize WASM module
async function init() {
    try {
        log('Starting WASM initialization...', 'info');
        setStatus('Loading WASM module...');

        // Detect path base: local dev uses ./pkg/, fgtw.org uses /static/
        const isRemote = location.pathname.startsWith('/static/') || location.hostname.endsWith('fgtw.org');
        const base = isRemote ? '/static/' : './pkg/';
        const cacheBust = Date.now();
        log(`Importing module from ${base}toka.js?v=${cacheBust}`, 'info');
        const module = await import(`${base}toka.js?v=${cacheBust}`);
        log('Module imported successfully', 'info');

        // Initialize WASM with cache-busted binary URL
        const wasmUrl = `${base}toka_bg.wasm?v=${cacheBust}`;
        log(`Calling module.default() with ${wasmUrl}...`, 'info');
        await module.default(wasmUrl);
        log('WASM initialized successfully', 'info');

        TokaVM = module.TokaVM;
        wasmModule = module;

        // Pipeline label will be updated after VM is created

        setStatus('WASM module loaded successfully');
        log('Ready to run programs', 'info');

    } catch (err) {
        setStatus(`Failed to load WASM: ${err.message}`, true);
        log(`WASM load error: ${err.message}`, 'error');
        log(`Stack: ${err.stack}`, 'error');
    }
}

// Create new VM instance with bytecode using window dimensions
function createVM(bytecode) {
    if (!TokaVM) {
        setStatus('WASM module not loaded!', true);
        return null;
    }

    try {
        const width = window.innerWidth;
        const height = window.innerHeight;
        log(`Creating VM with ${bytecode.length} bytes of bytecode at ${width}x${height}`, 'info');
        const vm = new TokaVM(bytecode, width, height);
        log('VM created successfully', 'info');
        setStatus('VM created successfully');
        return vm;
    } catch (err) {
        setStatus(`VM creation failed: ${err.message}`, true);
        log(`VM creation error: ${err.stack}`, 'error');
        return null;
    }
}

// Render VM canvas to HTML canvas
function render() {
    if (!currentVM) {
        log('render() called but currentVM is null', 'error');
        return;
    }

    try {
        // Get RGBA bytes from VM and render to canvas
        const rgba = currentVM.get_canvas_rgba();
        const width = currentVM.width();
        const height = currentVM.height();

        const imageData = new ImageData(
            new Uint8ClampedArray(rgba),
            width,
            height
        );

        ctx.putImageData(imageData, 0, 0);
    } catch (err) {
        setStatus(`Render error: ${err.message}`, true);
        log(`Render error: ${err.message}`, 'error');
    }
}

// Fetch-at-render: drain the resource keys the VM couldn't resolve (e.g. avatar keys behind
// draw_image), fetch each over the VSF wire, hand the bytes back, then re-run + repaint so they
// blit. The VM builds/parses the VSF; this loop just shuttles opaque bytes. Bounded rounds guard
// against a capsule that keeps surfacing new keys. Fire-and-forget — it repaints itself.
async function resolveResources() {
    if (!currentVM) return;
    for (let round = 0; round < 4; round++) {
        const pending = currentVM.take_pending_requests();
        if (!pending || pending.length === 0) return;
        await Promise.all(pending.map(async (key) => {
            try {
                const req = currentVM.build_resource_request(key);
                const resp = await fetch('/resource_get', { method: 'POST', body: req });
                if (!resp.ok) return;
                const bytes = new Uint8Array(await resp.arrayBuffer());
                currentVM.provide_resource(key, bytes);
            } catch (err) {
                log(`resource fetch failed for ${key}: ${err.message}`, 'error');
            }
        }));
        // Re-run + repaint so the newly-provided resources decode and blit in place.
        currentVM.reset();
        while (currentVM.run(256)) {}
        render();
    }
}

// Setup canvas
function setupCanvas() {
    canvas = document.getElementById('canvas');
    ctx = canvas.getContext('2d', { colorSpace: 'srgb', alpha: false });
    log(`Canvas colour space: ${ctx.getContextAttributes?.()?.colorSpace ?? 'unknown'}`, 'info');

    // Set canvas size to window size
    function resizeCanvas() {
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;

        // Don't clear to black - let old content stretch until re-render completes
        // This prevents black flash during resize
    }

    // Resize handler — resize canvas + rerun, preserving widget state (text inputs, focus)
    function handleResize() {
        const newWidth = window.innerWidth;
        const newHeight = window.innerHeight;

        if (currentBytecode && currentVM) {
            canvas.width = newWidth;
            canvas.height = newHeight;
            stopBlinkey();
            currentVM.resize(newWidth, newHeight);
            currentVM.rerun(currentBytecode);
            currentVM.drain_events();
            render();
            if (currentVM.focused_widget() >= 0) startBlinkey();
        } else if (currentBytecode) {
            canvas.width = newWidth;
            canvas.height = newHeight;
            reactiveRender();
        }
    }

    resizeCanvas();
    window.addEventListener('resize', handleResize);

    // Colour picker — samples canvas pixel under cursor, updates when console is open
    const swatch = document.getElementById('colourSwatch');
    const hex = document.getElementById('colourHex');
    const coords = document.getElementById('colourCoords');

    canvas.addEventListener('mousemove', (e) => {
        const consoleEl = document.getElementById('console');
        if (!consoleEl.classList.contains('visible')) return;

        const [r, g, b, a] = ctx.getImageData(e.offsetX, e.offsetY, 1, 1).data;
        const hexStr = [r, g, b, a].map(v => v.toString(16).padStart(2, '0').toUpperCase()).join(' ');
        swatch.style.background = `rgba(${r},${g},${b},${a / 255})`;
        hex.textContent = hexStr;
        coords.textContent = `x:${e.offsetX} y:${e.offsetY}`;
    });
}

// Setup scroll tracking for reactive scenes
function setupScrollTracking() {
    function handleWheel(e) {
        if (!currentVM || !currentBytecode) return;
        e.preventDefault();
        const delta = Math.round(e.deltaY);
        if (delta === 0) return;
        // Shift pixels + set clip to exposed strip
        currentVM.scroll_by(delta);
        // Show shifted buffer immediately
        render();
        // Rerun draws only into the clipped strip
        currentVM.rerun(currentBytecode);
        currentVM.clear_clip();
        render();
    }

    canvas.addEventListener('wheel', handleWheel, { passive: false });
    log('Scroll tracking enabled', 'info');
}

// Handle resolution via memory-hard proof-of-work
// plaintext → blake3 → 17-round proof → base64url filename → fetch capsule
async function resolveHandle(handleName) {
    const normalized = handleName.toLowerCase().trim();

    try {
        log(`Resolving handle: "${handleName}"`, 'info');

        // Run memory-hard proof to get deterministic filename
        log('Computing handle proof (this takes a few seconds)...', 'info');
        const filename = wasmModule.resolve_handle(normalized);
        currentCapsuleAddress = filename.replace('.vsf', '');
        log(`Proof complete → /${filename}`, 'info');

        const response = await fetch(`/${filename}?v=${Date.now()}`);
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }
        const arrayBuffer = await response.arrayBuffer();
        const capsuleData = new Uint8Array(arrayBuffer);
        log(`Loaded ${capsuleData.length} bytes from ${filename}`, 'info');

        // Show VSF inspector output (vsfinfo style) in app console
        try {
            const inspectorOutput = wasmModule.inspect_capsule(capsuleData);
            // Log as a single pre-formatted block
            const consoleEl = document.getElementById('console');
            const pre = document.createElement('pre');
            pre.className = 'console-line console-info';
            pre.style.cssText = 'font-family: monospace; white-space: pre; margin: 5px 0; font-size: 10px;';
            pre.textContent = inspectorOutput;
            consoleEl.appendChild(pre);
            consoleEl.scrollTop = consoleEl.scrollHeight;
        } catch (err) {
            log(`Inspector failed: ${err}`, 'error');
        }

        // Use WASM load_capsule to parse VSF and extract executable bytecode
        const bytecode = wasmModule.load_capsule(capsuleData);
        log(`Extracted ${bytecode.length} bytes of executable bytecode`, 'info');

        return bytecode;
    } catch (err) {
        log(`Failed to load capsule: ${err}`, 'error');
        console.error('Capsule load error:', err);
        return null;
    }
}

let currentBytecode = null;  // Store bytecode for reactive rendering
let currentCapsuleAddress = null;  // base64url filename = action POST endpoint

// Load a capsule directly by URL (no handle resolution)
async function loadDirect(url) {
    try {
        log(`Fetching capsule: ${url}`, 'info');
        const response = await fetch(url);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const capsuleData = new Uint8Array(await response.arrayBuffer());
        log(`Fetched ${capsuleData.length} bytes`, 'info');

        const bytecode = wasmModule.load_capsule(capsuleData);
        log(`Extracted ${bytecode.length} bytes of bytecode`, 'info');

        // Extract capsule address from URL path
        const path = new URL(url, location.origin).pathname;
        currentCapsuleAddress = path.split('/').pop().split('?')[0].replace('.vsf', '');

        currentBytecode = bytecode;
        reactiveRender();
    } catch (err) {
        log(`Failed to load capsule: ${err}`, 'error');
    }
}
window.loadDirect = loadDirect;

async function loadAndRenderCapsule(handleName) {
    const bytecode = await resolveHandle(handleName);
    if (!bytecode) return;

    currentBytecode = bytecode;  // Save for resize
    reactiveRender();

    // Hide status bar once capsule is displayed
    const statusBar = document.querySelector('.status-bar');
    if (statusBar) statusBar.style.display = 'none';
}

// Reactive rendering - re-executes program with current viewport dimensions
// Creates a new VM (used for initial load and resize — loses widget state)
function reactiveRender() {
    if (!currentBytecode) return;

    // Preserve pipeline across re-creation
    const prevPipeline = currentVM ? currentVM.pipeline_name() : 'fast';

    log(`Creating VM with ${currentBytecode.length} bytes of bytecode...`, 'info');

    currentVM = createVM(currentBytecode);
    if (currentVM) {
        if (prevPipeline !== currentVM.pipeline_name()) {
            currentVM.set_pipeline(prevPipeline);
        }
        const label = document.getElementById('pipelineLabel');
        if (label) label.textContent = `pipeline: ${currentVM.pipeline_name()}`;
        try {
            log('Running VM...', 'info');
            while (currentVM.run(256)) {}  // Run to completion

            // Get and display execution trace
            const trace = currentVM.get_trace();
            if (trace.length > 0) {
                log(`Executed ${trace.length} opcodes: ${trace.join(' → ')}`, 'info');
            }

            currentVM.drain_events();
            render();
            resolveResources(); // async: fetch any draw_image resources (avatars), then repaint
            log('Rendered successfully', 'info');
        } catch (err) {
            log(`VM execution error: ${err}`, 'error');
            log(`Error type: ${typeof err}`, 'error');
            log(`Error message: ${err.message || 'none'}`, 'error');
            log(`Error toString: ${err.toString()}`, 'error');
            if (err.stack) {
                log(`Stack: ${err.stack}`, 'error');
            }
        }
    }
}

// Full interactive re-render — re-executes entire bytecode (preserves widget state)
// Used for mouse clicks (buttons need post-table stack processing for actions)
function interactiveRerender() {
    if (!currentVM || !currentBytecode) return;
    try {
        stopBlinkey();
        currentVM.rerun(currentBytecode);
        currentVM.drain_events();
        render();
        processActions();
        if (currentVM.focused_widget() >= 0) startBlinkey();
    } catch (err) {
        log(`Interactive render error: ${err}`, 'error');
    }
}

// Differential widget rerender — only redraws widget regions, skips full VM execution
// Used for keystrokes and cursor updates (order of magnitude faster)
function widgetRerender() {
    if (!currentVM) return;
    try {
        if (currentVM.has_widget_snapshots()) {
            stopBlinkey();
            currentVM.rerun_widgets();
            render();
            if (currentVM.focused_widget() >= 0) startBlinkey();
        } else {
            // Fallback to full rerender if no snapshots (first frame etc.)
            interactiveRerender();
        }
    } catch (err) {
        log(`Widget render error: ${err}`, 'error');
    }
}

// Blinkey cursor animation — flips wave every ~200ms (randomized for organic feel)
let blinkeyTimer = null;
function startBlinkey() {
    stopBlinkey();
    function tick() {
        if (!currentVM) return;
        try {
            currentVM.flip_blinkey();
            render();
        } catch (e) { /* ignore */ }
        // Random interval 100-350ms for organic feel
        blinkeyTimer = setTimeout(tick, 100 + Math.random() * 250);
    }
    blinkeyTimer = setTimeout(tick, 200);
}
function stopBlinkey() {
    if (blinkeyTimer) {
        clearTimeout(blinkeyTimer);
        blinkeyTimer = null;
    }
}

// Process triggered actions from button clicks
function processActions() {
    if (!currentVM || !currentCapsuleAddress) return;
    const endpoint = `/${currentCapsuleAddress}`;
    while (true) {
        const bytes = currentVM.drain_action();
        if (bytes.length === 0) break;
        const nullIdx = bytes.indexOf(0);
        const section = nullIdx >= 0
            ? new TextDecoder().decode(bytes.slice(0, nullIdx))
            : new TextDecoder().decode(bytes);
        log(`Action: ${section} → ${endpoint}`, 'info');
        fetch(endpoint, {
            method: 'POST',
            body: bytes,
        }).then(resp => {
            if (resp.ok) {
                log(`Action OK: ${section}`, 'info');
            } else {
                log(`Action failed: ${resp.status} ${section}`, 'error');
            }
        }).catch(err => log(`Action error: ${err}`, 'error'));
    }
}

// Convert page coordinates to RU (render units, center-origin)
function pageToRU(pageX, pageY) {
    const w = currentVM.width();
    const h = currentVM.height();
    const span = currentVM.get_span();
    const ru = currentVM.get_ru();
    // Pixel offset from center, then convert to RU
    const x = (pageX - w / 2) / (span * ru);
    const y = (pageY - h / 2) / (span * ru);
    return [x, y];
}

// Setup interactive event handling (mouse + keyboard → VM)
function setupInteraction() {
    // ── Mouse ──────────────────────────────────────────

    canvas.addEventListener('mousemove', (e) => {
        if (!currentVM) return;
        const [rx, ry] = pageToRU(e.offsetX, e.offsetY);
        currentVM.set_mouse(rx, ry);

        // Update CSS cursor based on hit regions
        const cursor = currentVM.cursor_at(rx, ry);
        canvas.style.cursor = cursor;
    });

    canvas.addEventListener('mousedown', (e) => {
        if (!currentVM) return;
        const [rx, ry] = pageToRU(e.offsetX, e.offsetY);
        currentVM.push_mouse_down(rx, ry);
        interactiveRerender();

        // Update cursor after click (focus may have changed)
        canvas.style.cursor = currentVM.cursor_at(rx, ry);
    });

    canvas.addEventListener('mouseup', (e) => {
        if (!currentVM) return;
        const [rx, ry] = pageToRU(e.offsetX, e.offsetY);
        currentVM.push_mouse_up(rx, ry);
        interactiveRerender();
    });

    // ── Keyboard ───────────────────────────────────────

    // Only forward keyboard events when a toka widget has focus
    // (not when typing in HTML inputs like the handle field or console)
    window.addEventListener('keydown', (e) => {
        if (!currentVM) return;
        // Don't intercept if an HTML input/textarea is focused
        const active = document.activeElement;
        if (active && (active.tagName === 'INPUT' || active.tagName === 'TEXTAREA')) return;
        // Only forward if a toka widget has focus
        if (currentVM.focused_widget() < 0) return;

        // Let Ctrl/Cmd shortcuts pass through to browser (paste handled by paste event)
        if (e.ctrlKey || e.metaKey) return;

        // Special keys → push_key_down
        const specialKeys = ['Backspace', 'Delete', 'ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End', 'Tab', 'Escape'];
        if (e.key === 'Enter') {
            e.preventDefault();
            currentVM.push_key_press('\n');
            widgetRerender();
            requestAnimationFrame(() => interactiveRerender());
        } else if (specialKeys.includes(e.key)) {
            e.preventDefault();
            currentVM.push_key_down(e.key);
            widgetRerender();
        }
    });

    window.addEventListener('keypress', (e) => {
        if (!currentVM) return;
        const active = document.activeElement;
        if (active && (active.tagName === 'INPUT' || active.tagName === 'TEXTAREA')) return;
        if (currentVM.focused_widget() < 0) return;

        // Printable characters → push_key_press
        if (e.key.length === 1) {
            e.preventDefault();
            currentVM.push_key_press(e.key);
            widgetRerender();
        }
    });

    // Paste — covers Ctrl+V and right-click paste
    window.addEventListener('paste', (e) => {
        if (!currentVM) return;
        const active = document.activeElement;
        if (active && (active.tagName === 'INPUT' || active.tagName === 'TEXTAREA')) return;
        if (currentVM.focused_widget() < 0) return;

        e.preventDefault();
        const text = (e.clipboardData || window.clipboardData).getData('text');
        if (text) {
            currentVM.push_key_press(text);
            widgetRerender();
        }
    });

    log('Interactive event handling enabled', 'info');
}

// Setup handle input (both modal overlay and console toolbar)
function setupHandleInput() {
    const handleInput = document.getElementById('handleInput');
    const handleField = document.getElementById('handleField');

    // Modal overlay input (shown on first load)
    if (handleField) {
        handleField.addEventListener('keypress', async (e) => {
            if (e.key === 'Enter') {
                const handle = handleField.value.trim();
                if (handle) {
                    log(`Loading: ${handle}`, 'info');
                    if (handleInput) handleInput.classList.add('hidden');
                    await loadAndRenderCapsule(handle);
                }
            }
        });
        handleField.focus();
    }

    // Console toolbar handle input (always available)
    const consoleHandle = document.getElementById('consoleHandle');
    const loadConsoleHandle = async () => {
        if (!consoleHandle) return;
        const handle = consoleHandle.value.trim();
        if (handle) {
            log(`Loading: ${handle}`, 'info');
            if (handleInput) handleInput.classList.add('hidden');
            consoleHandle.value = '';
            await loadAndRenderCapsule(handle);
        }
    };
    if (consoleHandle) {
        consoleHandle.addEventListener('keypress', async (e) => {
            if (e.key === 'Enter') await loadConsoleHandle();
        });
    }
    const consoleHandleGo = document.getElementById('consoleHandleGo');
    if (consoleHandleGo) {
        consoleHandleGo.addEventListener('click', loadConsoleHandle);
    }

    log('Handle input ready', 'info');
}

// Toggle between fast and quality pipeline
function togglePipeline() {
    if (!currentVM || !currentBytecode) return;
    const current = currentVM.pipeline_name();
    const next = current === 'fast' ? 'quality' : 'fast';
    try {
        currentVM.set_pipeline(next);
        currentVM.reset();
        while (currentVM.run(256)) {}
        const label = document.getElementById('pipelineLabel');
        if (label) label.textContent = `pipeline: ${next}`;
        render();
        log(`Switched to ${next} pipeline`, 'info');
    } catch (err) {
        log(`Pipeline switch error: ${err}`, 'error');
    }
}
window.togglePipeline = togglePipeline;

// Load capsule from raw bytes (used by WebSocket push)
function loadCapsuleBytes(capsuleData) {
    try {
        const bytecode = wasmModule.load_capsule(capsuleData);
        log(`Extracted ${bytecode.length} bytes of executable bytecode`, 'info');
        currentBytecode = bytecode;
        reactiveRender();
    } catch (err) {
        log(`Failed to load pushed capsule: ${err}`, 'error');
    }
}

// WebSocket connection for live capsule updates
function setupWebSocket() {
    const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${proto}//${location.host}/ws`;
    log(`WebSocket connecting: ${wsUrl}`, 'info');

    const ws = new WebSocket(wsUrl);
    ws.binaryType = 'arraybuffer';

    ws.onopen = () => log('WebSocket connected', 'info');

    ws.onmessage = (event) => {
        if (event.data instanceof ArrayBuffer) {
            const bytes = new Uint8Array(event.data);
            log(`WebSocket: received ${bytes.length} bytes`, 'info');
            loadCapsuleBytes(bytes);
        }
    };

    ws.onclose = () => {
        log('WebSocket closed, reconnecting in 3s...', 'info');
        setTimeout(setupWebSocket, 3000);
    };

    ws.onerror = (err) => {
        log(`WebSocket error`, 'error');
    };
}

// Main entry point
async function main() {
    log('Application starting...', 'info');
    setupCanvas();
    setupScrollTracking();
    setupInteraction();
    await init();
    setupHandleInput();
    setupWebSocket();
    log('Toka VM ready - enter a handle name', 'info');
}

main().catch(err => {
    log(`Fatal error: ${err.message}`, 'error');
    log(`Stack: ${err.stack}`, 'error');
    setStatus(`Fatal error: ${err.message}`, true);
});

// Loom vt capsule test functions (call from browser console)
window.testRedBox = function() {
    const bytecode = [0x7b, 0x70, 0x73, 0x7d, 0x76, 0x74, 0x33, 0x98, 0x62, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x40, 0x00, 0x40, 0x00, 0x00, 0x01, 0x40, 0x00, 0x00, 0x00, 0x00, 0x01, 0x7b, 0x72, 0x6c, 0x7d, 0x7b, 0x68, 0x6c, 0x7d];
    currentVM = createVM(new Uint8Array(bytecode));
    if (currentVM) {
        currentVM.run();
        render();
        log('Red box rendered (vt capsule)', 'info');
    }
};

window.testGreenBox = function() {
    const bytecode = [0x7b, 0x70, 0x73, 0x7d, 0x76, 0x74, 0x33, 0x98, 0x62, 0x40, 0x00, 0x40, 0x00, 0xff, 0xff, 0x40, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x01, 0x7b, 0x72, 0x6c, 0x7d, 0x7b, 0x68, 0x6c, 0x7d];
    currentVM = createVM(new Uint8Array(bytecode));
    if (currentVM) {
        currentVM.run();
        render();
        log('Green box rendered (vt capsule)', 'info');
    }
};

window.testBlueCircle = function() {
    const bytecode = [0x7b, 0x70, 0x73, 0x7d, 0x76, 0x74, 0x33, 0x88, 0x63, 0x40, 0x00, 0x40, 0x00, 0x00, 0x00, 0x4c, 0xcc, 0xff, 0xff, 0x00, 0x00, 0x40, 0x00, 0x00, 0x01, 0x7b, 0x72, 0x6c, 0x7d, 0x7b, 0x68, 0x6c, 0x7d];
    currentVM = createVM(new Uint8Array(bytecode));
    if (currentVM) {
        currentVM.run();
        render();
        log('Blue circle rendered (vt capsule)', 'info');
    }
};
