//! Native direct-mode adapter. The teacher controls the student's computer.
use super::{
    direct_ui_model::pasted_invitation,
    gui::{Canvas, Rect},
    platform,
    video::{AdaptiveVideo, Latest, Viewport},
    Result,
};
use arboard::Clipboard;
use enigo::{Button, Direction, Enigo, Mouse, Settings};
use image::{
    codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage, ImageFormat, ImageReader,
};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use serde_json::json;
use std::{
    collections::HashSet,
    io::Cursor,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use xcap::Monitor;
use yt_direct_core::{
    start_student, start_teacher, Event, Frame, InputCommand, Session, TeacherOptions,
};

const W: usize = 800;
const H: usize = 510;
const WHITE: u32 = 0xf0f5ff;
const MUTED: u32 = 0x9daec5;
const BLUE: u32 = 0x397bef;
const RED: u32 = 0x973b4c;
const GREEN: u32 = 0x20876a;
const PRIMARY: Rect = Rect {
    x: 70,
    y: 184,
    w: 660,
    h: 52,
};
const SECONDARY: Rect = Rect {
    x: 70,
    y: 318,
    w: 660,
    h: 48,
};
const BOTTOM: Rect = Rect {
    x: 70,
    y: 394,
    w: 660,
    h: 48,
};

#[derive(PartialEq)]
enum Page {
    Home,
    Preparing,
    TeacherWaiting,
    Connecting,
    Approval,
    Sharing,
    Status,
    About,
}

struct StudentWorker {
    run: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
    session: Arc<Session>,
}
impl Drop for StudentWorker {
    fn drop(&mut self) {
        self.run.store(false, SeqCst);
        self.session.stop();
        if let Some(worker) = self.thread.take() {
            let _ = worker.join();
        }
    }
}

/// Startup and permission failure occurs before capture/input objects are created.
fn start_capture(session: Arc<Session>, error: Arc<Latest<String>>) -> StudentWorker {
    let run = Arc::new(AtomicBool::new(true));
    let running = run.clone();
    let sharing = session.clone();
    let worker = thread::spawn(move || {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            capture_student(&sharing, &running)
        })) {
            Ok(Err(reason)) => error.put(reason.to_string()),
            Err(_) => error.put("The native sharing worker stopped unexpectedly.".into()),
            _ => {}
        }
        running.store(false, SeqCst);
        sharing.stop();
    });
    StudentWorker {
        run,
        thread: Some(worker),
        session,
    }
}

struct HeldInput {
    native: Enigo,
    keys: HashSet<String>,
    buttons: HashSet<String>,
}
impl HeldInput {
    fn new() -> Result<Self> {
        Ok(Self {
            native: Enigo::new(&Settings::default())?,
            keys: HashSet::new(),
            buttons: HashSet::new(),
        })
    }
    fn apply(&mut self, command: InputCommand) {
        let value = match command {
            InputCommand::Move { x, y } => json!({"kind":"move","x":x,"y":y}),
            InputCommand::Button {
                x,
                y,
                button,
                pressed,
            } => {
                if pressed {
                    self.buttons.insert(button.clone());
                } else {
                    self.buttons.remove(&button);
                }
                json!({"kind":"button","x":x,"y":y,"button":button,"pressed":pressed})
            }
            InputCommand::Scroll { delta } => json!({"kind":"scroll","delta":delta}),
            InputCommand::Key { name, pressed } => {
                json!({"kind":"key","name":name,"pressed":pressed})
            }
        };
        super::apply_input(&mut self.native, value, &mut self.keys);
    }
}
impl Drop for HeldInput {
    fn drop(&mut self) {
        for key in self.keys.drain() {
            super::press_key(&mut self.native, &key, false);
        }
        for name in self.buttons.drain() {
            let button = match name.as_str() {
                "left" => Button::Left,
                "right" => Button::Right,
                _ => Button::Middle,
            };
            let _ = self.native.button(button, Direction::Release);
        }
    }
}

fn capture_student(session: &Session, run: &AtomicBool) -> Result<()> {
    if !session.active.load(SeqCst) || !session.approved.load(SeqCst) {
        return Err("Student approval is required before capture.".into());
    }
    thread::scope(|scope| {
        let _cleanup = StopSession(session);
        let input = scope.spawn(|| input_worker(session, run));
        let result = capture_frames(session, run);
        session.stop();
        let input_result = input
            .join()
            .map_err(|_| "Native input worker stopped unexpectedly.")?;
        result?;
        input_result.map_err(Into::into)
    })
}

struct StopSession<'a>(&'a Session);
impl Drop for StopSession<'_> {
    fn drop(&mut self) {
        self.0.stop();
    }
}

fn input_worker(session: &Session, run: &AtomicBool) -> std::result::Result<(), String> {
    let _cleanup = StopSession(session);
    let mut input = HeldInput::new().map_err(|e| e.to_string())?;
    while run.load(SeqCst) && session.active.load(SeqCst) && session.approved.load(SeqCst) {
        if let Some(command) = session.try_input() {
            session.with_consent(|| input.apply(command));
        } else {
            thread::sleep(Duration::from_millis(1));
        }
    }
    Ok(())
}

fn capture_frames(session: &Session, run: &AtomicBool) -> Result<()> {
    let monitor = Monitor::all()?
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .ok_or("No student display found.")?;
    let mut policy = AdaptiveVideo::default();
    let mut previous = Vec::new();
    let mut sequence = 0;
    let mut next = Instant::now();
    while run.load(SeqCst) && session.active.load(SeqCst) && session.approved.load(SeqCst) {
        if Instant::now() >= next {
            let began = Instant::now();
            let shot = monitor.capture_image()?;
            let changed = previous != *shot.as_raw();
            if changed {
                previous.clone_from(shot.as_raw());
                let image = DynamicImage::ImageRgba8(shot)
                    .resize(1920, 1080, FilterType::Triangle)
                    .to_rgb8();
                let (width, height) = image.dimensions();
                let mut jpeg = Vec::new();
                JpegEncoder::new_with_quality(&mut jpeg, policy.quality()).encode_image(&image)?;
                sequence += 1;
                if run.load(SeqCst) && session.active.load(SeqCst) && session.approved.load(SeqCst)
                {
                    let _ = session.publish_frame(Frame {
                        sequence,
                        width,
                        height,
                        jpeg,
                    });
                }
            }
            policy.observe(
                began.elapsed(),
                Duration::from_millis(session.stats.last_rtt_ms.load(SeqCst)),
            );
            next = began + policy.interval();
        }
        thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}

fn clicked(window: &Window, previous: &mut bool) -> Option<(usize, usize)> {
    let down = window.get_mouse_down(MouseButton::Left);
    let edge = down && !*previous;
    *previous = down;
    edge.then(|| window.get_mouse_pos(MouseMode::Clamp))
        .flatten()
        .map(|(x, y)| (x as usize, y as usize))
}
fn hit(click: Option<(usize, usize)>, rect: Rect) -> bool {
    click.is_some_and(|(x, y)| rect.contains(x, y))
}
fn paragraph(canvas: &mut Canvas, y: usize, value: &str) {
    let mut line = String::new();
    let mut row = y;
    for word in value.split_whitespace() {
        if line.len() + word.len() + 1 > 78 {
            canvas.text(70, row, &line, MUTED, 1);
            row += 20;
            line.clear();
            if row > 370 {
                break;
            }
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    canvas.text(70, row, &line, MUTED, 1);
}

pub(super) fn launch() -> Result<()> {
    let mut window = Window::new(platform::PRODUCT_NAME, W, H, WindowOptions::default())?;
    window.set_target_fps(60);
    let mut canvas = Canvas::new();
    let mut clipboard = Clipboard::new().ok();
    let mut page = Page::Home;
    let mut session: Option<Arc<Session>> = None;
    let mut worker: Option<StudentWorker> = None;
    let errors = Arc::new(Latest::default());
    let mut teacher = false;
    let mut invitation = String::new();
    let mut pasted = String::new();
    let mut status = String::new();
    let mut mouse = false;
    let mut copied = false;
    let mut peer_connected = false;
    while window.is_open() {
        let click = clicked(&window, &mut mouse);
        if let Some(error) = errors.take() {
            status = error;
            page = Page::Status;
            if let Some(s) = &session {
                s.stop();
            }
        }
        if let Some(s) = &session {
            for _ in 0..32 {
                let Some(event) = s.try_event() else {
                    break;
                };
                match event {
                    Event::Invitation(code) => {
                        invitation = code;
                        page = Page::TeacherWaiting;
                    }
                    Event::Connected => {
                        if teacher {
                            peer_connected = true;
                        } else {
                            page = Page::Approval;
                        }
                    }
                    Event::Approved if teacher => {
                        let viewer_session = s.clone();
                        drop(window);
                        let result = teacher_viewer(viewer_session.clone());
                        viewer_session.stop();
                        return result;
                    }
                    Event::Error(message) => {
                        status = message;
                        page = Page::Status;
                    }
                    Event::Ended(message) if page != Page::Home && page != Page::Status => {
                        status = message;
                        page = Page::Status;
                    }
                    _ => {}
                }
            }
        }
        canvas.clear();
        canvas.text(70, 46, "YT REMOTE 3.0", WHITE, 3);
        canvas.text(70, 94, "Teacher controls the STUDENT computer.", MUTED, 1);
        match page {
            Page::Home => {
                let about = Rect {
                    x: 570,
                    y: 118,
                    w: 160,
                    h: 24,
                };
                canvas.button(about, "LICENSE", BLUE);
                canvas.text(
                    70,
                    148,
                    "TEACHER: Create an invitation for your student",
                    WHITE,
                    1,
                );
                canvas.button(PRIMARY, "GENERATE CODE (TEACHER)", BLUE);
                canvas.text(
                    70,
                    270,
                    "STUDENT: Paste the code provided by your teacher",
                    WHITE,
                    1,
                );
                canvas.button(SECONDARY, "PASTE CODE", BLUE);
                if !pasted.is_empty() {
                    canvas.text(
                        70,
                        375,
                        "Invitation loaded. Nothing is shared yet.",
                        MUTED,
                        1,
                    );
                }
                canvas.button(BOTTOM, "CONNECT", GREEN);
                if hit(click, about) {
                    page = Page::About;
                } else if hit(click, PRIMARY) {
                    teacher = true;
                    copied = false;
                    peer_connected = false;
                    invitation.clear();
                    session = Some(Arc::new(start_teacher(TeacherOptions::default())));
                    page = Page::Preparing;
                } else if hit(click, SECONDARY)
                    || ((window.is_key_down(Key::LeftCtrl)
                        || window.is_key_down(Key::RightCtrl)
                        || window.is_key_down(Key::LeftSuper)
                        || window.is_key_down(Key::RightSuper))
                        && window.is_key_pressed(Key::V, KeyRepeat::No))
                {
                    match clipboard
                        .as_mut()
                        .ok_or("Clipboard is unavailable. Please restart the app.")
                        .and_then(|c| c.get_text().map_err(|_| "Cannot read the clipboard."))
                        .and_then(|raw| pasted_invitation(&raw))
                    {
                        Ok(code) => pasted = code,
                        Err(message) => {
                            status = message.into();
                            page = Page::Status;
                        }
                    }
                } else if hit(click, BOTTOM) && !pasted.is_empty() {
                    match start_student(&pasted) {
                        Ok(s) => {
                            teacher = false;
                            session = Some(Arc::new(s));
                            page = Page::Connecting;
                            pasted.clear();
                        }
                        Err(message) => {
                            status = message;
                            page = Page::Status;
                        }
                    }
                }
            }
            Page::Preparing | Page::Connecting => {
                canvas.text(
                    70,
                    170,
                    if teacher {
                        "PREPARING INVITATION"
                    } else {
                        "CONNECTING TO TEACHER"
                    },
                    WHITE,
                    2,
                );
                paragraph(
                    &mut canvas,
                    230,
                    "No screen or keyboard access is permitted yet. You can cancel at any time.",
                );
                canvas.button(BOTTOM, "CANCEL", RED);
                if hit(click, BOTTOM) {
                    if let Some(s) = session.take() {
                        s.stop();
                    }
                    page = Page::Home;
                }
            }
            Page::TeacherWaiting => {
                canvas.text(70, 146, "YOUR STUDENT'S SCREEN WILL APPEAR HERE", WHITE, 1);
                paragraph(&mut canvas,260,"Copy the invitation and send it privately to your student. The student pastes it into YT Remote and explicitly approves screen, keyboard and mouse control. This invitation expires after ten minutes.");
                canvas.button(PRIMARY, "COPY CODE", BLUE);
                canvas.button(BOTTOM, "CANCEL", RED);
                if copied {
                    canvas.text(70, 244, "Copied. Send this invitation privately.", GREEN, 1);
                }
                if peer_connected {
                    canvas.text(
                        70,
                        370,
                        "Student connected. Waiting for student approval.",
                        GREEN,
                        1,
                    );
                }
                if hit(click, PRIMARY) {
                    if clipboard
                        .as_mut()
                        .is_some_and(|c| c.set_text(invitation.clone()).is_ok())
                    {
                        copied = true;
                    } else {
                        status = "Cannot copy the invitation.".into();
                        page = Page::Status;
                    }
                }
                if hit(click, BOTTOM) {
                    if let Some(s) = session.take() {
                        s.stop();
                    }
                    invitation.clear();
                    page = Page::Home;
                }
            }
            Page::Approval => {
                canvas.text(70, 146, "STUDENT PERMISSION REQUIRED", WHITE, 2);
                paragraph(&mut canvas,252,"The teacher will see YOUR screen and control YOUR mouse and keyboard. Programs and edits will run on YOUR computer. Allow only if you trust the teacher who provided this code.");
                canvas.button(PRIMARY, "ALLOW SCREEN & KEYBOARD", GREEN);
                canvas.button(BOTTOM, "DENY", RED);
                if hit(click, PRIMARY) {
                    match platform::ensure_capture_permissions() {
                        Ok(()) => {
                            if let Some(s) = &session {
                                if s.allow() {
                                    worker = Some(start_capture(s.clone(), errors.clone()));
                                    page = Page::Sharing;
                                } else {
                                    status =
                                        "The connection ended. Ask for a new invitation.".into();
                                    page = Page::Status;
                                }
                            }
                        }
                        Err(message) => {
                            if let Some(s) = &session {
                                s.deny();
                            }
                            status = message;
                            page = Page::Status;
                        }
                    }
                } else if hit(click, BOTTOM) {
                    if let Some(s) = session.take() {
                        s.deny();
                    }
                    page = Page::Home;
                }
            }
            Page::Sharing => {
                canvas.text(70, 150, "YOUR SCREEN IS BEING SHARED", WHITE, 2);
                paragraph(&mut canvas,264,"Your teacher controls your computer. Stop immediately with this button or Escape.");
                canvas.button(PRIMARY, "STOP SHARING NOW", RED);
                if hit(click, PRIMARY) || window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                    if let Some(s) = session.take() {
                        s.stop();
                    }
                    worker.take();
                    page = Page::Home;
                }
            }
            Page::Status => {
                if let Some(s) = session.take() {
                    s.stop();
                }
                worker.take();
                invitation.clear();
                canvas.text(70, 150, "CONNECTION STATUS", WHITE, 2);
                paragraph(&mut canvas, 220, &status);
                canvas.button(BOTTOM, "BACK", BLUE);
                if hit(click, BOTTOM) {
                    page = Page::Home;
                }
            }
            Page::About => {
                canvas.text(70, 150, "LICENSE AND SOURCE", WHITE, 2);
                paragraph(&mut canvas,210,"YT Remote 3.0 includes modified RustDesk QoS code. This desktop derivative is licensed under GNU AGPL version 3. There is no warranty. Redistribution is permitted under that license. See COPYING and THIRD_PARTY_NOTICES.md supplied with this application. SOURCE.txt identifies the exact corresponding source and build instructions.");
                canvas.text(
                    70,
                    358,
                    "https://github.com/klckylmz-art/yt-remote-2",
                    MUTED,
                    1,
                );
                canvas.button(BOTTOM, "BACK", BLUE);
                if hit(click, BOTTOM) {
                    page = Page::Home;
                }
            }
        }
        canvas.text(
            70,
            477,
            "EXPERIMENTAL - native/device/network testing is still required",
            MUTED,
            1,
        );
        if let Err(error) = window.update_with_buffer(&canvas.pixels, W, H) {
            if let Some(s) = &session {
                s.stop();
            }
            worker.take();
            return Err(error.into());
        }
    }
    if let Some(s) = session {
        s.stop();
    }
    worker.take();
    Ok(())
}

struct Rendered {
    pixels: Vec<u32>,
    viewport: Viewport,
    sequence: u64,
}

fn decode_frame(frame: Frame, width: usize, height: usize) -> Result<Rendered> {
    let dimensions =
        ImageReader::with_format(Cursor::new(&frame.jpeg), ImageFormat::Jpeg).into_dimensions()?;
    if dimensions != (frame.width, frame.height)
        || u64::from(dimensions.0) * u64::from(dimensions.1) > 1920 * 1080
    {
        return Err("Remote frame dimensions exceed the negotiated bounds.".into());
    }
    let mut reader = ImageReader::with_format(Cursor::new(&frame.jpeg), ImageFormat::Jpeg);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode()?;
    let viewport =
        Viewport::fit(frame.width, frame.height, width, height).ok_or("Invalid viewer size")?;
    let image = image
        .resize_exact(
            viewport.width as u32,
            viewport.height as u32,
            FilterType::Triangle,
        )
        .to_rgb8();
    let mut pixels = vec![0; width * height];
    for (x, y, pixel) in image.enumerate_pixels() {
        pixels[(y as usize + viewport.y) * width + x as usize + viewport.x] =
            (u32::from(pixel[0]) << 16) | (u32::from(pixel[1]) << 8) | u32::from(pixel[2]);
    }
    Ok(Rendered {
        pixels,
        viewport,
        sequence: frame.sequence,
    })
}

fn key_name(key: Key) -> Option<String> {
    let name = match key {
        Key::LeftCtrl | Key::RightCtrl => "control".into(),
        Key::LeftShift | Key::RightShift => "shift".into(),
        Key::LeftAlt | Key::RightAlt => "alt".into(),
        Key::LeftSuper | Key::RightSuper => "meta".into(),
        Key::Enter => "enter".into(),
        Key::Backspace => "backspace".into(),
        Key::Tab => "tab".into(),
        Key::Space => "space".into(),
        Key::Delete => "delete".into(),
        Key::Home => "home".into(),
        Key::End => "end".into(),
        Key::PageUp => "pageup".into(),
        Key::PageDown => "pagedown".into(),
        Key::Left => "left".into(),
        Key::Right => "right".into(),
        Key::Up => "up".into(),
        Key::Down => "down".into(),
        Key::Equal => "=".into(),
        Key::Minus => "-".into(),
        Key::Comma => ",".into(),
        Key::Period => ".".into(),
        Key::Slash => "/".into(),
        Key::Backslash => "\\".into(),
        Key::Semicolon => ";".into(),
        Key::Apostrophe => "'".into(),
        Key::Backquote => "`".into(),
        Key::LeftBracket => "[".into(),
        Key::RightBracket => "]".into(),
        Key::CapsLock => "capslock".into(),
        Key::F1 => "f1".into(),
        Key::F2 => "f2".into(),
        Key::F3 => "f3".into(),
        Key::F4 => "f4".into(),
        Key::F5 => "f5".into(),
        Key::F6 => "f6".into(),
        Key::F7 => "f7".into(),
        Key::F8 => "f8".into(),
        Key::F9 => "f9".into(),
        Key::F10 => "f10".into(),
        Key::F11 => "f11".into(),
        Key::F12 => "f12".into(),
        _ => {
            let debug = format!("{key:?}");
            let value = debug.strip_prefix("Key").unwrap_or(&debug);
            if value.len() != 1 || !value.as_bytes()[0].is_ascii_alphanumeric() {
                return None;
            }
            value.to_ascii_lowercase()
        }
    };
    Some(name)
}

fn release_viewer_inputs(
    session: &Session,
    keys: &mut HashSet<String>,
    buttons: &mut HashSet<String>,
) {
    for name in keys.drain() {
        session.send_input(InputCommand::Key {
            name,
            pressed: false,
        });
    }
    for button in buttons.drain() {
        session.send_input(InputCommand::Button {
            x: 0.5,
            y: 0.5,
            button,
            pressed: false,
        });
    }
}

fn teacher_viewer(session: Arc<Session>) -> Result<()> {
    let display = Monitor::all()?
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .ok_or("No teacher display found")?;
    let (view_w, view_h) = (display.width()? as usize, display.height()? as usize);
    let factor = (1920.0 / view_w as f64)
        .min(1080.0 / view_h as f64)
        .min(1.0);
    let (width, height) = (
        (view_w as f64 * factor).round() as usize,
        (view_h as f64 * factor).round() as usize,
    );
    let mut window = Window::new(
        "YT Remote 3.0 - STUDENT SCREEN",
        view_w,
        view_h,
        WindowOptions {
            borderless: true,
            scale_mode: minifb::ScaleMode::Stretch,
            ..WindowOptions::default()
        },
    )?;
    window.set_position(0, 0);
    window.set_target_fps(60);
    let frames = Arc::new(Latest::<Rendered>::default());
    let decoded = frames.clone();
    let peer = session.clone();
    let decoder = thread::spawn(move || {
        while peer.active.load(SeqCst) && peer.approved.load(SeqCst) {
            if let Some(frame) = peer.take_frame() {
                match decode_frame(frame, width, height) {
                    Ok(frame) => decoded.put(frame),
                    Err(_) => {
                        peer.stop();
                        break;
                    }
                }
            } else {
                thread::sleep(Duration::from_millis(2));
            }
        }
        decoded.clear();
    });
    let mut remote: Option<Rendered> = None;
    let mut buffer = vec![0; width * height];
    let mut held = HashSet::new();
    let mut buttons = HashSet::new();
    let mut last_pointer = None;
    let mut old_click = false;
    let result = (|| -> Result<()> {
        while window.is_open()
            && session.active.load(SeqCst)
            && session.approved.load(SeqCst)
            && !window.is_key_down(Key::Escape)
        {
            if let Some(frame) = frames.take() {
                session.ack_frame(frame.sequence);
                remote = Some(frame);
            }
            if let Some(frame) = &remote {
                buffer.copy_from_slice(&frame.pixels);
            } else {
                buffer.fill(0x101827);
            }
            let overlay_w = width.min(96);
            let overlay_h = height.min(32);
            for y in 0..overlay_h {
                for x in width - overlay_w..width {
                    buffer[y * width + x] = if x >= width - overlay_w / 2 {
                        RED
                    } else {
                        BLUE
                    };
                }
            }
            // Simple local minus / cross glyphs, never sent to the student.
            if overlay_w >= 48 && overlay_h >= 24 {
                for x in width - overlay_w + 12..width - overlay_w + 28 {
                    buffer[16 * width + x] = WHITE;
                }
                for n in 0..14 {
                    buffer[(9 + n) * width + width - 30 + n] = WHITE;
                    buffer[(9 + n) * width + width - 17 - n] = WHITE;
                }
            }
            if !window.is_active() {
                release_viewer_inputs(&session, &mut held, &mut buttons);
                last_pointer = None;
            } else {
                let size = window.get_size();
                let point = window.get_mouse_pos(MouseMode::Pass).map(|(x, y)| {
                    (
                        x * width as f32 / size.0.max(1) as f32,
                        y * height as f32 / size.1.max(1) as f32,
                    )
                });
                let down = window.get_mouse_down(MouseButton::Left);
                let edge = down && !old_click;
                old_click = down;
                if let Some((x, y)) = point {
                    if x >= (width - overlay_w) as f32 && y >= 0.0 && y < overlay_h as f32 {
                        release_viewer_inputs(&session, &mut held, &mut buttons);
                        if edge {
                            if x >= (width - overlay_w / 2) as f32 {
                                break;
                            }
                            platform::minimize(&window).map_err(|e| e.to_string())?;
                        }
                    } else if let Some((x, y)) =
                        remote.as_ref().and_then(|f| f.viewport.remote_point(x, y))
                    {
                        if last_pointer != Some((x.to_bits(), y.to_bits())) {
                            session.send_input(InputCommand::Move { x, y });
                            last_pointer = Some((x.to_bits(), y.to_bits()));
                        }
                        for (button, name) in [
                            (MouseButton::Left, "left"),
                            (MouseButton::Right, "right"),
                            (MouseButton::Middle, "middle"),
                        ] {
                            let pressed = window.get_mouse_down(button);
                            let was = buttons.contains(name);
                            if pressed != was {
                                session.send_input(InputCommand::Button {
                                    x,
                                    y,
                                    button: name.into(),
                                    pressed,
                                });
                                if pressed {
                                    buttons.insert(name.into());
                                } else {
                                    buttons.remove(name);
                                }
                            }
                        }
                        if let Some((_, dy)) = window.get_scroll_wheel() {
                            let delta = (-dy.round() as i32).clamp(-5, 5);
                            if delta != 0 {
                                session.send_input(InputCommand::Scroll { delta });
                            }
                        }
                    } else {
                        release_viewer_inputs(&session, &mut held, &mut buttons);
                        last_pointer = None;
                    }
                }
                for key in window.get_keys_pressed(KeyRepeat::No) {
                    if let Some(name) = key_name(key) {
                        if held.insert(name.clone()) {
                            session.send_input(InputCommand::Key {
                                name,
                                pressed: true,
                            });
                        }
                    }
                }
                for key in window.get_keys_released() {
                    if let Some(name) = key_name(key) {
                        if !window
                            .get_keys()
                            .iter()
                            .any(|k| key_name(*k).as_deref() == Some(&name))
                            && held.remove(&name)
                        {
                            session.send_input(InputCommand::Key {
                                name,
                                pressed: false,
                            });
                        }
                    }
                }
            }
            window.update_with_buffer(&buffer, width, height)?;
        }
        Ok(())
    })();
    session.stop();
    let _ = decoder.join();
    frames.clear();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decode_checks_real_jpeg_dimensions_before_allocation() {
        let image = image::RgbImage::from_pixel(8, 4, image::Rgb([255, 255, 255]));
        let mut jpeg = Vec::new();
        JpegEncoder::new(&mut jpeg).encode_image(&image).unwrap();
        let wrong = Frame {
            sequence: 1,
            width: 1920,
            height: 1080,
            jpeg: jpeg.clone(),
        };
        assert!(decode_frame(wrong, 100, 100).is_err());
        let correct = decode_frame(
            Frame {
                sequence: 1,
                width: 8,
                height: 4,
                jpeg,
            },
            100,
            100,
        )
        .unwrap();
        assert_eq!(correct.pixels.len(), 10000);
        assert_eq!(correct.viewport.height, 50);
    }
    #[test]
    fn digit_and_mac_command_keys_have_native_supported_names() {
        assert_eq!(key_name(Key::Key1).as_deref(), Some("1"));
        assert_eq!(key_name(Key::LeftSuper).as_deref(), Some("meta"));
        assert_eq!(key_name(Key::Escape), None);
        assert_eq!(key_name(Key::Equal).as_deref(), Some("="));
        assert_eq!(key_name(Key::F5).as_deref(), Some("f5"));
    }
}
