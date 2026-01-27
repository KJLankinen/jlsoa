use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jlsoa::{Aos, Soa, StructMetadata};
use rand::SeedableRng;
use rand_distr::{Distribution, Normal, Uniform};

#[derive(Copy, Clone, Default, Aos)]
pub struct Sphere {
    pub position: [f32; 3],
    pub radius: f32,
    pub tag: u64,
    // Adding some dummy data to showcase how Aos access gets worse when more data is added to the
    // struct, while soa access stays constant, regardless of the "extra" fields.
    // Even with all these commented out, i.e. only necessary data in the struct, the soa access is
    // faster, probably due to auto-vectorization.
    //pub colour: [f32; 3],
    //pub colour0: [f32; 3],
    //pub colour1: [f32; 3],
    //pub colour2: [f32; 3],
}

const N: usize = 1 << 20;
type SphereSoa = Soa<Sphere, { Sphere::LAYOUT.len() }>;

fn generate_spheres() -> Vec<Sphere> {
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(1);

    let normal = Normal::new(1.0, 0.2).unwrap();
    let uniform = Uniform::new(-1.0, 1.0);
    let x: Vec<f32> = uniform.sample_iter(&mut rng).take(N).collect();
    let y: Vec<f32> = uniform.sample_iter(&mut rng).take(N).collect();
    let z: Vec<f32> = uniform.sample_iter(&mut rng).take(N).collect();
    let r: Vec<f32> = normal.sample_iter(&mut rng).take(N).collect();

    (0..N)
        .map(|i| Sphere {
            position: [x[i], y[i], z[i]],
            radius: r[i],
            ..Default::default()
        })
        .collect::<Vec<Sphere>>()
}

fn generate_soa(capacity: usize) -> SphereSoa {
    let mut soa = SphereSoa::new(capacity);
    for sphere in generate_spheres() {
        soa.push(&sphere);
    }
    soa
}

fn assign_tag(position: &[f32; 3], radius: &f32) -> u64 {
    let limit_min = -1.0;
    let limit_max = 1.0;
    let mut tag = 0u64;
    for p in position {
        if p - radius < limit_min || p + radius > limit_max {
            tag |= 1 << 0;
        }
    }

    tag
}

fn soa_access(c: &mut Criterion) {
    let mut soa = black_box(generate_soa(N));
    c.bench_function("soa access", |b| {
        b.iter(|| {
            let len = soa.len();
            for i in 0..len {
                soa.tag_mut()[i] = assign_tag(&soa.position()[i], &soa.radius()[i]);
            }
        })
    });
}

fn aos_access(c: &mut Criterion) {
    let mut aos = black_box(generate_spheres());
    c.bench_function("aos access", |b| {
        b.iter(|| {
            for sphere in aos.iter_mut() {
                sphere.tag = assign_tag(&sphere.position, &sphere.radius);
            }
        })
    });
}

criterion_group!(benches, soa_access, aos_access,);
criterion_main!(benches);
