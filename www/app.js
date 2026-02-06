// Toka VM Browser Integration
// Loads WASM module and provides interactive example programs

// Immediate log to verify module loads
console.log('[APP.JS] Module loaded!');
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

    // Also log to browser console
    if (type === 'error') {
        console.error(message);
    } else {
        console.log(message);
    }
}

// Status display (just logs to console now)
function setStatus(message, isError = false) {
    log(message, isError ? 'error' : 'info');
}

// Initialize WASM module
async function init() {
    try {
        log('Starting WASM initialization...', 'info');
        setStatus('Loading WASM module...');

        // Import the WASM module
        log('Importing module from ./pkg/toka.js', 'info');
        const module = await import('./pkg/toka.js');
        log('Module imported successfully', 'info');

        log('Calling module.default() to initialize WASM...', 'info');
        await module.default();
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
        // Get RGBA bytes from VM
        log('Getting RGBA from VM...', 'info');
        const rgba = currentVM.get_canvas_rgba();
        log(`Got ${rgba.length} bytes of RGBA data`, 'info');

        // Sample first 20 pixels to see what colors we're actually rendering
        const samples = [];
        for (let i = 0; i < Math.min(20, rgba.length / 4); i++) {
            const idx = i * 4;
            samples.push(`[${rgba[idx]},${rgba[idx+1]},${rgba[idx+2]},${rgba[idx+3]}]`);
        }
        log(`First pixels: ${samples.join(' ')}`, 'info');

        const width = currentVM.width();
        const height = currentVM.height();
        log(`VM canvas size: ${width}x${height}`, 'info');

        // Create ImageData and render
        const imageData = new ImageData(
            new Uint8ClampedArray(rgba),
            width,
            height
        );

        log('Putting ImageData to canvas...', 'info');
        ctx.putImageData(imageData, 0, 0);
        log('ImageData rendered to canvas', 'info');
    } catch (err) {
        setStatus(`Render error: ${err.message}`, true);
        log(`Render error: ${err.message}`, 'error');
        console.error('Render error:', err);
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

        // Clear to black
        ctx.fillStyle = '#000';
        ctx.fillRect(0, 0, canvas.width, canvas.height);

        // Re-render if VM exists
        if (currentVM) {
            render();
        }
    }

    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);
}

// Handle resolution (temporary local mapping, will become FGTW later)
const handleMap = {
    'redbox': 'capsules/redbox.vsf',
    'greenbox': 'capsules/greenbox.vsf',
    'bluecircle': 'capsules/bluecircle.vsf',
};

async function resolveHandle(handleName) {
    const normalized = handleName.toLowerCase().trim();
    const filename = handleMap[normalized];

    if (!filename) {
        log(`Unknown handle: "${handleName}"`, 'error');
        log(`Available: ${Object.keys(handleMap).join(', ')}`, 'info');
        return null;
    }

    try {
        log(`Resolving handle: "${handleName}"`, 'info');
        const response = await fetch(filename);
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }
        const arrayBuffer = await response.arrayBuffer();
        const capsuleData = new Uint8Array(arrayBuffer);
        log(`Loaded ${capsuleData.length} bytes from ${filename}`, 'info');

        // Use WASM load_capsule to parse VSF and extract executable bytecode
        const bytecode = wasmModule.load_capsule(capsuleData);
        log(`Extracted ${bytecode.length} bytes of executable bytecode`, 'info');
        return bytecode;
    } catch (err) {
        log(`Failed to load capsule: ${err.message}`, 'error');
        return null;
    }
}

async function loadAndRenderCapsule(handleName) {
    const bytecode = await resolveHandle(handleName);
    if (!bytecode) return;

    log('Creating VM with executable bytecode...', 'info');

    // Debug: dump bytecode hex
    const hex = Array.from(bytecode).map(b => '0x' + b.toString(16).padStart(2, '0')).join(' ');
    log(`Bytecode (${bytecode.length} bytes): ${hex.substring(0, 200)}...`, 'info');

    currentVM = createVM(bytecode);
    if (currentVM) {
        try {
            log('Running VM...', 'info');
            const result = currentVM.run(1000);  // Execute up to 1000 instructions
            log(`VM run() returned: ${result}`, 'info');

            log('Calling render()...', 'info');
            render();
            log(`Rendered: "${handleName}"`, 'info');
        } catch (err) {
            log(`VM execution error: ${err}`, 'error');
            log(`Error type: ${typeof err}`, 'error');
            log(`Error message: ${err.message || 'none'}`, 'error');
            log(`Error toString: ${err.toString()}`, 'error');
            if (err.stack) {
                log(`Stack: ${err.stack}`, 'error');
            }
            console.error('Full error object:', err);
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
        log(`Key pressed: ${e.key}`, 'info');
        if (e.key === 'Enter') {
            const handle = handleField.value.trim();
            log(`Enter pressed, handle value: "${handle}"`, 'info');
            if (handle) {
                log(`Handle entered: "${handle}"`, 'info');
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
