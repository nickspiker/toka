// =============================================================================
// blake3.v  —  BLAKE3 Compression Function
// =============================================================================
// Translated to Verilog-2001 from:
//   github.com/6a-62/blake3_vhdl  (BSD-3-Clause, Copyright 6a-62)
// Translation copyright released under same BSD-3-Clause terms.
//
// Implements the BLAKE3 compression function ONLY.
// Tree structure, chunking, padding, and root finalization are
// the caller's responsibility.
//
// Port description:
//   i_chain    [255:0]  Chaining value: 8 x 32-bit words, word 0 in [31:0]
//   i_mblock   [511:0]  Message block: 16 x 32-bit words, word 0 in [31:0]
//   i_counter  [63:0]   Block counter (low 32 in [31:0])
//   i_numbytes [31:0]   Number of bytes in this block (0..64)
//   i_dflags   [31:0]   Domain separation flags:
//                         CHUNK_START        = 32'h01
//                         CHUNK_END          = 32'h02
//                         PARENT             = 32'h04
//                         ROOT               = 32'h08
//                         KEYED_HASH         = 32'h10
//                         DERIVE_KEY_CONTEXT = 32'h20
//                         DERIVE_KEY_MATERIAL= 32'h40
//   i_valid             Strobe: assert for one cycle to load inputs
//   o_hash    [511:0]   Compression output (512 bits = two 256-bit halves)
//   o_valid             High when o_hash is valid and core is idle
//
// Latency: 16 clock cycles from i_valid strobe to o_valid
//   1 (PREPARE) + 7 × (GCOL + GDIAG) + 1 (OUTPUT)
//
// Reset: asynchronous, active-low (i_reset = 0 resets)
//
// Notes:
//   - All arithmetic is 32-bit unsigned modular (wrapping)
//   - Message words and chaining value words are little-endian per BLAKE3 spec
//   - Only IV[0..3] are used inside the compression function; IV[4..7] are
//     used externally when seeding the first chunk's chaining value
// =============================================================================

`timescale 1ns / 1ps

module blake3 (
    input  wire         i_clk,
    input  wire         i_reset,      // Active-low asynchronous reset
    input  wire         i_ce,         // Clock enable (1=advance, 0=hold)

    // Inputs
    input  wire [255:0] i_chain,      // Input chaining value
    input  wire [511:0] i_mblock,     // Message block
    input  wire  [63:0] i_counter,    // Block counter
    input  wire  [31:0] i_numbytes,   // Number of input bytes
    input  wire  [31:0] i_dflags,     // Domain separation flags
    input  wire         i_valid,      // Strobe: inputs ready to sample

    // Outputs
    output reg  [511:0] o_hash,       // 512-bit compression output
    output reg          o_valid       // Output valid / core idle
);

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------
localparam [2:0]
    S_IDLE    = 3'd0,
    S_PREPARE = 3'd1,
    S_GCOL    = 3'd2,
    S_GDIAG   = 3'd3,
    S_OUTPUT  = 3'd4;

reg [2:0] r_state;

// ---------------------------------------------------------------------------
// BLAKE3 / BLAKE2s IV constants
// (first 32 fractional bits of √2, √3, √5, √7, √11, √13, √17, √19)
// Only IV0..IV3 are loaded into v[8..11]; IV4..7 listed for completeness.
// ---------------------------------------------------------------------------
localparam [31:0]
    IV0 = 32'h6a09e667,
    IV1 = 32'hbb67ae85,
    IV2 = 32'h3c6ef372,
    IV3 = 32'ha54ff53a,
    IV4 = 32'h510e527f,   // not used inside compression function
    IV5 = 32'h9b05688c,   // not used inside compression function
    IV6 = 32'h1f83d9ab,   // not used inside compression function
    IV7 = 32'h5be0cd19;   // not used inside compression function

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------
reg [31:0]  r_v [0:15];   // 16-word compression state
reg [511:0] r_mblock;     // Current round's message words (permuted each round)
reg  [2:0]  r_round;      // Round counter: 0..6  (7 rounds total)

// ---------------------------------------------------------------------------
// G quarter-round function (pure combinatorial)
//
// Implements:
//   a = a + b + mx
//   d = (d ^ a) >>> 16
//   c = c + d
//   b = (b ^ c) >>> 12
//   a = a + b + my
//   d = (d ^ a) >>> 8
//   c = c + d
//   b = (b ^ c) >>> 7
//
// Returns {a_out[127:96], b_out[95:64], c_out[63:32], d_out[31:0]}
// ---------------------------------------------------------------------------
function [127:0] G;
    input [31:0] a, b, c, d, mx, my;
    reg   [31:0] ta, tb, tc, td, t;
    begin
        ta = a  + b  + mx;
        t  = d  ^ ta;  td = {t[15:0], t[31:16]};   // ROR 16
        tc = c  + td;
        t  = b  ^ tc;  tb = {t[11:0], t[31:12]};   // ROR 12
        ta = ta + tb + my;
        t  = td ^ ta;  td = {t[7:0],  t[31:8]};    // ROR  8
        tc = tc + td;
        t  = tb ^ tc;  tb = {t[6:0],  t[31:7]};    // ROR  7
        G  = {ta, tb, tc, td};
    end
endfunction

// ---------------------------------------------------------------------------
// Combinatorial G instances for column round
// Indices: G(a, b, c, d, m_even, m_odd)
//   Col 0: v[0], v[4],  v[8],  v[12], m[0],  m[1]
//   Col 1: v[1], v[5],  v[9],  v[13], m[2],  m[3]
//   Col 2: v[2], v[6],  v[10], v[14], m[4],  m[5]
//   Col 3: v[3], v[7],  v[11], v[15], m[6],  m[7]
// ---------------------------------------------------------------------------
wire [127:0] g_col0, g_col1, g_col2, g_col3;

assign g_col0 = G(r_v[0], r_v[4],  r_v[8],  r_v[12],
                  r_mblock[0*32 +: 32], r_mblock[1*32 +: 32]);
assign g_col1 = G(r_v[1], r_v[5],  r_v[9],  r_v[13],
                  r_mblock[2*32 +: 32], r_mblock[3*32 +: 32]);
assign g_col2 = G(r_v[2], r_v[6],  r_v[10], r_v[14],
                  r_mblock[4*32 +: 32], r_mblock[5*32 +: 32]);
assign g_col3 = G(r_v[3], r_v[7],  r_v[11], r_v[15],
                  r_mblock[6*32 +: 32], r_mblock[7*32 +: 32]);

// ---------------------------------------------------------------------------
// Combinatorial G instances for diagonal round
//   Diag 0: v[0], v[5],  v[10], v[15], m[8],  m[9]
//   Diag 1: v[1], v[6],  v[11], v[12], m[10], m[11]
//   Diag 2: v[2], v[7],  v[8],  v[13], m[12], m[13]
//   Diag 3: v[3], v[4],  v[9],  v[14], m[14], m[15]
// ---------------------------------------------------------------------------
wire [127:0] g_diag0, g_diag1, g_diag2, g_diag3;

assign g_diag0 = G(r_v[0], r_v[5],  r_v[10], r_v[15],
                   r_mblock[8*32  +: 32], r_mblock[9*32  +: 32]);
assign g_diag1 = G(r_v[1], r_v[6],  r_v[11], r_v[12],
                   r_mblock[10*32 +: 32], r_mblock[11*32 +: 32]);
assign g_diag2 = G(r_v[2], r_v[7],  r_v[8],  r_v[13],
                   r_mblock[12*32 +: 32], r_mblock[13*32 +: 32]);
assign g_diag3 = G(r_v[3], r_v[4],  r_v[9],  r_v[14],
                   r_mblock[14*32 +: 32], r_mblock[15*32 +: 32]);

// ---------------------------------------------------------------------------
// State machine — sequential logic
// ---------------------------------------------------------------------------
integer ii;

always @(posedge i_clk or negedge i_reset) begin
    if (!i_reset) begin
        r_state <= S_IDLE;
        o_valid <= 1'b1;
        o_hash  <= 512'b0;
        r_round <= 3'd0;
        // r_v and r_mblock reset not required (never read before PREPARE)
    end else if (i_ce) begin
        case (r_state)

        // ------------------------------------------------------------------
        S_IDLE: begin
            if (i_valid) begin
                o_valid <= 1'b0;
                r_state <= S_PREPARE;
            end
        end

        // ------------------------------------------------------------------
        S_PREPARE: begin
            // v[0..7]  ← chaining value
            for (ii = 0; ii < 8; ii = ii + 1)
                r_v[ii] <= i_chain[ii*32 +: 32];
            // v[8..11] ← IV[0..3]
            r_v[8]  <= IV0;
            r_v[9]  <= IV1;
            r_v[10] <= IV2;
            r_v[11] <= IV3;
            // v[12..15] ← counter, length, flags
            r_v[12] <= i_counter[31:0];
            r_v[13] <= i_counter[63:32];
            r_v[14] <= i_numbytes;
            r_v[15] <= i_dflags;
            // Latch message block
            r_mblock <= i_mblock;
            r_round  <= 3'd0;
            r_state  <= S_GCOL;
        end

        // ------------------------------------------------------------------
        // Column round: apply G to all four columns in parallel.
        // g_col0..3 are already computed combinatorially from current r_v.
        // Capture results into r_v on this clock edge.
        // G result packing: [127:96]=a_out [95:64]=b_out [63:32]=c_out [31:0]=d_out
        // ------------------------------------------------------------------
        S_GCOL: begin
            // Column 0: (v[0], v[4], v[8], v[12])
            r_v[0]  <= g_col0[127:96];
            r_v[4]  <= g_col0[95:64];
            r_v[8]  <= g_col0[63:32];
            r_v[12] <= g_col0[31:0];

            // Column 1: (v[1], v[5], v[9], v[13])
            r_v[1]  <= g_col1[127:96];
            r_v[5]  <= g_col1[95:64];
            r_v[9]  <= g_col1[63:32];
            r_v[13] <= g_col1[31:0];

            // Column 2: (v[2], v[6], v[10], v[14])
            r_v[2]  <= g_col2[127:96];
            r_v[6]  <= g_col2[95:64];
            r_v[10] <= g_col2[63:32];
            r_v[14] <= g_col2[31:0];

            // Column 3: (v[3], v[7], v[11], v[15])
            r_v[3]  <= g_col3[127:96];
            r_v[7]  <= g_col3[95:64];
            r_v[11] <= g_col3[63:32];
            r_v[15] <= g_col3[31:0];

            r_state <= S_GDIAG;
        end

        // ------------------------------------------------------------------
        // Diagonal round: apply G to all four diagonals in parallel.
        // g_diag0..3 use the post-GCOL r_v (already registered from S_GCOL).
        // After capturing results, either apply the message permutation σ
        // and loop back to GCOL, or proceed to OUTPUT on round 6.
        //
        // Message permutation σ (applied before rounds 1..6):
        //   σ = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8]
        //   new_word[i] = old_word[σ[i]]
        //
        // Non-blocking assignments evaluate all RHS before writing LHS,
        // so the permutation is atomic — no temporary buffer needed.
        // ------------------------------------------------------------------
        S_GDIAG: begin
            // Diagonal 0: (v[0], v[5], v[10], v[15])
            r_v[0]  <= g_diag0[127:96];
            r_v[5]  <= g_diag0[95:64];
            r_v[10] <= g_diag0[63:32];
            r_v[15] <= g_diag0[31:0];

            // Diagonal 1: (v[1], v[6], v[11], v[12])
            r_v[1]  <= g_diag1[127:96];
            r_v[6]  <= g_diag1[95:64];
            r_v[11] <= g_diag1[63:32];
            r_v[12] <= g_diag1[31:0];

            // Diagonal 2: (v[2], v[7], v[8], v[13])
            r_v[2]  <= g_diag2[127:96];
            r_v[7]  <= g_diag2[95:64];
            r_v[8]  <= g_diag2[63:32];
            r_v[13] <= g_diag2[31:0];

            // Diagonal 3: (v[3], v[4], v[9], v[14])
            r_v[3]  <= g_diag3[127:96];
            r_v[4]  <= g_diag3[95:64];
            r_v[9]  <= g_diag3[63:32];
            r_v[14] <= g_diag3[31:0];

            r_round <= r_round + 3'd1;

            if (r_round == 3'd6) begin
                r_round <= 3'd0;
                r_state <= S_OUTPUT;
            end else begin
                // Apply message permutation σ before next round.
                // new[i] = old[σ[i]]:  σ = {2,6,3,10,7,0,4,13,1,11,12,5,9,14,15,8}
                r_mblock[ 0*32 +: 32] <= r_mblock[ 2*32 +: 32];  // σ[0]  = 2
                r_mblock[ 1*32 +: 32] <= r_mblock[ 6*32 +: 32];  // σ[1]  = 6
                r_mblock[ 2*32 +: 32] <= r_mblock[ 3*32 +: 32];  // σ[2]  = 3
                r_mblock[ 3*32 +: 32] <= r_mblock[10*32 +: 32];  // σ[3]  = 10
                r_mblock[ 4*32 +: 32] <= r_mblock[ 7*32 +: 32];  // σ[4]  = 7
                r_mblock[ 5*32 +: 32] <= r_mblock[ 0*32 +: 32];  // σ[5]  = 0
                r_mblock[ 6*32 +: 32] <= r_mblock[ 4*32 +: 32];  // σ[6]  = 4
                r_mblock[ 7*32 +: 32] <= r_mblock[13*32 +: 32];  // σ[7]  = 13
                r_mblock[ 8*32 +: 32] <= r_mblock[ 1*32 +: 32];  // σ[8]  = 1
                r_mblock[ 9*32 +: 32] <= r_mblock[11*32 +: 32];  // σ[9]  = 11
                r_mblock[10*32 +: 32] <= r_mblock[12*32 +: 32];  // σ[10] = 12
                r_mblock[11*32 +: 32] <= r_mblock[ 5*32 +: 32];  // σ[11] = 5
                r_mblock[12*32 +: 32] <= r_mblock[ 9*32 +: 32];  // σ[12] = 9
                r_mblock[13*32 +: 32] <= r_mblock[14*32 +: 32];  // σ[13] = 14
                r_mblock[14*32 +: 32] <= r_mblock[15*32 +: 32];  // σ[14] = 15
                r_mblock[15*32 +: 32] <= r_mblock[ 8*32 +: 32];  // σ[15] = 8
                r_state <= S_GCOL;
            end
        end

        // ------------------------------------------------------------------
        // Output: XOR the two halves of the state with each other and the
        // original chaining value.
        //
        //   o_hash[i*32 +: 32]       = v[i]   ^ v[i+8]          i = 0..7
        //   o_hash[(i+8)*32 +: 32]   = v[i+8] ^ h[i]            i = 0..7
        //
        // where h[i] = i_chain[i*32 +: 32] is the input chaining value.
        // i_chain is held stable by the caller during the entire operation.
        // ------------------------------------------------------------------
        S_OUTPUT: begin
            for (ii = 0; ii < 8; ii = ii + 1) begin
                o_hash[ ii    *32 +: 32] <= r_v[ii]   ^ r_v[ii+8];
                o_hash[(ii+8) *32 +: 32] <= r_v[ii+8] ^ i_chain[ii*32 +: 32];
            end
            o_valid <= 1'b1;
            r_state <= S_IDLE;
        end

        // ------------------------------------------------------------------
        default: r_state <= S_IDLE;

        endcase
    end
end

endmodule