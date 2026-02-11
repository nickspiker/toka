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

        // Import the WASM module with cache-busting timestamp
        const cacheBust = Date.now();
        log(`Importing module from ./pkg/toka.js?v=${cacheBust}`, 'info');
        const module = await import(`./pkg/toka.js?v=${cacheBust}`);
        log('Module imported successfully', 'info');

        // Initialize WASM with cache-busted binary URL
        const wasmUrl = `./pkg/toka_bg.wasm?v=${cacheBust}`;
        log(`Calling module.default() with ${wasmUrl}...`, 'info');
        await module.default(wasmUrl);
        log('WASM initialized successfully', 'info');

        TokaVM = module.TokaVM;
        wasmModule = module;

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

// Setup canvas
function setupCanvas() {
    canvas = document.getElementById('canvas');
    ctx = canvas.getContext('2d');

    // Set canvas size to window size
    function resizeCanvas() {
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;

        // Don't clear to black - let old content stretch until re-render completes
        // This prevents black flash during resize
    }

    // Resize handler - re-render scene graph immediately
    function handleResize() {
        const newWidth = window.innerWidth;
        const newHeight = window.innerHeight;

        if (currentVM && currentVM.has_scene()) {
            // Efficient path: re-render scene graph at new resolution
            try {
                // 1. Render to new size in WASM (doesn't touch canvas)
                currentVM.resize(newWidth, newHeight);

                // 2. Update canvas size (clears it)
                canvas.width = newWidth;
                canvas.height = newHeight;

                // 3. Immediately display new pixels (no black flash)
                render();
            } catch (err) {
                log(`Resize error: ${err}`, 'error');
            }
        } else if (currentBytecode) {
            // Fallback: no scene graph, re-execute bytecode (legacy mode)
            canvas.width = newWidth;
            canvas.height = newHeight;
            reactiveRender();
        }
    }

    resizeCanvas();
    window.addEventListener('resize', handleResize);
}

// Handle resolution (temporary local mapping, will become FGTW later)
// Automatically maps: "box" → "capsules/box.vsf"
async function resolveHandle(handleName) {
    const normalized = handleName.toLowerCase().trim();
    const filename = `capsules/${normalized}.vsf`;

    try {
        log(`Resolving handle: "${handleName}"`, 'info');
        const response = await fetch(filename);
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

async function loadAndRenderCapsule(handleName) {
    const bytecode = await resolveHandle(handleName);
    if (!bytecode) return;

    currentBytecode = bytecode;  // Save for resize
    reactiveRender();
}

// Reactive rendering - re-executes program with current viewport dimensions
function reactiveRender() {
    if (!currentBytecode) return;

    log(`Creating VM with ${currentBytecode.length} bytes of bytecode...`, 'info');

    currentVM = createVM(currentBytecode);
    if (currentVM) {
        try {
            log('Running VM...', 'info');
            const result = currentVM.run(1000);  // Execute up to 1000 instructions

            // Get and display execution trace
            const trace = currentVM.get_trace();
            if (trace.length > 0) {
                log(`Executed ${trace.length} opcodes: ${trace.join(' → ')}`, 'info');
            }

            render();
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

// Setup handle input
function setupHandleInput() {
    const handleInput = document.getElementById('handleInput');
    const handleField = document.getElementById('handleField');

    if (!handleField) {
        log('ERROR: handleField element not found!', 'error');
        return;
    }

    log('Setting up handle input listener...', 'info');

    handleField.addEventListener('keypress', async (e) => {
        if (e.key === 'Enter') {
            const handle = handleField.value.trim();
            if (handle) {
                log(`Loading: ${handle}`, 'info');
                handleInput.classList.add('hidden');
                await loadAndRenderCapsule(handle);
            }
        }
    });

    // Focus on load
    log('Focusing handle field...', 'info');
    handleField.focus();
    log('Handle input setup complete', 'info');
}

// Main entry point
async function main() {
    log('Application starting...', 'info');
    setupCanvas();
    await init();
    setupHandleInput();
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
