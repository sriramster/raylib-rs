use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use std::thread;
use std::time::Duration;

use chrono::prelude::*;

use raylib::core::color::Color;
use raylib::core::drawing::RaylibDraw;
use raylib::core::math::Vector2;
use raylib::ffi::*;

const SEGMENT_LEN: f32     = 60.0;
const SEGMENT_THICK: f32   = 20.0;
const OFFSET_Y_ADJUST: f32 = SEGMENT_THICK * 0.3;

const DIGIT_STRIDE: f32 = SEGMENT_THICK * 2.0 + SEGMENT_LEN + SEGMENT_THICK;
const COLON_WIDTH: f32  = SEGMENT_THICK;

const DIGIT_SEGMENTS: [u8; 10] = [
    0b00111111, // 0
    0b00000110, // 1
    0b01011011, // 2
    0b01001111, // 3
    0b01100110, // 4
    0b01101101, // 5
    0b01111101, // 6
    0b00000111, // 7
    0b01111111, // 8
    0b01101111, // 9
];

#[derive(Clone)]
struct ClockState {
    hour:   Arc<AtomicU32>,
    minute: Arc<AtomicU32>,
    second: Arc<AtomicU32>,
}

impl ClockState {
    fn new() -> Self {
        Self {
            hour:   Arc::new(AtomicU32::new(0)),
            minute: Arc::new(AtomicU32::new(0)),
            second: Arc::new(AtomicU32::new(0)),
        }
    }

    fn snapshot(&self) -> (u32, u32, u32) {
        (
            self.hour.load(Relaxed),
            self.minute.load(Relaxed),
            self.second.load(Relaxed),
        )
    }

    fn write(&self, now: DateTime<Local>) {
        self.hour.store(now.hour(),     Relaxed);
        self.minute.store(now.minute(), Relaxed);
        self.second.store(now.second(), Relaxed);
    }
}

fn spawn_time_thread(state: ClockState) {
    thread::spawn(move || {
        loop {
            state.write(Local::now());
            thread::sleep(Duration::from_millis(100));
        }
    });
}

fn segment(context: &mut impl RaylibDraw, center: Vector2, vertical: bool, color: Color) {
    let mut pts: [Vector2; 6] = if !vertical {
        [
            Vector2::new(center.x - SEGMENT_LEN / 2.0 - SEGMENT_THICK / 2.0, center.y),
            Vector2::new(center.x - SEGMENT_LEN / 2.0, center.y + SEGMENT_THICK / 2.0),
            Vector2::new(center.x - SEGMENT_LEN / 2.0, center.y - SEGMENT_THICK / 2.0),
            Vector2::new(center.x + SEGMENT_LEN / 2.0, center.y + SEGMENT_THICK / 2.0),
            Vector2::new(center.x + SEGMENT_LEN / 2.0, center.y - SEGMENT_THICK / 2.0),
            Vector2::new(center.x + SEGMENT_LEN / 2.0 + SEGMENT_THICK / 2.0, center.y),
        ]
    } else {
        [
            Vector2::new(center.x, center.y - SEGMENT_LEN / 2.0 - SEGMENT_THICK / 2.0),
            Vector2::new(center.x - SEGMENT_THICK / 2.0, center.y - SEGMENT_LEN / 2.0),
            Vector2::new(center.x + SEGMENT_THICK / 2.0, center.y - SEGMENT_LEN / 2.0),
            Vector2::new(center.x - SEGMENT_THICK / 2.0, center.y + SEGMENT_LEN / 2.0),
            Vector2::new(center.x + SEGMENT_THICK / 2.0, center.y + SEGMENT_LEN / 2.0),
            Vector2::new(center.x, center.y + SEGMENT_LEN / 2.0 + SEGMENT_THICK / 2.0),
        ]
    };
    context.draw_triangle_strip(&mut pts, color);
}

fn seven(context: &mut impl RaylibDraw, pos: Vector2, mask: u8, on: Color, off: Color) {
    let c = |bit| if mask & bit != 0 { on } else { off };

    // A – top horizontal
    segment(context,
        Vector2::new(pos.x + SEGMENT_THICK + SEGMENT_LEN / 2.0,
                     pos.y + SEGMENT_THICK),
        false, c(0b00000001));

    // B – upper-right vertical
    segment(context,
        Vector2::new(pos.x + SEGMENT_THICK + SEGMENT_LEN + SEGMENT_THICK / 2.0,
                     pos.y + 2.0 * SEGMENT_THICK + SEGMENT_LEN / 2.0 - OFFSET_Y_ADJUST),
        true, c(0b00000010));

    // C – lower-right vertical
    segment(context,
        Vector2::new(pos.x + SEGMENT_THICK + SEGMENT_LEN + SEGMENT_THICK / 2.0,
                     pos.y + 4.0 * SEGMENT_THICK + SEGMENT_LEN + SEGMENT_LEN / 2.0 - 3.0 * OFFSET_Y_ADJUST),
        true, c(0b00000100));

    // D – bottom horizontal
    segment(context,
        Vector2::new(pos.x + SEGMENT_THICK + SEGMENT_LEN / 2.0,
                     pos.y + 5.0 * SEGMENT_THICK + 2.0 * SEGMENT_LEN - 4.0 * OFFSET_Y_ADJUST),
        false, c(0b00001000));

    // E – lower-left vertical
    segment(context,
        Vector2::new(pos.x + SEGMENT_THICK / 2.0,
                     pos.y + 4.0 * SEGMENT_THICK + SEGMENT_LEN + SEGMENT_LEN / 2.0 - 3.0 * OFFSET_Y_ADJUST),
        true, c(0b00010000));

    // F – upper-left vertical
    segment(context,
        Vector2::new(pos.x + SEGMENT_THICK / 2.0,
                     pos.y + 2.0 * SEGMENT_THICK + SEGMENT_LEN / 2.0 - OFFSET_Y_ADJUST),
        true, c(0b00100000));

    // G – middle horizontal
    segment(context,
        Vector2::new(pos.x + SEGMENT_THICK + SEGMENT_LEN / 2.0,
                     pos.y + 3.0 * SEGMENT_THICK + SEGMENT_LEN - 2.0 * OFFSET_Y_ADJUST),
        false, c(0b01000000));
}

fn digit(context: &mut impl RaylibDraw, pos: Vector2, value: u32, on: Color, off: Color) {
    seven(context, pos, DIGIT_SEGMENTS[value as usize], on, off);
}

fn colon(context: &mut impl RaylibDraw, x: f32, pos: Vector2, color: Color) {
    context.draw_circle(x as i32, (pos.y + 70.0)  as i32, 12.0, color);
    context.draw_circle(x as i32, (pos.y + 150.0) as i32, 12.0, color);
}

fn digital(context: &mut impl RaylibDraw, pos: Vector2, h: u32, m: u32, s: u32) {
    let on       = Color::RED;
    let off      = Color::alpha(&Color::LIGHTGRAY, 0.3);
    let colon_on = if s % 2 != 0 { on } else { off };

    let dx = |n: u32, colons_before: u32| -> Vector2 {
        Vector2::new(
            pos.x + n as f32 * DIGIT_STRIDE + colons_before as f32 * COLON_WIDTH,
            pos.y,
        )
    };
    let cx = |n: u32| pos.x + n as f32 * DIGIT_STRIDE;

    digit(context, dx(0, 0), h / 10, on, off);
    digit(context, dx(1, 0), h % 10, on, off);
    colon(context, cx(2),             pos, colon_on);
    digit(context, dx(2, 1), m / 10, on, off);
    digit(context, dx(3, 1), m % 10, on, off);
    colon(context, cx(4) + COLON_WIDTH, pos, colon_on);
    digit(context, dx(4, 2), s / 10, on, off);
    digit(context, dx(5, 2), s % 10, on, off);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut rl, thread) = raylib::init()
        .size(800, 450)
        .title("Clock")
        .build();

    unsafe { SetTargetFPS(60); }

    let state = ClockState::new();
    spawn_time_thread(state.clone());

    let position = Vector2::new(30.0, 60.0);

    while !rl.window_should_close() {
        let (h, m, s) = state.snapshot();

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::RAYWHITE);

        digital(&mut d, position, h, m, s);

        let owned_str = unsafe {
            let fmt = CString::new("%02i:%02i:%02i").unwrap();
            let time = TextFormat(fmt.as_ptr() as *const i8, h, m, s);
            CStr::from_ptr(time as *const c_char).to_string_lossy().into_owned()
        };

        d.draw_text(
            &owned_str,
            d.get_screen_width() / 2 - d.measure_text(&owned_str, 150) / 2,
            300, 150, Color::BLACK,
        );
    }
    Ok(())
}
