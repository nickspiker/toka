// Single-instance CE-gated BLAKE3 self-test for Colorlight 5A-75B v8.0
//
// One blake3 instance, run twice: slow CE (gold, guaranteed correct) then
// fast CE (test, full PLL speed). Counter-based protocol ensures both
// phases process identical PRNG sequences. Compare accumulated hashes.
//
// CRT shows: gold hash (bottom strip), test hash (top strip, flipped CRT).
// Output: J1 pin 1 (R0, C4) = sync, J1 pin 2 (G0, D4) = video

module top_ntsc (
    input  wire clk,       // 25 MHz oscillator (P6)
    output reg  led,       // LED on T6 (active-low)
    input  wire btn,       // User button on R7 (active-low)
    output reg  ntsc_sync, // J1 R0 (C4) — 560Ω
    output reg  ntsc_vid   // J1 G0 (D4) — 220Ω
);

    // =========================================================================
    // PLL: high-speed clock for test
    // =========================================================================
    wire sys_clk, pll_lock;

    // Power-on reset: hold blake3 in reset for 15 sys_clk cycles after PLL lock
    reg [3:0] por_cnt = 0;
    wire      por_done = &por_cnt;

`ifdef PLL_CLKFB_DIV
    (* ICP_CURRENT="12" *) (* LPF_RESISTOR="8" *)
    (* MFG_ENABLE_FILTEROPAMP="1" *) (* MFG_GMCREF_SEL="2" *)
    EHXPLLL #(
        .PLLRST_ENA       ("DISABLED"),
        .INTFB_WAKE       ("DISABLED"),
        .STDBY_ENABLE     ("DISABLED"),
        .DPHASE_SOURCE    ("DISABLED"),
        .OUTDIVIDER_MUXA  ("DIVA"),
        .OUTDIVIDER_MUXB  ("DIVB"),
        .OUTDIVIDER_MUXC  ("DIVC"),
        .OUTDIVIDER_MUXD  ("DIVD"),
        .CLKI_DIV         (`PLL_CLKI_DIV),
        .CLKFB_DIV        (`PLL_CLKFB_DIV),
        .CLKOP_ENABLE     ("ENABLED"),
        .CLKOP_DIV        (`PLL_CLKOP_DIV),
        .CLKOP_CPHASE     (`PLL_CLKOP_CPHASE),
        .CLKOP_FPHASE     (0),
        .FEEDBK_PATH      ("CLKOP")
    ) pll (
        .RST(1'b0), .STDBY(1'b0), .CLKI(clk),
        .CLKOP(sys_clk), .CLKFB(sys_clk), .CLKINTFB(),
        .PHASESEL0(1'b0), .PHASESEL1(1'b0),
        .PHASEDIR(1'b1), .PHASESTEP(1'b1), .PHASELOADREG(1'b1),
        .PLLWAKESYNC(1'b0), .ENCLKOP(1'b0), .LOCK(pll_lock)
    );
`else
    assign sys_clk  = clk;
    assign pll_lock = 1'b1;
`endif

    always @(posedge sys_clk)
        if (!pll_lock)    por_cnt <= 0;
        else if (!por_done) por_cnt <= por_cnt + 1;

    // Blake3 reset: active-low. Assert during POR and button hold.
    wire b3_reset_n = por_done & pll_lock & ~btn_held_sys;

    // =========================================================================
    // Constants
    // =========================================================================
    localparam [31:0] SEED = 32'hCAFE_BABE;
    localparam PRNG_FILL   = 24;       // cycles to fill 768 bits
    localparam CE_GOLD_DIV = 256;      // gold CE divider (effective freq = PLL/256)

    // Protocol: 10-bit counter, bit taps for events (zero comparisons)
    //   [0..127]   warmup (LFSR runs, no accumulation)
    //   [128..511]  accumulate (384 cycles)
    //   bit 9 high  → done
    localparam PROTO_BITS = 10;

    // =========================================================================
    // XOR-fold: 512 bits → 32 bits
    // =========================================================================
    function [31:0] xor_fold;
        input [511:0] h;
        integer i;
        begin
            xor_fold = 32'b0;
            for (i = 0; i < 16; i = i + 1)
                xor_fold = xor_fold ^ h[i*32 +: 32];
        end
    endfunction

    // =========================================================================
    // Phase + CE
    // =========================================================================
    localparam [1:0] PH_GOLD = 2'd0, PH_SWITCH = 2'd1,
                     PH_TEST = 2'd2, PH_DONE   = 2'd3;
    reg [1:0] phase = PH_GOLD;

    // CE generator (registered for clean fanout at high freq)
    reg [7:0] ce_div = 0;
    reg       ce = 0;
    always @(posedge sys_clk) begin
        ce_div <= (ce_div == CE_GOLD_DIV - 1) ? 8'd0 : ce_div + 1;
        ce     <= (phase == PH_TEST) || (ce_div == CE_GOLD_DIV - 1);
    end

    // =========================================================================
    // PRNG: 64-bit Galois LFSR (maximal length, period 2^64-1)
    // Critical path: lfsr[0] → 1 LUT (AND tap) → FF.  ~0.7ns.
    // Taps: x^64 + x^63 + x^61 + x^60  (maximal, from Xilinx XAPP052)
    // Output: lfsr[31:0] window
    // =========================================================================
    localparam [63:0] LFSR_SEED = 64'hCAFE_BABE_DEAD_BEEF;
    localparam [63:0] LFSR_TAPS = 64'hD800000000000000;  // bits 63,62,60,59

    reg [63:0] lfsr;
    wire       lfsr_fb = lfsr[0];
    wire [63:0] lfsr_next = {1'b0, lfsr[63:1]} ^ (lfsr_fb ? LFSR_TAPS : 64'b0);

    // Aliases for blake3 fill path
    reg [31:0] s1, s2, s3;
    wire [31:0] s1_next = s3 ^ (s3 << 13);
    wire [31:0] s2_next = s1 ^ (s1 >> 17);
    wire [31:0] s3_next = s2 ^ (s2 << 5);
    reg [767:0] sr;

    // =========================================================================
    // Blake3 (CE-gated)
    // =========================================================================
    reg         b3_valid;
    wire [511:0] b3_hash;
    wire        b3_done;

    blake3 b3 (
        .i_clk(sys_clk),
        .i_reset(b3_reset_n),
        .i_ce(ce),
        .i_chain(sr[255:0]),
        .i_mblock(sr[767:256]),
        .i_counter(64'b0),
        .i_numbytes(32'd64),
        .i_dflags(32'h03),
        .i_valid(b3_valid),
        .o_hash(b3_hash),
        .o_valid(b3_done)
    );

    // =========================================================================
    // Fill/strobe/wait FSM + protocol counter + accumulator + phase FSM
    // All in ONE always block to avoid multi-driver issues
    // =========================================================================
    localparam [1:0] F_FILL = 2'd0, F_STROBE = 2'd1, F_WAIT_ACK = 2'd2, F_WAIT_DONE = 2'd3;
    reg [1:0]  f_state;
    reg [4:0]  fill_cnt;

    reg [PROTO_BITS-1:0] proto_cnt;
    wire       proto_done = proto_cnt[9];           // bit tap: done at 512
    wire       accumulating = proto_cnt[7] & ~proto_done;  // bit tap: accum from 128..511
    reg [31:0] accum;
    reg [31:0] gold_reg = 0, test_reg = 0;
    reg        test_done_sys = 0;

    always @(posedge sys_clk) begin
        if (!pll_lock || !por_done || btn_held_sys) begin
            // Global reset (PLL not locked OR button held)
            phase         <= PH_GOLD;
            lfsr          <= LFSR_SEED;
            s1            <= SEED;
            s2            <= 0;
            s3            <= 0;
            sr            <= 0;
            b3_valid      <= 0;
            f_state       <= F_FILL;
            fill_cnt      <= 0;
            proto_cnt      <= 0;
            accum          <= 0;
            gold_reg       <= 0;
            test_reg       <= 0;
            test_done_sys  <= 0;
        end else begin

            // =================================================================
            // Phase transitions (not CE-gated, run every sys_clk cycle)
            // =================================================================
            case (phase)
                PH_GOLD: begin
                    if (proto_done) begin
                        gold_reg <= accum;
                        phase    <= PH_SWITCH;
                    end
                end
                PH_SWITCH: begin
                    // Reset datapath for test phase
                    lfsr         <= LFSR_SEED;
                    s1           <= SEED;
                    s2           <= 0;
                    s3           <= 0;
                    sr           <= 0;
                    b3_valid     <= 0;
                    f_state      <= F_FILL;
                    fill_cnt     <= 0;
                    proto_cnt      <= 0;
                    accum          <= 0;
                    phase          <= PH_TEST;
                end
                PH_TEST: begin
                    if (proto_done) begin
                        test_reg      <= accum;
                        test_done_sys <= 1;
                        phase         <= PH_DONE;
                    end
                end
                PH_DONE: ;
            endcase

            // =================================================================
            // CE-gated datapath (only runs when ce=1)
            // =================================================================
            if (ce && phase != PH_SWITCH && phase != PH_DONE) begin
                b3_valid <= 0;

                // LFSR advance (1 bit per CE cycle)
                lfsr <= lfsr_next;

                // Blake3 fill/strobe/wait FSM
                case (f_state)
                    F_FILL: begin
                        sr <= {sr[735:0], lfsr[31:0]};
                        s1 <= s1_next; s2 <= s2_next; s3 <= s3_next;
                        fill_cnt <= fill_cnt + 1;
                        if (fill_cnt == PRNG_FILL - 1)
                            f_state <= F_STROBE;
                    end
                    F_STROBE: begin
                        b3_valid <= 1;
                        f_state  <= F_WAIT_ACK;
                    end
                    F_WAIT_ACK: begin
                        // Wait 1 cycle for blake3 to latch input
                        f_state <= F_WAIT_DONE;
                    end
                    F_WAIT_DONE: begin
                        if (b3_done) begin
                            // Accumulate when in accumulation window
                            if (accumulating)
                                accum <= {accum[30:0], accum[31]} ^ xor_fold(b3_hash);
                            // Start next fill
                            fill_cnt <= 0;
                            f_state  <= F_FILL;
                        end
                    end
                endcase

                // Counter advance (bit 9 = done, stays high = stops counting)
                if (!proto_done)
                    proto_cnt <= proto_cnt + 1;
            end
        end
    end

    // =========================================================================
    // Button debounce (clk domain)
    // =========================================================================
    reg [1:0] btn_sync = 2'b11;
    reg [17:0] btn_deb = 0;
    reg btn_clean = 1, btn_prev = 1;
    wire btn_press = btn_prev & ~btn_clean;

    always @(posedge clk) begin
        btn_sync <= {btn_sync[0], btn};
        if (btn_sync[1] != btn_clean) begin
            btn_deb <= btn_deb + 1;
            if (&btn_deb) btn_clean <= btn_sync[1];
        end else
            btn_deb <= 0;
        btn_prev <= btn_clean;
    end

    // CDC: btn_clean → sys_clk domain (btn_clean=0 when pressed, active-low)
    reg [1:0] btn_sync_sys = 2'b11;
    always @(posedge sys_clk) btn_sync_sys <= {btn_sync_sys[0], btn_clean};
    wire btn_held_sys = ~btn_sync_sys[1];  // 1 when button is held down

    // =========================================================================
    // CDC: test_done_sys → clk domain
    // =========================================================================
    reg done_sync1 = 0, done_sync2 = 0;
    always @(posedge clk) begin
        done_sync1 <= test_done_sys;
        done_sync2 <= done_sync1;
    end

    reg lock_sync1 = 0, lock_sync2 = 0;
    always @(posedge clk) begin
        lock_sync1 <= pll_lock;
        lock_sync2 <= lock_sync1;
    end

    // =========================================================================
    // Status (clk domain)
    // =========================================================================
    wire pass = (gold_reg == test_reg);
    wire [1:0] status = !lock_sync2  ? 2'd2 :
                        !done_sync2  ? 2'd0 :
                        pass         ? 2'd1 : 2'd2;

    // =========================================================================
    // LED (clk domain)
    // =========================================================================
    reg [24:0] blink_ctr = 0;
    always @(posedge clk) blink_ctr <= blink_ctr + 1;

    always @(posedge clk) begin
        case (status)
            2'd0:    led <= blink_ctr[24];
            2'd2:    led <= (blink_ctr[24:22] != 3'b000);
            default: led <= (blink_ctr[24:22] == 3'b000);
        endcase
    end

    // =========================================================================
    // NTSC display (clk domain = 25 MHz)
    // =========================================================================
    wire ntsc_sync_w, ntsc_vid_w;
    wire btn_held = ~btn_clean;  // clk domain: 1 when button held

    ntsc_framebuf #(
        .FB_W    (320),
        .FB_H    (240),
        .H_SCALE (4)
    ) ntsc (
        .clk       (clk),
        .status    (status),
        .hash      (gold_reg),
        .hash2     (test_reg),
        .sync_pin  (ntsc_sync_w),
        .video_pin (ntsc_vid_w)
    );

    // Kill NTSC output when button held (both low = no signal)
    always @(posedge clk) begin
        ntsc_sync <= btn_held ? 1'b0 : ntsc_sync_w;
        ntsc_vid  <= btn_held ? 1'b0 : ntsc_vid_w;
    end

endmodule
