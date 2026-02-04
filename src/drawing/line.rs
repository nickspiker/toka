//! Anti-aliased line drawing using Xiaolin Wu's algorithm
//!
//! Adapted for Spirix ScalarF4E4 (no IEEE-754 floats).

use spirix::ScalarF4E4;

/// Draw an anti-aliased line on a Scalar pixel buffer
///
/// Uses Xiaolin Wu's line algorithm for smooth anti-aliasing.
/// Blends start and end colours along the line length.
///
/// # Arguments
///
/// * `pixels` - Mutable slice of RGBA pixels (row-major, s44 format)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `x0`, `y0` - Start point coordinates (pixel space)
/// * `x1`, `y1` - End point coordinates (pixel space)
/// * `colour_start` - RGBA colour at line start [r, g, b, a]
/// * `colour_end` - RGBA colour at line end [r, g, b, a]
pub fn draw_line_s44(
    pixels: &mut [[ScalarF4E4; 4]],
    width: usize,
    height: usize,
    x0: ScalarF4E4,
    y0: ScalarF4E4,
    x1: ScalarF4E4,
    y1: ScalarF4E4,
    colour_start: [ScalarF4E4; 4],
    colour_end: [ScalarF4E4; 4],
) {
    // Calculate total line distance for colour interpolation
    let dx = x1 - x0;
    let dy = y1 - y0;
    let total_distance = (dx * dx + dy * dy).sqrt();

    // Check if line is steep (more vertical than horizontal)
    let steep = dy.magnitude() > dx.magnitude();

    let (mut x0, mut y0, mut x1, mut y1) = (x0, y0, x1, y1);

    // For steep lines, swap x and y coordinates
    if steep {
        std::mem::swap(&mut x0, &mut y0);
        std::mem::swap(&mut x1, &mut y1);
    }

    // Ensure we always draw left to right
    if x0 > x1 {
        std::mem::swap(&mut x0, &mut x1);
        std::mem::swap(&mut y0, &mut y1);
    }

    let dx = x1 - x0;
    let dy = y1 - y0;
    let gradient = if dx == ScalarF4E4::ZERO {
        ScalarF4E4::ONE
    } else {
        dy / dx
    };

    // Helper function to plot a pixel with anti-aliasing
    let mut plot = |x: isize, y: isize, coverage: ScalarF4E4, blend_factor: ScalarF4E4| {
        // Bounds check
        if x < 0 || x >= width as isize || y < 0 || y >= height as isize {
            return;
        }

        let idx = (y as usize) * width + (x as usize);

        // Interpolate colour based on position along line
        let mut colour = [ScalarF4E4::ZERO; 4];
        for i in 0..4 {
            colour[i] = colour_start[i] * (ScalarF4E4::ONE - blend_factor)
                + colour_end[i] * blend_factor;
        }

        // Alpha blend with anti-aliasing coverage
        let alpha = colour[3] * coverage;
        let inv_alpha = ScalarF4E4::ONE - alpha;

        for i in 0..4 {
            pixels[idx][i] = colour[i] * alpha + pixels[idx][i] * inv_alpha;
        }
    };

    // First endpoint
    let xend = x0.round();
    let yend = y0 + gradient * (xend - x0);
    let xgap = ScalarF4E4::ONE - (x0 + ScalarF4E4::from(0.5)).frac();
    let xpxl1 = xend;
    let ypxl1 = yend.floor();

    if steep {
        plot(
            ypxl1.to_isize(),
            xpxl1.to_isize(),
            (ScalarF4E4::ONE - yend.frac()) * xgap,
            ScalarF4E4::ZERO,
        );
        plot(
            (ypxl1 + ScalarF4E4::ONE).to_isize(),
            xpxl1.to_isize(),
            yend.frac() * xgap,
            ScalarF4E4::ZERO,
        );
    } else {
        plot(
            xpxl1.to_isize(),
            ypxl1.to_isize(),
            (ScalarF4E4::ONE - yend.frac()) * xgap,
            ScalarF4E4::ZERO,
        );
        plot(
            xpxl1.to_isize(),
            (ypxl1 + ScalarF4E4::ONE).to_isize(),
            yend.frac() * xgap,
            ScalarF4E4::ZERO,
        );
    }

    let mut intery = yend + gradient;

    // Second endpoint
    let xend = x1.round();
    let yend = y1 + gradient * (xend - x1);
    let xgap = (x1 + ScalarF4E4::from(0.5)).frac();
    let xpxl2 = xend;
    let ypxl2 = yend.floor();

    if steep {
        plot(
            ypxl2.to_isize(),
            xpxl2.to_isize(),
            (ScalarF4E4::ONE - yend.frac()) * xgap,
            ScalarF4E4::ONE,
        );
        plot(
            (ypxl2 + ScalarF4E4::ONE).to_isize(),
            xpxl2.to_isize(),
            yend.frac() * xgap,
            ScalarF4E4::ONE,
        );
    } else {
        plot(
            xpxl2.to_isize(),
            ypxl2.to_isize(),
            (ScalarF4E4::ONE - yend.frac()) * xgap,
            ScalarF4E4::ONE,
        );
        plot(
            xpxl2.to_isize(),
            (ypxl2 + ScalarF4E4::ONE).to_isize(),
            yend.frac() * xgap,
            ScalarF4E4::ONE,
        );
    }

    // Main loop - draw line between endpoints
    let start_x = xpxl1.to_isize() + 1;
    let end_x = xpxl2.to_isize();

    if steep {
        for x in start_x..end_x {
            let x_s44 = ScalarF4E4::from(x);
            let current_dx = x_s44 - x0;
            let current_dy = intery - y0;
            let current_distance = (current_dx * current_dx + current_dy * current_dy).sqrt();
            let blend_factor = current_distance / total_distance;

            plot(
                intery.floor().to_isize(),
                x,
                ScalarF4E4::ONE - intery.frac(),
                blend_factor,
            );
            plot(
                (intery.floor() + ScalarF4E4::ONE).to_isize(),
                x,
                intery.frac(),
                blend_factor,
            );
            intery = intery + gradient;
        }
    } else {
        for x in start_x..end_x {
            let x_s44 = ScalarF4E4::from(x);
            let current_dx = x_s44 - x0;
            let current_dy = intery - y0;
            let current_distance = (current_dx * current_dx + current_dy * current_dy).sqrt();
            let blend_factor = current_distance / total_distance;

            plot(
                x,
                intery.floor().to_isize(),
                ScalarF4E4::ONE - intery.frac(),
                blend_factor,
            );
            plot(
                x,
                (intery.floor() + ScalarF4E4::ONE).to_isize(),
                intery.frac(),
                blend_factor,
            );
            intery = intery + gradient;
        }
    }
}
