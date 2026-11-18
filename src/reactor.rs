mod clack;
mod luma;
pub(crate) use luma::setup_images;

use crate::avatar::Avatar;
use std::cell::{OnceCell, RefCell};
use std::f32::consts::{PI, TAU};
use std::sync::LazyLock;

use image::RgbaImage;
use rune::{Color, El, LineCap, Pixels, Shape, canvas, el};

use crate::timer::{Phase, Status};

const W: usize = 384;
const H: usize = 228;
const CENTER: f32 = W as f32 / 2.0;
const SPRITE: f32 = 185.0;
static TOMATO: LazyLock<RgbaImage> = LazyLock::new(|| {
    image::load_from_memory_with_format(
        include_bytes!("../static/reactor/tomato.png"),
        image::ImageFormat::Png,
    )
    .expect("bundled reactor tomato is a valid PNG")
    .into_rgba8()
});

type Rgb = [f32; 3];

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

fn light(a: Rgb, b: Rgb, strength: f32) -> Rgb {
    std::array::from_fn(|i| a[i] + b[i] * strength)
}

fn rgba(color: Rgb, alpha: f32) -> [u8; 4] {
    let [r, g, b] = color.map(|c| c.clamp(0.0, 255.0) as u8);
    [r, g, b, (255.0 * alpha.clamp(0.0, 1.0)) as u8]
}

fn texture(w: u32, h: u32, bytes: Vec<u8>) -> Pixels {
    let px = Pixels::new();
    px.set(w, h, bytes);
    px
}

fn glow(x: f32, y: f32) -> f32 {
    (-(x * x + y * y)).exp()
}

fn grain(x: u32, y: u32) -> f32 {
    let bits = x.wrapping_mul(198491317) ^ y.wrapping_mul(6542989);
    let noise = (bits ^ (bits >> 13)).wrapping_mul(1274126177);
    (noise >> 24) as f32 / 255.0 - 0.5
}

#[derive(Clone, Copy)]
struct Orbit {
    rx: f32,
    ry: f32,
    slope: f32,
    y: f32,
}

const MAIN: Orbit = Orbit {
    rx: 119.0,
    ry: 32.0,
    slope: -0.22,
    y: 122.0,
};
const AUX: Orbit = Orbit {
    rx: 106.0,
    ry: 39.0,
    slope: 0.38,
    y: 124.0,
};

impl Orbit {
    fn point(self, angle: f32) -> [f32; 2] {
        let (s, c) = angle.sin_cos();
        let x = self.rx * c;
        [CENTER + x, self.y + self.slope * x + self.ry * s]
    }

    // The sign of the projected latitude divides the ring into its near and far sides.
    fn surface(self, x: f32, y: f32) -> (f32, f32, f32) {
        let u = x / self.rx;
        let v = (y - self.y - self.slope * x) / self.ry;
        let dx = u / self.rx - v * self.slope / self.ry;
        let dy = v / self.ry;
        let distance = (u * u + v * v - 1.0) / (2.0 * (dx * dx + dy * dy).sqrt()).max(0.001);
        (distance, u, v)
    }
}

struct Chamber {
    accent: Color,
    backdrop: Pixels,
    near_ring: Pixels,
    lamp: Rgb,
    tomato: OnceCell<TomatoPaint>,
}

struct TomatoPaint {
    fruit: Pixels,
    gloss: Pixels,
    reflection: Pixels,
    lids: [Pixels; 2],
}

thread_local! {
    static CHAMBERS: RefCell<[Option<Chamber>; 3]> = const { RefCell::new([None, None, None]) };
}

impl Chamber {
    fn new(phase: Phase, accent: Color) -> Self {
        let dye = match phase {
            Phase::Work => [255.0, 91.0, 38.0],
            Phase::ShortBreak => [39.0, 245.0, 164.0],
            Phase::LongBreak => [144.0, 124.0, 255.0],
        };
        let tint = [accent.r, accent.g, accent.b].map(|c| rune::color::srgb_encode(c) * 255.0);
        let lamp = mix(dye, tint, 0.28);
        let (backdrop, near_ring) = chamber_surfaces(lamp);
        Self {
            accent,
            backdrop,
            near_ring,
            lamp,
            tomato: OnceCell::new(),
        }
    }
}

fn chamber_surfaces(lamp: Rgb) -> (Pixels, Pixels) {
    const SCALE: usize = 2;
    let width = (W * SCALE) as u32;
    let height = (H * SCALE) as u32;
    let mut back = Vec::with_capacity((width * height * 4) as usize);
    let mut front = Vec::with_capacity(back.capacity());
    for row in 0..height {
        let y = (row as f32 + 0.5) / SCALE as f32;
        for col in 0..width {
            let x = (col as f32 + 0.5) / SCALE as f32 - CENTER;
            let vignette = glow(x / 158.0, (y - 115.0) / 97.0);
            let mut ground = light([12.0, 11.0, 19.0], [16.0, 11.0, 21.0], vignette);
            ground = light(
                ground,
                lamp,
                0.20 * glow((x + 18.0) / 88.0, (y - 124.0) / 72.0),
            );
            ground = light(
                ground,
                [18.0, 47.0, 60.0],
                glow((x - 38.0) / 85.0, (y - 96.0) / 60.0) * 0.40,
            );

            let floor = y - 200.0;
            let reflection = glow(x / 78.0, floor / 11.0);
            ground = light(
                ground,
                lamp,
                reflection * (0.24 + 0.085 * (floor * 2.7).cos()),
            );
            let ellipse = ((x / 87.0).powi(2) + (floor / 9.5).powi(2)).sqrt();
            let rim = (1.0 - (ellipse - 1.0).abs() * 11.0).clamp(0.0, 1.0);
            ground = light(ground, lamp, rim * (0.14 + 0.18 * (x / 26.0).sin().powi(2)));
            let ground_lines = if y > 190.0 {
                let pick = ((x / (y - 158.0) * 2.9).rem_euclid(1.0) - 0.5).abs();
                (1.0 - pick * 52.0).max(0.0) * 4.5
            } else {
                0.0
            };
            let dot = (x + CENTER).rem_euclid(13.0) < 0.9 && y.rem_euclid(13.0) < 0.9;
            let texture =
                grain(col, row) * 6.0 + ground_lines + if dot { 12.0 * vignette } else { 0.0 };
            ground = light(ground, [1.0; 3], texture);

            let mut overlay = [0u8; 4];
            for (orbit, major) in [(AUX, false), (MAIN, true)] {
                let (d, u, v) = orbit.surface(x, y);
                let near = v > 0.0;
                let radius = if major { 2.55 } else { 0.85 };
                let coverage = (0.5 + (radius - d.abs()) * SCALE as f32).clamp(0.0, 1.0);
                if coverage == 0.0 {
                    continue;
                }
                let angle = v.atan2(u);
                let flare = (angle * 2.0 + 1.1).cos().max(0.0).powi(5);
                let silver = [255.0, 226.0, 199.0];
                let mut metal = mix(lamp, silver, 0.14 + 0.72 * flare);
                if major {
                    // A dark wall and two polished edges give the thin ring its physical thickness.
                    let top = glow((d + 1.1) / 0.78, 0.0);
                    let edge = glow((d - 2.0) / 0.4, 0.0);
                    metal = light([19.0, 21.0, 27.0], metal, 0.14 + top * 0.85 + edge * 0.43);
                } else {
                    metal = mix([55.0, 53.0, 71.0], [155.0, 210.0, 224.0], flare * 0.84);
                }
                if near {
                    overlay = rgba(metal, coverage);
                } else {
                    ground = mix(ground, metal, coverage * 0.78);
                }
            }
            back.extend(rgba(ground, 1.0));
            front.extend(overlay);
        }
    }
    (texture(width, height, back), texture(width, height, front))
}

// The LED sockets in the 420 px texture. Their replacement pixels sample the adjacent glass.
const EYE_X: [u32; 2] = [159, 256];
const EYE_Y: u32 = 258;
const LID_W: u32 = 36;
const LID_H: u32 = 58;

fn closed_eye(shell: &RgbaImage, cx: u32) -> Pixels {
    let mut patch = Vec::with_capacity((LID_W * LID_H * 4) as usize);
    let color_at = |x, y| {
        let [r, g, b, _] = shell.get_pixel(x, y).0;
        [r, g, b].map(f32::from)
    };
    let led = color_at(cx, EYE_Y);
    for row in 0..LID_H {
        let y = EYE_Y + row - LID_H / 2;
        let left = color_at(cx - LID_W / 2, y);
        let right = color_at(cx + LID_W / 2, y);
        for col in 0..LID_W {
            let (x, y) = (col as f32 - 18.0, row as f32 - 29.0);
            let glass = mix(left, right, col as f32 / LID_W as f32);
            let line = glow((x.abs() - 4.8).max(0.0) / 2.0, (y - 0.032 * x * x) / 1.65);
            let color = mix(light(glass, led, glow(x / 10.0, y / 5.5) * 0.20), led, line);
            let alpha = ((18.0 - x.abs()) * 0.22).clamp(0.0, 1.0)
                * ((29.0 - y.abs()) * 0.22).clamp(0.0, 1.0);
            patch.extend(rgba(color, alpha));
        }
    }
    texture(LID_W, LID_H, patch)
}

fn fruit_surfaces(phase: Phase, lamp: Rgb) -> TomatoPaint {
    let (w, h) = TOMATO.dimensions();
    let mut shell = (*TOMATO).clone();
    let mut gloss = Vec::with_capacity((w * h * 4) as usize);
    for pixel in shell.pixels_mut() {
        let [r, g, b, alpha] = pixel.0;
        let color = [r, g, b].map(f32::from);
        let red = (color[0] - color[1].max(color[2])).max(0.0);
        if phase != Phase::Work {
            let enamel = lamp.map(|value| value / 255.0);
            let colored = light([color[0] - red, color[1], color[2]], enamel, red);
            pixel.0 = rgba(colored, f32::from(alpha) / 255.0);
        }
        let specular = (color[0].max(color[1]).max(color[2]) / 255.0).powi(2);
        gloss.extend(rgba(
            [255.0, 247.0, 229.0],
            specular * f32::from(alpha) / 255.0 * 0.42,
        ));
    }
    let mut reflection = Vec::new();
    let reflection_h = 60;
    for y in 0..reflection_h {
        let v = y as f32 / reflection_h as f32;
        let source_y = ((0.92 - v * 0.78) * (h - 1) as f32) as u32;
        let ripple = (v * 52.0).sin() * (1.0 + 5.0 * v);
        for x in 0..w {
            let source_x = (x as f32 + ripple).clamp(0.0, (w - 1) as f32) as u32;
            let [r, g, b, a] = shell.get_pixel(source_x, source_y).0;
            let color = mix([r, g, b].map(f32::from), lamp, 0.23);
            let opacity = (1.0 - v).powi(2) * (0.23 + 0.10 * (v * 63.0).cos());
            reflection.extend(rgba(color, a as f32 / 255.0 * opacity));
        }
    }
    let lids = EYE_X.map(|x| closed_eye(&shell, x));
    TomatoPaint {
        fruit: texture(w, h, shell.into_raw()),
        gloss: texture(w, h, gloss),
        reflection: texture(w, reflection_h, reflection),
        lids,
    }
}

fn plate(surface: &Pixels, x: f32, y: f32, w: f32, h: f32) -> El {
    el().absolute(x, y).size(w, h).pixels(surface)
}

fn light_trails(time: f32, pulse: f32, accent: Color, near: bool) -> El {
    let mut shape = Shape::new();
    for (orbit, angle, direction, color) in [
        (MAIN, time * 1.12 + 0.65, 1.0, accent),
        (MAIN, time * 1.12 + PI + 0.65, 1.0, accent),
        (AUX, 3.0 - time * 0.7, -1.0, Color::WHITE),
    ] {
        let angle = angle.rem_euclid(TAU);
        let hot = color.mix(Color::WHITE, 0.68);
        let head = orbit.point(angle);
        for segment in 0..4 {
            let end = angle - direction * segment as f32 * 0.085;
            let start = end - direction * 0.085;
            if (end.sin() >= 0.0) != near {
                continue;
            }
            let points = [
                orbit.point(start),
                orbit.point((start + end) * 0.5),
                orbit.point(end),
            ];
            let strength = 1.0 - segment as f32 / 4.0;
            shape = shape
                .stroke(points.to_vec(), 3.4, color.alpha(strength * 0.34))
                .stroke(points.to_vec(), 1.2, hot.alpha(strength * 0.91));
        }
        if (angle.sin() >= 0.0) == near {
            let radius = 1.2 + pulse * 1.5;
            let [x, y] = head;
            shape = shape
                .line(
                    [x - radius, y],
                    [x + radius, y],
                    radius * 3.0,
                    LineCap::Round,
                    color.alpha(0.21),
                )
                .line([x - radius, y], [x + radius, y], 1.8, LineCap::Round, hot);
        }
    }
    if !near {
        if pulse > 1.0 {
            let age = 2.0 - pulse;
            for index in 0..10 {
                let (s, c) = (TAU * index as f32 / 10.0 + 0.18).sin_cos();
                let radius = 83.0 + age * 37.0;
                let length = 15.0 * (1.0 - age);
                let ray = |r| [CENTER + c * r, 104.0 + s * r * 0.74];
                shape = shape.line(
                    ray(radius),
                    ray(radius + length),
                    1.5,
                    LineCap::Round,
                    accent.mix(Color::WHITE, 0.64).alpha(0.85 * (1.0 - age)),
                );
            }
        }
        for index in 0..14 {
            let i = index as f32;
            let angle = i * 2.4 + time * 0.035;
            let (s, c) = angle.sin_cos();
            let x = CENTER + c * (90.0 + 49.0 * (i * 0.9).sin().abs());
            let y = 107.0 + s * (64.0 + 24.0 * (i * 1.2).cos().abs());
            let alpha = 0.18 + 0.34 * (time * 0.8 + i * 3.6).cos().powi(2);
            shape = shape.line(
                [x - 0.35, y],
                [x + 0.35, y],
                1.2,
                LineCap::Round,
                accent.mix(Color::WHITE, 0.34).alpha(alpha),
            );
        }
    }
    canvas(shape).absolute(0.0, 0.0).size(W as f32, H as f32)
}

fn sight_marks(accent: Color, pulse: f32) -> El {
    let ink = Color::WHITE.alpha(0.18);
    let mut sight = Shape::new();
    for sign in [-1.0, 1.0] {
        let x = CENTER + 176.0 * sign;
        for tick in -3i32..=3 {
            let y = 115.0 + tick as f32 * 5.0;
            let len = if tick == 0 { 8.0 } else { 3.0 };
            let color = if tick == 0 {
                accent.alpha(0.54 + pulse.min(1.0) * 0.40)
            } else {
                ink
            };
            sight = sight.line([x, y], [x - sign * len, y], 0.8, LineCap::Butt, color);
        }
        for y in [14.0, 214.0] {
            let inside_y = if y < 20.0 { y + 5.0 } else { y - 5.0 };
            sight = sight.stroke(vec![[x, inside_y], [x, y], [x - sign * 9.0, y]], 0.8, ink);
        }
    }
    canvas(sight).absolute(0.0, 0.0).size(W as f32, H as f32)
}

pub fn portrait(avatar: Avatar) -> El {
    match avatar {
        Avatar::Tomato => {
            thread_local! {
                static PORTRAIT: Pixels = texture(TOMATO.width(), TOMATO.height(), TOMATO.as_raw().clone());
            }
            PORTRAIT.with(|pixels| {
                el().size(64.0, 48.0)
                    .child(plate(pixels, 8.0, 0.0, 48.0, 48.0))
            })
        }
        Avatar::Clack => clack::portrait(),
        Avatar::Luma => luma::portrait(),
    }
}

fn tomato(paint: &TomatoPaint, status: Status, time: f32, pulse: f32, celebration: f32) -> El {
    let tap_age = (2.0 - pulse).clamp(0.0, 1.0);
    let spring = (tap_age * TAU * 1.7).sin() * (1.0 - tap_age).powi(2);
    let bounce = (tap_age * PI).sin() * (1.0 - tap_age);
    let hop = (celebration.max(0.0) * 4.0 * PI).sin().abs() * (1.0 - celebration.max(0.0));
    let fruit_w = SPRITE * (1.0 + 0.13 * spring - 0.07 * hop);
    let fruit_h = SPRITE * (1.0 - 0.15 * spring + 0.05 * hop);
    let fruit_x = CENTER - fruit_w * 0.5
        + 1.8 * (time * 0.65).sin()
        + 8.0 * (TAU * celebration.max(0.0) * 2.0).sin();
    let fruit_y = 11.0 + (SPRITE - fruit_h) * 0.78 + 3.7 * (time * 1.1).sin()
        - 2.8 * pulse.min(1.0)
        - 19.0 * bounce
        - 16.0 * hop;
    let scan = fruit_w * (0.50 + 0.44 * (time * 0.61 - 0.8).sin());
    let lamp_strength = if status == Status::Paused { 0.23 } else { 0.68 };
    let gloss = el()
        .absolute(fruit_x + scan - 13.0, fruit_y)
        .size(26.0, fruit_h)
        .clip_fade(12.5, 0.0, 12.5, 0.0)
        .child(plate(&paint.gloss, 13.0 - scan, 0.0, fruit_w, fruit_h))
        .opacity(lamp_strength);
    let blink = (1.0 - ((time + 3.6).rem_euclid(6.4) - 0.18).abs() / 0.18).clamp(0.0, 1.0);
    let sleepy = if status == Status::Paused { 1.0 } else { blink };
    let wink = (tap_age * PI).sin().max(0.0).sqrt();
    let eyes = [sleepy, sleepy.max(wink).max(hop.powi(3))];
    el().absolute(0.0, 0.0)
        .size(W as f32, H as f32)
        .child(plate(
            &paint.reflection,
            fruit_x + 4.0,
            195.0 - (fruit_y - 11.0) * 0.2,
            fruit_w - 8.0,
            31.0,
        ))
        .child(plate(&paint.fruit, fruit_x, fruit_y, fruit_w, fruit_h))
        .children(
            eyes.into_iter()
                .enumerate()
                .filter(|(_, shut)| *shut > 0.01)
                .map(|(i, shut)| {
                    let sx = fruit_w / TOMATO.width() as f32;
                    let sy = fruit_h / TOMATO.height() as f32;
                    plate(
                        &paint.lids[i],
                        fruit_x + (EYE_X[i] - LID_W / 2) as f32 * sx,
                        fruit_y + (EYE_Y - LID_H / 2) as f32 * sy,
                        LID_W as f32 * sx,
                        LID_H as f32 * sy,
                    )
                    .opacity(shut)
                }),
        )
        .child(gloss)
}

// Two side cannons fire paper streamers across the chamber. The timer sits below the glass.
fn confetti(progress: f32, accent: Color) -> El {
    let palette = [
        accent,
        rune::color::oklch(0.85, 0.14, 82.0),
        rune::color::oklch(0.92, 0.09, 188.0),
        rune::color::oklch(0.78, 0.16, 351.0),
    ];
    let mut shape = Shape::new();
    for index in 0..72 {
        let n = index as f32;
        let hash = |salt: u32| grain(index, salt) + 0.5;
        let t = (progress * 1.32 - hash(3) * 0.24).max(0.0);
        let side = if index % 2 == 0 { 1.0 } else { -1.0 };
        let x = CENTER
            + side * (-167.0 + (95.0 + 230.0 * hash(8)) * t)
            + (t * 11.0 + n).sin() * t * 6.0;
        let y = 193.0 - (245.0 + 400.0 * hash(5)) * t + 490.0 * t * t;
        if t == 0.0 || !(12.0..215.0).contains(&y) || !(12.0..372.0).contains(&x) {
            continue;
        }
        let spin = n + t * (9.0 + hash(1) * 19.0);
        let dx = (2.3 + hash(4) * 2.5) * spin.cos();
        let dy = 3.3 * (spin * 0.67).sin();
        let color = palette[index as usize % palette.len()].alpha((1.0 - progress).min(0.2) * 4.8);
        shape = shape.line(
            [x - dx, y - dy],
            [x + dx, y + dy],
            1.7 + hash(6),
            LineCap::Butt,
            color,
        );
    }
    canvas(shape).absolute(0.0, 0.0).size(W as f32, H as f32)
}

pub fn view(
    avatar: Avatar,
    phase: Phase,
    status: Status,
    time: f32,
    pulse: f32,
    celebration: f32,
    accent: Color,
) -> El {
    CHAMBERS.with_borrow_mut(|cache| {
        let slot = &mut cache[match phase {
            Phase::Work => 0,
            Phase::ShortBreak => 1,
            Phase::LongBreak => 2,
        }];
        if slot.as_ref().is_none_or(|chamber| chamber.accent != accent) {
            *slot = Some(Chamber::new(phase, accent));
        }
        let chamber = slot.as_ref().expect("initialized chamber");
        let pilot = match avatar {
            Avatar::Tomato => tomato(
                chamber
                    .tomato
                    .get_or_init(|| fruit_surfaces(phase, chamber.lamp)),
                status,
                time,
                pulse,
                celebration,
            ),
            Avatar::Clack => clack::view(phase, status, time, pulse, celebration, accent),
            Avatar::Luma => luma::view(phase, status, time, pulse, celebration, accent),
        };
        el().key("reactor")
            .size(W as f32, H as f32)
            .a11y_hidden()
            .child(plate(&chamber.backdrop, 0.0, 0.0, W as f32, H as f32).radius(10.0))
            .child(light_trails(time, pulse, accent, false))
            .child(pilot)
            .child(plate(&chamber.near_ring, 0.0, 0.0, W as f32, H as f32))
            .child(light_trails(time, pulse, accent, true))
            .child(sight_marks(accent, pulse))
            .children((celebration >= 0.0).then(|| confetti(celebration, accent)))
            .child(
                el().absolute(0.0, 0.0)
                    .size(W as f32, H as f32)
                    .radius(10.0)
                    .border_gradient(1.0, Color::BLACK.alpha(0.78), Color::WHITE.alpha(0.16)),
            )
    })
}
