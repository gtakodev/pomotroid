use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::sync::{LazyLock, OnceLock};

use rune::{Color, El, Image, ImageCache, LineCap, Shape, canvas, el};

use crate::timer::{Phase, Status};

const FOREWINGS: &str = include_str!("../../static/reactor/luma-forewings.svg");
const HINDWINGS: &str = include_str!("../../static/reactor/luma-hindwings.svg");
const ANTENNAE: &str = include_str!("../../static/reactor/luma-antennae.svg");
const BODY: &str = include_str!("../../static/reactor/luma-body.svg");
const EYES: &str = include_str!("../../static/reactor/luma-eyes.svg");
const GOLD: Color = Color {
    r: 1.0,
    g: 0.75,
    b: 0.39,
    a: 1.0,
};

static IMAGES: OnceLock<ImageCache> = OnceLock::new();

pub(crate) fn setup_images(wake: rune::Waker) {
    IMAGES.get_or_init(|| ImageCache::new(wake));
}

fn silk(source: &str, size: f32, phase: Phase) -> Image {
    let mut svg = source.to_owned();
    // Each column is the same iridescent scale under focus, daylight and moonlight.
    if phase != Phase::Work {
        for [opal, green, violet] in [
            ["#244659", "#264e4a", "#453658"],
            ["#497f98", "#509e83", "#766993"],
            ["#9adbe5", "#ace5bd", "#cebde9"],
            ["#c3ebe4", "#dbf0bb", "#e7d9f2"],
            ["#8ec7d9", "#9cd4ae", "#b7a9db"],
            ["#506781", "#577770", "#68678c"],
        ] {
            svg = svg.replace(
                opal,
                if phase == Phase::ShortBreak {
                    green
                } else {
                    violet
                },
            );
        }
    }
    let image = IMAGES
        .get()
        .expect("Rune initializes images before drawing")
        .svg(svg.into_bytes(), size, 2.0);
    // A scripted capture may jump straight to its last frame before the decoder wakes the window.
    if rune::is_headless() {
        image.wait().expect("bundled Luma artwork is valid SVG");
    }
    image
}

struct Wings {
    fore: Image,
    hind: Image,
}

impl Wings {
    fn new(phase: Phase) -> Self {
        Self {
            fore: silk(FOREWINGS, 204.0, phase),
            hind: silk(HINDWINGS, 152.0, phase),
        }
    }
}

static FOCUS: LazyLock<Wings> = LazyLock::new(|| Wings::new(Phase::Work));
static DAY: LazyLock<Wings> = LazyLock::new(|| Wings::new(Phase::ShortBreak));
static NIGHT: LazyLock<Wings> = LazyLock::new(|| Wings::new(Phase::LongBreak));
static FEELERS: LazyLock<Image> = LazyLock::new(|| silk(ANTENNAE, 86.0, Phase::Work));
static THORAX: LazyLock<Image> = LazyLock::new(|| silk(BODY, 99.0, Phase::Work));
static FACE_EYES: LazyLock<Image> = LazyLock::new(|| silk(EYES, 34.0, Phase::Work));

struct Pose {
    spread: f32,
    wing_height: f32,
    tail: f32,
    antenna: f32,
    eyes: f32,
    bob: f32,
}

impl Pose {
    fn at(phase: Phase, status: Status, time: f32, tap: f32, finale: f32) -> Self {
        let breath = (time * 1.6).sin();
        let tempo = match phase {
            Phase::Work => 5.7,
            Phase::ShortBreak => 3.7,
            Phase::LongBreak => 2.4,
        };
        let beat = (time * tempo).sin();
        let (spread, wing_height, bob) = match status {
            Status::Ready => (0.92 + breath * 0.035, 0.94 + breath * 0.035, -2.5 * breath),
            Status::Waiting => (
                0.79 + breath * 0.04,
                0.86 + breath * 0.03,
                2.0 - breath * 2.0,
            ),
            Status::Running => (
                0.84 + beat * 0.17,
                0.91 - beat * 0.115,
                -4.2 * (time * 2.0).sin(),
            ),
            Status::Paused => (0.56, 0.77, 5.0),
        };
        let display = (PI * finale.clamp(0.0, 1.0)).sin().max(0.0);
        let snap = tap * ((1.0 - tap) * PI * 2.8).cos();
        let blink = (1.0 - ((time + 1.9).rem_euclid(5.3) - 0.16).abs() / 0.16).max(0.0);
        Self {
            spread: (spread + 0.24 * snap) * (1.0 - display)
                + (1.12 + (finale * TAU * 3.0).sin() * 0.065) * display,
            wing_height: wing_height + 0.08 * snap + 0.09 * display,
            tail: 0.87 + 0.12 * spread + 0.09 * display,
            antenna: 1.0 + 0.08 * breath + 0.16 * snap - 0.09 * display,
            eyes: if status == Status::Paused {
                (0.09 + tap + display).min(1.0)
            } else {
                1.0 - 0.91 * blink * (1.0 - tap)
            },
            bob: bob - 10.0 * tap - 11.0 * display,
        }
    }
}

fn moth(phase: Phase, pose: &Pose, center: [f32; 2], scale: f32, size: [f32; 2]) -> El {
    let wings = match phase {
        Phase::Work => &*FOCUS,
        Phase::ShortBreak => &*DAY,
        Phase::LongBreak => &*NIGHT,
    };
    let [cx, cy] = center;
    let layer = |image: &Image, width: f32, top: f32, height: f32| {
        el().absolute(cx - width * scale * 0.5, cy + (top + pose.bob) * scale)
            .size(width * scale, height * scale)
            .pixels(image.pixels())
    };
    let tail_w = 152.0 * (0.23 + 0.77 * pose.spread);
    let eyes_h = 16.0 * pose.eyes;
    el().absolute(0.0, 0.0)
        .size(size[0], size[1])
        .a11y_hidden()
        .child(layer(
            &wings.hind,
            tail_w,
            -19.0 * pose.tail,
            103.0 * pose.tail,
        ))
        .child(layer(
            &wings.fore,
            204.0 * pose.spread,
            -76.0 * pose.wing_height,
            100.0 * pose.wing_height,
        ))
        .child(layer(
            &FEELERS,
            86.0 * pose.antenna,
            -27.0 - 41.0 / pose.antenna,
            41.0 / pose.antenna,
        ))
        .child(layer(&THORAX, 60.0, -41.0, 99.0))
        .child(layer(&FACE_EYES, 34.0, -17.0 - eyes_h * 0.5, eyes_h))
}

fn star(x: f32, y: f32, radius: f32) -> [[f32; 2]; 8] {
    std::array::from_fn(|i| {
        let angle = i as f32 * PI / 4.0;
        let r = if i % 2 == 0 { radius } else { radius * 0.26 };
        [x + r * angle.cos() * 0.76, y + r * angle.sin()]
    })
}

fn moondust(tap: f32, finale: f32, accent: Color) -> El {
    let mut shape = Shape::new();
    let gold = GOLD.mix(accent, 0.12);
    if tap > 0.0 {
        let age = 1.0 - tap;
        for i in 0..16 {
            let k = (i / 2) as f32;
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let x = 192.0 + side * (52.0 + k * 4.4 + (23.0 + k * 3.3) * age);
            let y = 80.0 + (k * 2.4).sin() * 17.0 - (20.0 + k * 4.1) * age + age * age * 24.0;
            shape = shape.fill(star(x, y, 1.3 + 2.6 * tap), gold.alpha(tap * 0.9));
        }
    }
    if (0.0..=1.0).contains(&finale) {
        // Unfurl the lunar phases from a single point above the moth's head.
        let sweep = (finale * 2.5).min(1.0);
        let spread = 1.0 - (1.0 - sweep).powi(3);
        let opacity = (finale * 8.0).min(1.0) * ((1.0 - finale) * 4.0).min(1.0);
        for index in -3i32..=3 {
            let arc = index as f32 * 0.42 * spread;
            let x = 192.0 + arc.sin() * 141.0;
            let y = 95.0 - arc.cos() * (46.0 + spread * 21.0);
            let r = 7.2 - index.abs() as f32 * 0.68;
            let side = if index < 0 { -1.0 } else { 1.0 };
            let terminator = 1.0 - index.abs() as f32 * 0.55;
            let mut outline = Vec::with_capacity(26);
            for j in 0..=12 {
                let angle = j as f32 * PI / 12.0;
                outline.push([x + side * r * angle.sin(), y - r * angle.cos()]);
            }
            for j in (1..12).rev() {
                let angle = j as f32 * PI / 12.0;
                outline.push([x - side * r * terminator * angle.sin(), y - r * angle.cos()]);
            }
            shape = shape.fill(outline, gold.mix(Color::WHITE, 0.26).alpha(opacity));
            if index == 0 {
                for (dx, dy, d) in [(-2.6, -2.1, 2.8), (2.4, 2.7, 1.9), (-0.1, 0.6, 1.1)] {
                    shape = shape.line(
                        [x + dx, y + dy],
                        [x + dx + 0.2, y + dy],
                        d,
                        LineCap::Round,
                        accent.alpha(opacity * 0.31),
                    );
                }
            }
        }
        for i in 0..18 {
            let n = i as f32;
            let angle = -FRAC_PI_2 + n * TAU / 18.0;
            let distance = 0.64 + finale * 0.50;
            let x = 192.0 + angle.cos() * 132.0 * distance;
            let y = 110.0 + angle.sin() * 83.0 * distance;
            let twinkle = (n * 2.29 + finale * TAU * 2.0).sin().abs();
            shape = shape.fill(
                star(x, y, 1.1 + twinkle * 2.5),
                GOLD.alpha(opacity * twinkle * 0.81),
            );
        }
    }
    canvas(shape).absolute(0.0, 0.0).size(384.0, 228.0)
}

pub(super) fn view(
    phase: Phase,
    status: Status,
    time: f32,
    pulse: f32,
    celebration: f32,
    accent: Color,
) -> El {
    let tap = (pulse - 1.0).clamp(0.0, 1.0);
    let pose = Pose::at(phase, status, time, tap, celebration);
    let shine = GOLD.mix(accent, 0.36);
    let mut scene = el().absolute(0.0, 0.0).size(384.0, 228.0).a11y_hidden();
    for side in [-1.0, 1.0] {
        let x = 192.0 + side * 36.0 * pose.spread;
        scene = scene.child(
            el().absolute(x - 31.0, 194.0 - pose.bob * 0.10)
                .size(62.0, 9.0)
                .radial_gradient(
                    [0.5, 0.5],
                    [31.0, 4.5],
                    [
                        (0.0, shine.alpha(0.20)),
                        (0.52, shine.alpha(0.085)),
                        (1.0, shine.alpha(0.0)),
                    ],
                ),
        );
    }
    scene = scene.child(moth(phase, &pose, [192.0, 114.0], 1.0, [384.0, 228.0]));
    if tap > 0.0 || celebration >= 0.0 {
        scene = scene.child(moondust(tap, celebration, accent));
    }
    scene
}

pub(super) fn portrait() -> El {
    let pose = Pose::at(Phase::Work, Status::Ready, 0.0, 0.0, -1.0);
    el().size(64.0, 48.0).a11y_hidden().child(moth(
        Phase::Work,
        &pose,
        [32.0, 22.5],
        0.305,
        [64.0, 48.0],
    ))
}
