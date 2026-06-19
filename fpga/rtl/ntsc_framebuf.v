// NTSC B&W composite framebuffer display
// Pixel clock: 25 MHz, 1-bit framebuffer in BRAM
// Output: 2-pin DAC (sync + video) thru resistor network
//
// Stores 3 bitmaps (RUN/PASS/FAIL) in one ROM, selected by status input.
// DSP-free address math (shift-add for ×40).

module ntsc_framebuf #(
    parameter FB_W     = 320,  // framebuffer width (must be multiple of 8)
    parameter FB_H     = 240,  // framebuffer height
    parameter H_SCALE  = 4     // clocks per framebuffer pixel (horizontal)
)(
    input  wire        clk,       // 25 MHz
    input  wire [1:0]  status,    // 0=RUN, 1=PASS, 2=FAIL
    input  wire [31:0] hash,      // displayed as 32 blocks (rows 30-45)
    input  wire [31:0] hash2,     // displayed as 32 blocks (rows 210-225, top on flipped CRT)
    output reg         sync_pin,
    output reg         video_pin
);

    // =========================================================================
    // NTSC timing (25 MHz)
    // =========================================================================
    localparam H_TOTAL   = 1589;
    localparam H_SYNC    = 118;
    localparam H_BACK    = 142;
    localparam H_MAX_ACT = 1280;

    localparam V_TOTAL   = 262;
    localparam V_SYNC    = 3;
    localparam V_TOP     = 19;
    localparam V_MAX_ACT = 240;

    // Content centering
    localparam H_CONTENT = FB_W * H_SCALE;
    localparam V_CONTENT = FB_H;
    localparam H_BORDER  = (H_MAX_ACT - H_CONTENT) / 2;
    localparam V_BORDER  = (V_MAX_ACT - V_CONTENT) / 2;

    localparam H_CONTENT_START = H_SYNC + H_BACK + H_BORDER;
    localparam V_CONTENT_START = V_SYNC + V_TOP + V_BORDER;

    // Framebuffer geometry
    localparam BYTES_PER_ROW = FB_W / 8;          // 40
    localparam FB_BYTES      = BYTES_PER_ROW * FB_H;  // 9600
    localparam ROM_BYTES     = FB_BYTES * 3;       // 28800 (RUN+PASS+FAIL)

    // =========================================================================
    // Scan counters
    // =========================================================================
    reg [10:0] h_cnt = 0;
    reg [8:0]  v_cnt = 0;
    wire h_end = (h_cnt == H_TOTAL - 1);

    always @(posedge clk) begin
        if (h_end) begin
            h_cnt <= 0;
            v_cnt <= (v_cnt == V_TOTAL - 1) ? 0 : v_cnt + 1;
        end else begin
            h_cnt <= h_cnt + 1;
        end
    end

    wire h_sync_region = (h_cnt < H_SYNC);
    wire v_sync_region = (v_cnt < V_SYNC);
    wire in_sync = h_sync_region | v_sync_region;

    // =========================================================================
    // Content region tracking
    // =========================================================================
    wire h_in_content = (h_cnt >= H_CONTENT_START) &&
                        (h_cnt <  H_CONTENT_START + H_CONTENT);
    wire v_in_content = (v_cnt >= V_CONTENT_START) &&
                        (v_cnt <  V_CONTENT_START + V_CONTENT);

    reg [3:0]  h_scale_cnt = 0;
    reg [9:0]  fb_x = 0;

    always @(posedge clk) begin
        if (h_cnt == H_CONTENT_START - 1) begin
            h_scale_cnt <= 0;
            fb_x        <= 0;
        end else if (h_in_content) begin
            if (h_scale_cnt == H_SCALE - 1) begin
                h_scale_cnt <= 0;
                fb_x <= fb_x + 1;
            end else begin
                h_scale_cnt <= h_scale_cnt + 1;
            end
        end
    end

    wire [8:0] fb_y = v_cnt - V_CONTENT_START;

    // =========================================================================
    // Framebuffer ROM — 3 bitmaps concatenated (RUN @ 0, PASS @ 9600, FAIL @ 19200)
    // =========================================================================
    reg [7:0] fb_rom [0:ROM_BYTES-1];
    initial $readmemh("build/status_320x240.mem", fb_rom);

    // DSP-free address: fb_y * 40 = (fb_y << 5) + (fb_y << 3)
    wire [14:0] row_offset = {fb_y, 5'b0} + {2'b0, fb_y, 3'b0};  // fb_y*32 + fb_y*8
    wire [14:0] byte_in_row = fb_x[9:3];

    // Status offset: 0=0, 1=9600, 2=19200
    reg [14:0] status_offset;
    always @(*) begin
        case (status)
            2'd1:    status_offset = 15'd9600;
            2'd2:    status_offset = 15'd19200;
            default: status_offset = 15'd0;
        endcase
    end

    wire [14:0] fb_addr = status_offset + row_offset + byte_in_row;
    wire [2:0]  fb_bit  = fb_x[2:0];

    reg [7:0] fb_byte;
    always @(posedge clk) fb_byte <= fb_rom[fb_addr];

    // Hash overlay 1 (BRAM readback): rows 30-45, marker at row 29
    wire in_hash1_rows = (fb_y >= 9'd30) & (fb_y < 9'd46);
    wire in_marker1_row = (fb_y == 9'd29 || fb_y == 9'd46) & (fb_x < 10'd256);
    wire [4:0] hash_idx = fb_x[7:3];
    wire hash1_px = hash[31 - hash_idx] & ~fb_x[8];

    // Hash overlay 2 (XOR mismatch): rows 210-225, marker at row 209
    // High rows = top on flipped CRT
    wire in_hash2_rows = (fb_y >= 9'd210) & (fb_y < 9'd226);
    wire in_marker2_row = (fb_y == 9'd209 || fb_y == 9'd226) & (fb_x < 10'd256);
    wire hash2_px = hash2[31 - hash_idx] & ~fb_x[8];

    // 1-cycle read latency compensation
    reg [2:0] fb_bit_d1;
    reg       content_d1;
    reg       hash1_region_d1, hash2_region_d1;
    reg       hash1_pixel_d1, hash2_pixel_d1;
    reg       marker1_d1, marker2_d1;
    always @(posedge clk) begin
        fb_bit_d1      <= fb_bit;
        content_d1     <= h_in_content & v_in_content;
        hash1_region_d1 <= h_in_content & v_in_content & in_hash1_rows;
        hash1_pixel_d1  <= hash1_px;
        marker1_d1     <= h_in_content & v_in_content & in_marker1_row;
        hash2_region_d1 <= h_in_content & v_in_content & in_hash2_rows;
        hash2_pixel_d1  <= hash2_px;
        marker2_d1     <= h_in_content & v_in_content & in_marker2_row;
    end

    wire in_overlay = hash1_region_d1 | marker1_d1 | hash2_region_d1 | marker2_d1;
    wire overlay_px = (marker1_d1 | marker2_d1) ? 1'b1 :
                      hash1_region_d1 ? hash1_pixel_d1 :
                      hash2_pixel_d1;
    wire pixel  = in_overlay ? overlay_px : fb_byte[7 - fb_bit_d1];
    wire active = content_d1;

    // =========================================================================
    // Composite output
    // =========================================================================
    always @(posedge clk) begin
        sync_pin  <= ~in_sync;
        video_pin <= active & pixel;
    end

endmodule
