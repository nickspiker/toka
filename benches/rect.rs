use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spirix::{CircleF4E4, ScalarF4E4};
use toka::drawing::CanvasFast;
use vsf::types::VsfType;

fn bench_rect(c: &mut Criterion) {
    let mut g = c.benchmark_group("fill_rotated_rect");

    let w = 1920usize;
    let h = 1080usize;

    let pos  = CircleF4E4::from((0, 0));
    let size = CircleF4E4::from((1, 1));
    let size_skinny = CircleF4E4::from((2, 0));  // thin bar
    let angle_0   = ScalarF4E4::ZERO;
    let angle_45  = ScalarF4E4::PI >> 2;
    let angle_30  = ScalarF4E4::PI / ScalarF4E4::from(6);
    let colour = 0xFF00007Fu32;

    g.bench_function("axis_aligned", |b| {
        let mut canvas = CanvasFast::new(w, h);
        b.iter(|| {
            canvas.clear(&VsfType::rck).unwrap();
            canvas.fill_rotated_rect_ru(black_box(pos), black_box(size), black_box(angle_0), black_box(colour));
        });
    });

    g.bench_function("45deg", |b| {
        let mut canvas = CanvasFast::new(w, h);
        b.iter(|| {
            canvas.clear(&VsfType::rck).unwrap();
            canvas.fill_rotated_rect_ru(black_box(pos), black_box(size), black_box(angle_45), black_box(colour));
        });
    });

    g.bench_function("30deg", |b| {
        let mut canvas = CanvasFast::new(w, h);
        b.iter(|| {
            canvas.clear(&VsfType::rck).unwrap();
            canvas.fill_rotated_rect_ru(black_box(pos), black_box(size), black_box(angle_30), black_box(colour));
        });
    });

    g.bench_function("skinny_45deg", |b| {
        let mut canvas = CanvasFast::new(w, h);
        b.iter(|| {
            canvas.clear(&VsfType::rck).unwrap();
            canvas.fill_rotated_rect_ru(black_box(pos), black_box(size_skinny), black_box(angle_45), black_box(colour));
        });
    });

    g.finish();
}

criterion_group!(benches, bench_rect);
criterion_main!(benches);
