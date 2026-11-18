use std::cell::LazyCell;
use std::f32::consts::{PI, TAU};

use image::{RgbaImage, imageops};
use rune::{Anim, Color, El, LineCap, Pixels, Shape, canvas, el};

use crate::timer::{Phase, Status};

struct Clack {
    radio: Pixels,
    head: Pixels,
    eye: Pixels,
    legs: [Pixels; 2],
    palms: [Pixels; 2],
    fingers: [Pixels; 2],
}

thread_local! {
    static ART: LazyCell<Clack> = LazyCell::new(Clack::new);
}

impl Clack {
    fn new() -> Self {
        const SCALE: u32 = 4;
        // The PNG is the 4× export of clack.svg. Opening the gallery only decodes the atlas.
        let atlas = image::load_from_memory_with_format(
            include_bytes!("../../static/reactor/clack.png"),
            image::ImageFormat::Png,
        )
        .expect("bundled Clack artwork is a valid PNG")
        .into_rgba8();
        let cut = |x, y, w, h| {
            imageops::crop_imm(&atlas, x * SCALE, y * SCALE, w * SCALE, h * SCALE).to_image()
        };
        let texture =
            |image: RgbaImage| super::texture(image.width(), image.height(), image.into_raw());
        let pair = |image: RgbaImage| {
            let mirrored = imageops::flip_horizontal(&image);
            [texture(image), texture(mirrored)]
        };
        Self {
            radio: texture(cut(0, 0, 118, 94)),
            head: texture(cut(121, 1, 69, 33)),
            eye: texture(cut(121, 36, 24, 24)),
            legs: pair(cut(149, 36, 41, 34)),
            palms: pair(cut(1, 100, 42, 44)),
            fingers: pair(cut(52, 100, 23, 32)),
        }
    }
}

#[derive(Clone, Copy)]
struct Stage {
    origin: [f32; 2],
    scale: f32,
    size: [f32; 2],
}

impl Stage {
    fn point(self, [x, y]: [f32; 2]) -> [f32; 2] {
        [
            self.origin[0] + x * self.scale,
            self.origin[1] + y * self.scale,
        ]
    }

    fn sprite(self, pixels: &Pixels, [x, y, w, h]: [f32; 4]) -> El {
        let [x, y] = self.point([x, y]);
        super::plate(pixels, x, y, w * self.scale, h * self.scale)
    }

    fn stroke(self, ink: Shape, a: [f32; 2], b: [f32; 2], width: f32, color: Color) -> Shape {
        ink.line(
            self.point(a),
            self.point(b),
            width * self.scale,
            LineCap::Round,
            color,
        )
    }

    fn ink(self, ink: Shape) -> El {
        canvas(ink)
            .absolute(0.0, 0.0)
            .size(self.size[0], self.size[1])
    }
}

fn metal(r: u8, g: u8, b: u8) -> Color {
    let [r, g, b] = [r, g, b].map(|v| rune::color::srgb_decode(f32::from(v) / 255.0));
    Color { r, g, b, a: 1.0 }
}

fn turn(sprite: El, angle: f32, pivot: [f32; 2]) -> El {
    sprite
        .origin(pivot[0], pivot[1])
        .rotate(angle)
        .rotate_motion(Anim::INSTANT)
}

fn between(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    std::array::from_fn(|i| a[i] + t * (b[i] - a[i]))
}

fn arm(stage: Stage, ink: Shape, shoulder: [f32; 2], wrist: [f32; 2]) -> Shape {
    let mut ink = stage.stroke(ink, shoulder, wrist, 7.0, metal(68, 54, 51));
    ink = stage.stroke(ink, shoulder, wrist, 3.3, metal(184, 123, 76));
    let dx = wrist[0] - shoulder[0];
    let dy = wrist[1] - shoulder[1];
    let length = dx.hypot(dy).max(1.0);
    let (nx, ny) = (-dy / length * 4.0, dx / length * 4.0);
    for i in 1..=6 {
        let t = i as f32 / 7.5;
        let [x, y] = between(shoulder, wrist, t);
        let a = [x - nx, y - ny];
        let b = [x + nx + dx / length * 1.8, y + ny + dy / length * 1.8];
        ink = stage.stroke(ink, a, b, 2.4, metal(113, 65, 51));
        ink = stage.stroke(ink, a, between(a, b, 0.64), 1.25, metal(245, 189, 122));
    }
    ink
}

fn aerial(stage: Stage, mut ink: Shape, base: [f32; 2], tip: [f32; 2], lamp: Color) -> Shape {
    for (lo, hi, width) in [(0.0, 0.37, 3.8), (0.36, 0.7, 2.7), (0.7, 1.0, 1.8)] {
        let a = between(base, tip, lo);
        let b = between(base, tip, hi);
        ink = stage.stroke(ink, a, b, width, metal(95, 82, 72));
        ink = stage.stroke(
            ink,
            [a[0] - 0.4, a[1]],
            [b[0] - 0.4, b[1]],
            width * 0.45,
            metal(239, 217, 177),
        );
    }
    ink = stage.stroke(ink, tip, tip, 4.8, metal(131, 88, 62));
    stage.stroke(
        ink,
        [tip[0] - 0.4, tip[1] - 0.6],
        tip,
        2.8,
        lamp.mix(Color::WHITE, 0.48),
    )
}

fn broadcast(stage: Stage, tip: [f32; 2], progress: f32, lamp: Color) -> Shape {
    let mut ink = Shape::new();
    let envelope = (PI * progress).sin().max(0.0).sqrt();
    for i in 0..3 {
        let age = (progress * 3.0 + i as f32 / 3.0).fract();
        let r = 6.0 + 18.0 * age;
        let color = lamp.mix(Color::WHITE, 0.42).alpha(envelope * (1.0 - age));
        for start in [-PI + 0.23, -1.23] {
            let arc: Vec<_> = (0..=9)
                .map(|j| {
                    let a = start + j as f32 * 0.095;
                    stage.point([tip[0] + r * a.cos(), tip[1] + r * a.sin()])
                })
                .collect();
            ink = ink.stroke(arc, stage.scale * 1.2, color);
        }
    }
    for (i, side) in [-1.0, 1.0].into_iter().enumerate() {
        let age = (progress * 1.8 + 0.42 * i as f32).fract();
        let x = 98.0 + side * (60.0 + 21.0 * age);
        let y = 61.0 - age * 35.0 + (age * TAU).sin() * 3.0;
        let color = lamp
            .mix(Color::WHITE, 0.6)
            .alpha(envelope * (PI * age).sin().max(0.0));
        ink = stage.stroke(ink, [x - 2.1, y + 0.7], [x, y], 3.3, color);
        ink = stage.stroke(ink, [x + 1.0, y], [x + 1.0, y - 8.0], 1.1, color);
        ink = stage.stroke(ink, [x + 1.0, y - 8.0], [x + 4.4, y - 6.4], 1.7, color);
    }
    ink
}

struct Motion {
    time: f32,
    gait: f32,
    shuffle: f32,
    rest: f32,
    click: f32,
    dance: f32,
    beat: f32,
}

impl Motion {
    fn new(phase: Phase, status: Status, time: f32, pulse: f32, celebration: f32) -> Self {
        let pace = match phase {
            Phase::Work => 1.0,
            Phase::ShortBreak => 0.67,
            Phase::LongBreak => 0.43,
        };
        let step = time * 6.4 * pace;
        let (gait, shuffle) = if status == Status::Running {
            (step.sin(), 7.8 * (step * 0.24).sin())
        } else {
            (0.0, 0.0)
        };
        let age = (2.0 - pulse).clamp(0.0, 1.0);
        let snap = |center: f32| (1.0 - ((age - center) / 0.105).abs()).clamp(0.0, 1.0);
        let click = snap(0.17) + snap(0.53);
        let victory = celebration.clamp(0.0, 1.0);
        let dance = (victory * PI).sin().max(0.0).sqrt();
        let beat = (victory * TAU * 3.0).sin() * dance;
        Self {
            time: time * pace,
            gait,
            shuffle,
            rest: f32::from(status == Status::Paused),
            click,
            dance,
            beat,
        }
    }
}

fn puppet(stage: Stage, phase: Phase, motion: Motion, celebration: f32, accent: Color) -> El {
    let mut root = el().size(stage.size[0], stage.size[1]).a11y_hidden();
    let lamp = match phase {
        Phase::Work => metal(255, 197, 95),
        Phase::ShortBreak => metal(95, 241, 187),
        Phase::LongBreak => metal(168, 167, 255),
    }
    .mix(accent, 0.17);
    let Motion {
        time,
        gait,
        shuffle,
        rest,
        click,
        dance,
        beat,
    } = motion;
    let stage = Stage {
        origin: stage.point([
            shuffle + 7.0 * beat,
            3.0 * rest - 2.5 * gait.abs() - 8.0 * beat.abs() - 3.4 * click,
        ]),
        ..stage
    };
    let mut ink = Shape::new();
    let bob = (time * 1.36).sin() * 1.5 * (1.0 - rest) + beat.abs() * 2.8;
    let radio_y = 23.0 + bob + rest * 3.8;
    let radio_x = 43.0 + beat * 2.3;
    let base = [radio_x + 88.0, radio_y + 15.4];
    let tip = [
        base[0] + 22.5 + 6.5 * beat + click * 2.6,
        base[1] - 31.5 + rest * 7.0,
    ];
    ink = aerial(stage, ink, base, tip, lamp);
    let mut wrists = [[0.0; 2]; 2];
    for (i, sign) in [-1.0, 1.0].into_iter().enumerate() {
        let idle = ((time * 1.5 + i as f32 * 2.7).sin() - 0.14).max(0.0);
        let lift = idle * 5.8 * (1.0 - rest) + (dance + beat * sign) * 9.0 + click * 4.0;
        wrists[i] = [
            98.0 + sign * (64.0 - 7.8 * rest - idle * 1.5 + click * 2.4),
            113.0 + sign * gait * 3.5 + rest * 6.0 - lift + i as f32 * 1.8,
        ];
        ink = arm(stage, ink, [98.0 + sign * 30.0, 124.0], wrists[i]);
    }
    ART.with(|art| {
        for k in 0..3 {
            for (i, sign) in [-1.0, 1.0].into_iter().enumerate() {
                let depth = k as f32;
                let stride = (gait + beat * 1.3) * sign * if k == 1 { -1.0 } else { 1.0 };
                let x = 98.0 + sign * (35.0 - depth * 3.9 + stride * 2.3);
                let y = 112.5 + depth * 4.0 + rest * 2.2;
                let width = 48.0 - depth * 3.5;
                let height = 29.0 + depth * 1.7 - stride.max(0.0) * 3.2 - rest * 4.0;
                let x = if i == 0 { x - width } else { x };
                let leg = stage.sprite(&art.legs[i], [x, y, width, height]);
                root = root.child(turn(
                    leg,
                    -sign * (depth - 1.0) * 0.15 + stride * 0.135,
                    [1.0 - i as f32, 0.13],
                ));
            }
        }
        root = root.child(stage.ink(ink));
        root = root.child(stage.sprite(&art.radio, [radio_x, radio_y, 118.0, 94.0]));
        let dial_glass = [[10.7, 32.0], [87.6, 28.5], [87.3, 37.9], [10.4, 41.3]]
            .map(|[x, y]| stage.point([radio_x + x, radio_y + y]));
        let mut details = Shape::new().fill(dial_glass, lamp.alpha(0.18 - rest * 0.09));
        let needle_x = radio_x + 48.0 + 19.0 * (time * 0.73).sin() + 11.0 * beat;
        let needle_y = radio_y + 31.0 - (needle_x - radio_x - 8.0) * 0.045;
        details = stage.stroke(
            details,
            [needle_x, needle_y],
            [needle_x - 0.4, needle_y + 10.0],
            1.35,
            lamp,
        );
        details = stage.stroke(
            details,
            [needle_x, needle_y + 1.2],
            [needle_x - 0.2, needle_y + 7.4],
            0.49,
            Color::WHITE.alpha(0.82),
        );
        let glance = (time * 0.91).sin() * 1.7 * (1.0 - rest) + click * 1.1;
        let blink = (1.0 - ((time + 2.0).rem_euclid(5.1) - 0.17).abs() / 0.17).clamp(0.0, 1.0);
        let eyes = [-1.0, 1.0].map(|sign| {
            [
                93.0 + 17.8 * sign + glance,
                94.5 - sign + rest * 11.5 + bob + gait * sign - click * 2.5,
            ]
        });
        for (i, sign) in [-1.0, 1.0].into_iter().enumerate() {
            let socket = [94.0 + 12.0 * sign, 122.0 + bob];
            let eye = eyes[i];
            details = stage.stroke(details, socket, eye, 6.6, metal(106, 57, 45));
            details = stage.stroke(
                details,
                [socket[0] - 0.65, socket[1]],
                [eye[0] - 0.65, eye[1]],
                3.8,
                metal(248, 161, 94),
            );
            for t in [0.30, 0.43] {
                let [x, y] = between(socket, eye, t);
                details = stage.stroke(
                    details,
                    [x - 1.6, y + 0.25],
                    [x + 1.7, y - 0.25],
                    1.2,
                    metal(174, 105, 65),
                );
            }
        }
        root = root.child(stage.ink(details));
        root = root.child(stage.sprite(&art.head, [60.5, 111.0 + bob, 69.0, 33.0]));
        for (i, sign) in [-1.0, 1.0].into_iter().enumerate() {
            let [eye_x, eye_y] = eyes[i];
            let height = 23.0 * (1.0 - rest * 0.19 - blink * (1.0 - rest) * 0.83);
            root = root
                .child(stage.sprite(&art.eye, [eye_x - 11.5, eye_y - height * 0.5, 23.0, height]));
            let [x, y] = wrists[i];
            let s = if i == 0 { 1.08 } else { 0.88 };
            let palm_left = if i == 0 { x - 31.0 * s } else { x - 11.0 * s };
            let palm_top = y - 28.2 * s;
            let pivot = [if i == 0 { 31.0 / 42.0 } else { 11.0 / 42.0 }, 28.2 / 44.0];
            let [left, top] = stage.point([palm_left, palm_top]);
            let hand = el()
                .absolute(left, top)
                .size(42.0 * s * stage.scale, 44.0 * s * stage.scale);
            let hand_stage = Stage {
                origin: [0.0, 0.0],
                scale: s * stage.scale,
                size: stage.size,
            };
            let finger_left = if i == 0 { 16.6 } else { 2.4 };
            let finger = hand_stage.sprite(&art.fingers[i], [finger_left, 2.1, 23.0, 32.0]);
            let open = ((time * 1.9 + i as f32 * 2.4).cos() * 0.13 + 0.12) * (1.0 - rest);
            let jaw = sign * (open + rest * 0.42 + click * 0.88 + 0.22 * beat);
            let hand = hand
                .child(turn(finger, jaw, [if i == 0 { 0.63 } else { 0.37 }, 0.81]))
                .child(hand_stage.sprite(&art.palms[i], [0.0, 0.0, 42.0, 44.0]));
            root = root.child(turn(
                hand,
                sign * (0.14 + rest * 0.26 - dance * 0.32) - beat * 0.23,
                pivot,
            ));
        }
        if click > 0.0 {
            let mut strikes = Shape::new();
            for (i, side) in [-1.0, 1.0].into_iter().enumerate() {
                let [x, y] = wrists[i];
                for (dx, dy) in [(0.18, -1.0), (0.75, -0.72), (0.97, -0.24)] {
                    let point = |r| [x + side * (16.0 + r * dx), y - 24.0 + r * dy];
                    strikes = stage.stroke(
                        strikes,
                        point(4.0),
                        point(4.0 + click * 7.0),
                        1.4,
                        lamp.mix(Color::WHITE, 0.67).alpha(click),
                    );
                }
            }
            root = root.child(stage.ink(strikes));
        }
        if celebration >= 0.0 {
            root = root.child(stage.ink(broadcast(stage, tip, celebration, lamp)));
        }
        root
    })
}

pub(super) fn view(
    phase: Phase,
    status: Status,
    time: f32,
    pulse: f32,
    celebration: f32,
    accent: Color,
) -> El {
    let stage = Stage {
        origin: [94.0, 34.0],
        scale: 1.0,
        size: [384.0, 228.0],
    };
    let motion = Motion::new(phase, status, time, pulse, celebration);
    let center = 192.0 + motion.shuffle + 7.0 * motion.beat;
    let glint = metal(241, 127, 72).mix(accent, 0.18);
    let pilot = puppet(stage, phase, motion, celebration, accent).absolute(0.0, 0.0);
    pilot.children(
        [(-58.0, 42.0), (0.0, 63.0), (58.0, 38.0)].map(|(offset, width)| {
            el().absolute(center + offset - width * 0.5, 195.0)
                .size(width, 8.0)
                .radial_gradient(
                    [0.5, 0.5],
                    [width * 0.5, 4.0],
                    [
                        (0.0, glint.alpha(0.19)),
                        (0.55, glint.alpha(0.07)),
                        (1.0, glint.alpha(0.0)),
                    ],
                )
        }),
    )
}

pub(super) fn portrait() -> El {
    let stage = Stage {
        origin: [1.8, 0.4],
        scale: 0.31,
        size: [64.0, 48.0],
    };
    puppet(
        stage,
        Phase::Work,
        Motion::new(Phase::Work, Status::Ready, 0.0, 0.0, -1.0),
        -1.0,
        metal(251, 168, 83),
    )
}
