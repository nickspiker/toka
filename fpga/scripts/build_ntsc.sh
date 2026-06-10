#!/bin/bash
# Build NTSC CRT self-test for Colorlight 5A-75B (ECP5-25F).
#
# Usage:
#   ./build_ntsc.sh [FREQ_MHZ] [--program]
#
# FREQ_MHZ=25 uses raw clock (no PLL). Any other frequency uses PLL.
#
# Examples:
#   ./build_ntsc.sh                 # build at 25 MHz (no PLL)
#   ./build_ntsc.sh 50              # build at 50 MHz (PLL)
#   ./build_ntsc.sh 50 --program    # build + program
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FPGA_DIR="$SCRIPT_DIR/.."
RTL="$FPGA_DIR/rtl"
LPF="$FPGA_DIR/constraints/colorlight_5a75b_v8.lpf"
BUILD="$FPGA_DIR/build"

FREQ="${1:-25}"
PROGRAM="${2:-}"

mkdir -p "$BUILD"
cd "$FPGA_DIR"

# -------------------------------------------------------------------------
# Compute PLL parameters (skip if 25 MHz)
# -------------------------------------------------------------------------
PLL_DEFINES=""
if [ "$FREQ" != "25" ]; then
    PLL_OUT=$(ecppll -i 25 -o "$FREQ" -f /dev/null 2>&1)
    CLKI=$(echo "$PLL_OUT"  | awk '/Refclk divisor:/  {print $3}')
    CLKFB=$(echo "$PLL_OUT" | awk '/Feedback divisor:/ {print $3}')
    CLKOP=$(echo "$PLL_OUT" | awk '/clkout0 divisor:/  {print $3}')
    ACTUAL_FREQ=$(echo "$PLL_OUT" | awk '/clkout0 frequency:/ {print $3}')
    FVCO=$(echo "$PLL_OUT"  | awk '/VCO frequency:/    {print $3}')
    if [ -z "$CLKI" ] || [ -z "$CLKFB" ] || [ -z "$CLKOP" ]; then
        echo "ERROR: ecppll failed for ${FREQ} MHz"
        echo "$PLL_OUT"
        exit 1
    fi
    CPHASE=$(( (CLKOP - 1) / 2 ))
    PLL_DEFINES="-DPLL_CLKI_DIV=$CLKI -DPLL_CLKFB_DIV=$CLKFB -DPLL_CLKOP_DIV=$CLKOP -DPLL_CLKOP_CPHASE=$CPHASE"
    echo "================================================================"
    echo " NTSC self-test @ ${ACTUAL_FREQ} MHz"
    echo " PLL: CLKI=$CLKI CLKFB=$CLKFB CLKOP=$CLKOP (VCO=${FVCO} MHz)"
    echo "================================================================"
else
    echo "================================================================"
    echo " NTSC self-test @ 25 MHz (no PLL)"
    echo "================================================================"
fi

# -------------------------------------------------------------------------
# Collect all RTL files
# -------------------------------------------------------------------------
RTL_FILES=""
for f in "$RTL"/spirix_*.v; do
    RTL_FILES="$RTL_FILES read_verilog $f;"
done

# -------------------------------------------------------------------------
# Synthesize
# -------------------------------------------------------------------------
echo ""
echo "--- Synthesize ---"
yosys -p "
    $RTL_FILES
    read_verilog $RTL/blake3.v
    read_verilog $RTL/ntsc_framebuf.v
    read_verilog $PLL_DEFINES $RTL/top_ntsc.v
    synth_ecp5 -top top_ntsc -json $BUILD/ntsc.json
    stat
" > "$BUILD/ntsc_yosys.log" 2>&1

grep -E '^\s+[0-9]+ +(LUT4|TRELLIS_FF|MULT18X18D|CCU2C|EHXPLLL|DP16KD)$' "$BUILD/ntsc_yosys.log" || true
if grep -q 'ERROR' "$BUILD/ntsc_yosys.log"; then
    echo "YOSYS ERROR — see $BUILD/ntsc_yosys.log"
    exit 1
fi

# -------------------------------------------------------------------------
# Place and route
# -------------------------------------------------------------------------
echo ""
echo "--- Place & Route ---"
SEED="${SEED:-1}"
nextpnr-ecp5 --25k --package CABGA256 --speed 6 --seed "$SEED" \
    --json "$BUILD/ntsc.json" \
    --lpf "$LPF" \
    --textcfg "$BUILD/ntsc.config" \
    > "$BUILD/ntsc_pnr.log" 2>&1 || true

grep -E '(Max frequency|logic,)' "$BUILD/ntsc_pnr.log" || true
if grep -q 'ERROR' "$BUILD/ntsc_pnr.log"; then
    echo "PNR ERROR — see $BUILD/ntsc_pnr.log"
    exit 1
fi

# -------------------------------------------------------------------------
# Pack bitstream + SVF
# -------------------------------------------------------------------------
echo ""
echo "--- Pack ---"
ecppack --svf "$BUILD/ntsc.svf" "$BUILD/ntsc.config" "$BUILD/ntsc.bit"
echo "Bitstream: $BUILD/ntsc.bit ($(stat -c%s "$BUILD/ntsc.bit") bytes)"

# -------------------------------------------------------------------------
# Program
# -------------------------------------------------------------------------
if [ "$PROGRAM" = "--program" ]; then
    echo ""
    echo "--- Program (SVF/SRAM) ---"
    openFPGALoader -c ft232 "$BUILD/ntsc.svf"
    echo "Done. Check your CRT!"
else
    echo ""
    echo "Done. To program:"
    echo "  openFPGALoader -c ft232 $BUILD/ntsc.svf"
fi
