//! Minimal native single-window launcher. No background or unattended approval.
use super::{api, stream_student, teacher_viewer, token_get, token_post, Result};
use arboard::Clipboard;
use font8x8::{BASIC_FONTS, UnicodeFonts};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use serde_json::{json, Value};
use std::{sync::{atomic::{AtomicBool, Ordering}, Arc}, thread, time::{Duration, Instant}};

const W: usize = 800;
const H: usize = 510;
const BG: u32 = 0x101827;
const CARD: u32 = 0x1c293b;
const FG: u32 = 0xf0f5ff;
const MUTED: u32 = 0x9daec5;
const BLUE: u32 = 0x397bef;
const GREEN: u32 = 0x20876a;
const RED: u32 = 0x973b4c;
#[derive(Clone)]
struct Credentials { code: String, token: String }
enum Page {
    Home, Waiting(Credentials), Approval(Credentials, String),
    Sharing(Credentials, Arc<AtomicBool>),
    Connecting(String, String),
    Error(String),
}
#[derive(Copy,Clone)]
struct Rect { x: usize, y: usize, w: usize, h: usize }
impl Rect {
    fn contains(self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}
struct Canvas { pixels: Vec<u32> }
impl Canvas {
    fn new() -> Self { Self { pixels: vec![BG; W * H] } }
    fn clear(&mut self) { self.pixels.fill(BG); }
    fn fill(&mut self, r: Rect, color: u32) {
        for y in r.y..(r.y + r.h).min(H) {
            for x in r.x..(r.x + r.w).min(W) { self.pixels[y * W + x] = color; }
        }
    }
    fn text(&mut self, x: usize, y: usize, text: &str, color: u32, scale: usize) {
        let mut cursor = x;
        for ch in text.chars() {
            if let Some(bitmap) = BASIC_FONTS.get(ch) {
                for (dy, row) in bitmap.iter().enumerate() {
                    for dx in 0..8 {
                        if (*row >> dx) & 1 != 0 {
                            self.fill(Rect { x: cursor + dx * scale, y: y + dy * scale,
                                w: scale, h: scale }, color);
                        }
                    }
                }
            }
            cursor += 8 * scale;
            if cursor + 8 * scale > W { break; }
        }
    }
    fn button(&mut self, rect: Rect, text: &str, color: u32) {
        self.fill(rect, color);
        let width = text.len() * 8 * 2;
        let x = rect.x + rect.w.saturating_sub(width) / 2;
        let y = rect.y + rect.h.saturating_sub(16) / 2;
        self.text(x, y, text, FG, 2);
    }
}
const PRIMARY: Rect = Rect { x: 95, y: 245, w: 610, h: 55 };
const SECONDARY: Rect = Rect { x: 95, y: 325, w: 610, h: 55 };
const APPROVE: Rect = Rect { x: 95, y: 288, w: 610, h: 55 };
const DENY: Rect = Rect { x: 95, y: 358, w: 610, h: 55 };
fn start_student(server: &str) -> Result<Credentials> {
    let response: Value = ureq::post(&api(server, "/v1/sessions")).call()?.into_json()?;
    Ok(Credentials {
        code: response["code"].as_str().ok_or("server did not return a code")?.to_owned(),
        token: response["student_token"].as_str().ok_or("server did not return a student token")?.to_owned(),
    })
}
fn request_teacher(server: &str, code: &str) -> Result<(String,String)> {
    let response: Value = ureq::post(&api(server, &format!("/v1/sessions/{code}/request")))
        .call()?.into_json()?;
    Ok((
        response["teacher_token"].as_str().ok_or("missing teacher token")?.to_owned(),
        response["request_id"].as_str().ok_or("missing request id")?.to_owned(),
    ))
}
fn stop_student(server: &str, credentials: &Credentials) {
    let _ = token_post(&api(server, &format!("/v1/sessions/{}/end", credentials.code)),
        &credentials.token, json!({}));
}
fn is_click(window: &Window, mouse_was_down: &mut bool) -> Option<(usize,usize)> {
    let down = window.get_mouse_down(MouseButton::Left);
    let clicked = down && !*mouse_was_down;
    *mouse_was_down = down;
    if clicked {
        return window.get_mouse_pos(MouseMode::Clamp).map(|(x,y)| (x as usize,y as usize));
    }
    None
}
fn input_code(window: &Window, typed: &mut String, clipboard: &mut Option<Clipboard>) {
    if (window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl)
        || window.is_key_down(Key::LeftSuper) || window.is_key_down(Key::RightSuper))
        && window.is_key_pressed(Key::V, KeyRepeat::No)
    {
        if let Some(cb) = clipboard.as_mut() {
            if let Ok(text) = cb.get_text() {
                typed.clear();
                for c in text.chars().filter(|c| c.is_ascii_alphanumeric()) {
                    if typed.len() < 16 { typed.push(c.to_ascii_uppercase()); }
                }
            }
        }
        return;
    }
    for key in window.get_keys_pressed(KeyRepeat::No) {
        match key {
            Key::Backspace => { typed.pop(); }
            _ => {
                let label = format!("{key:?}");
                let raw = label.strip_prefix("Key").unwrap_or(&label);
                if raw.len() == 1
                    && raw.bytes().all(|b| b.is_ascii_alphanumeric())
                    && typed.len() < 16 {
                    typed.push(raw.chars().next().unwrap().to_ascii_uppercase());
                }
            }
        }
    }
}
fn formatted(code: &str) -> String {
    let chars: String = code.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    chars.as_bytes().chunks(4).map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
        .collect::<Vec<_>>().join("-")
}
pub(super) fn launch(server: &str) -> Result<()> {
    let mut window = Window::new("YT Remote 2.0", W, H, WindowOptions::default())?;
    window.set_target_fps(30);
    let mut canvas = Canvas::new();
    let mut page = Page::Home;
    let mut code_input = String::new();
    let mut clipboard = Clipboard::new().ok();
    let mut mouse_down = false;
    let mut last_poll = Instant::now() - Duration::from_secs(1);
    while window.is_open() {
        let click = is_click(&window, &mut mouse_down);
        canvas.clear();
        canvas.fill(Rect { x: 58, y: 40, w: 684, h: 425 }, CARD);
        canvas.text(95, 72, "YT REMOTE 2.0", FG, 3);
        canvas.text(95, 122, "Secure, permission-based remote desktop", MUTED, 1);
        let mut next = None;
        match &page {
            Page::Home => {
                canvas.text(95, 185, "STUDENT: Generate your connection code", FG, 1);
                canvas.button(PRIMARY, "GENERATE CODE (STUDENT)", BLUE);
                canvas.text(95, 314, "TEACHER: Paste your student's code", MUTED, 1);
                canvas.fill(Rect { x: 95, y: 339, w: 610, h: 45 }, BG);
                canvas.text(113, 353, &formatted(&code_input), FG, 2);
                canvas.button(Rect { x: 95, y: 399, w: 610, h: 45 }, "CONNECT", BLUE);
                input_code(&window, &mut code_input, &mut clipboard);
                if click.is_some_and(|(x,y)| PRIMARY.contains(x,y)) {
                    next = Some(match start_student(server) {
                        Ok(c) => Page::Waiting(c),
                        Err(e) => Page::Error(format!("Cannot create session: {e}")),
                    });
                } else if (click.is_some_and(|(x,y)| (Rect { x:95,y:399,w:610,h:45 }).contains(x,y))
                    || window.is_key_pressed(Key::Enter, KeyRepeat::No)) && code_input.len() == 16 {
                    let code = formatted(&code_input);
                    next = Some(match request_teacher(server, &code) {
                        Ok((token, _)) => Page::Connecting(code, token),
                        Err(e) => Page::Error(format!("Connection failed: {e}")),
                    });
                }
            }
            Page::Waiting(c) => {
                canvas.text(95, 180, "YOUR CONNECTION CODE", MUTED, 2);
                canvas.text(95, 222, &c.code, FG, 2);
                canvas.button(PRIMARY, "COPY CODE", BLUE);
                canvas.text(95, 322, "Waiting for your teacher to CONNECT...", MUTED, 1);
                canvas.button(Rect { x:95,y:360,w:610,h:50 }, "CANCEL", RED);
                if click.is_some_and(|(x,y)| PRIMARY.contains(x,y)) {
                    if let Some(cb) = clipboard.as_mut() { let _ = cb.set_text(c.code.clone()); }
                }
                if click.is_some_and(|(x,y)| (Rect { x:95,y:360,w:610,h:50 }).contains(x,y)) {
                    stop_student(server,c); next = Some(Page::Home);
                } else if last_poll.elapsed() >= Duration::from_millis(650) {
                    last_poll = Instant::now();
                    match token_get(&api(server,&format!("/v1/sessions/{}/student",c.code)),&c.token) {
                        Ok(status) if status["status"] == "requested" => {
                            if let Some(id) = status["request_id"].as_str() {
                                next = Some(Page::Approval(c.clone(), id.to_owned()));
                            }
                        }
                        Ok(status) if status["status"] == "expired" || status["status"] == "ended" =>
                            next = Some(Page::Error("The connection code has expired.".to_owned())),
                        Err(e) => next = Some(Page::Error(format!("Connection lost: {e}"))),
                        _ => {}
                    }
                }
            }
            Page::Approval(c,req) => {
                canvas.text(95, 179, "PERMISSION REQUIRED", FG, 2);
                canvas.text(95, 223, "Your teacher requests screen + mouse + keyboard.", MUTED, 1);
                canvas.text(95, 242, "Nothing is shared until you explicitly allow.", MUTED, 1);
                canvas.button(APPROVE, "ALLOW SCREEN & KEYBOARD", GREEN);
                canvas.button(DENY, "DENY", RED);
                let approve = click.is_some_and(|(x,y)| APPROVE.contains(x,y));
                let deny = click.is_some_and(|(x,y)| DENY.contains(x,y));
                if approve || deny {
                    let result = token_post(&api(server,&format!("/v1/sessions/{}/{}",
                        c.code, if approve {"allow"} else {"deny"})), &c.token, json!({"request_id":req}));
                    next = Some(match result {
                        Ok(()) if approve => {
                            let running = Arc::new(AtomicBool::new(true));
                            let flag = Arc::clone(&running);
                            let server = server.to_owned();
                            let code = c.code.clone();
                            let token = c.token.clone();
                            thread::spawn(move || {
                                let _ = stream_student(&server, &code, &token, &flag);
                                flag.store(false, Ordering::SeqCst);
                            });
                            Page::Sharing(c.clone(), running)
                        }
                        Ok(()) => Page::Home,
                        Err(e) => Page::Error(format!("Authorization failed: {e}")),
                    });
                }
            }
            Page::Sharing(c,running) => {
                canvas.text(95, 180, "SCREEN SHARING ACTIVE", FG, 2);
                canvas.text(95, 233, "Your teacher can see and control your screen.", MUTED, 1);
                canvas.text(95, 255, "You can revoke access immediately.", MUTED, 1);
                canvas.button(PRIMARY, "STOP SHARING NOW", RED);
                if click.is_some_and(|(x,y)| PRIMARY.contains(x,y))
                    || window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                    running.store(false, Ordering::SeqCst);
                    stop_student(server,c);
                    next = Some(Page::Home);
                } else if !running.load(Ordering::SeqCst) {
                    stop_student(server,c);
                    next = Some(Page::Error("Screen sharing disconnected.".to_owned()));
                }
            }
            Page::Connecting(code,token) => {
                canvas.text(95, 183, "WAITING FOR STUDENT APPROVAL", FG, 2);
                canvas.text(95, 230, "Screen access is blocked until the student allows.", MUTED, 1);
                canvas.button(PRIMARY, "CANCEL", RED);
                if click.is_some_and(|(x,y)| PRIMARY.contains(x,y)) {
                    next = Some(Page::Home);
                } else if last_poll.elapsed() >= Duration::from_millis(650) {
                    last_poll = Instant::now();
                    match token_get(&api(server,&format!("/v1/sessions/{code}/teacher")),token) {
                        Ok(status) if status["status"] == "approved" => {
                            // Close launcher before opening the fullscreen teacher viewer.
                            let (code,token) = (code.clone(),token.clone());
                            drop(window);
                            return teacher_viewer(server, &code, token);
                        }
                        Ok(status) if status["status"] == "denied" =>
                            next = Some(Page::Error("The student denied access.".to_owned())),
                        Ok(status) if status["status"] == "expired" || status["status"] == "ended" =>
                            next = Some(Page::Error("Connection expired or ended.".to_owned())),
                        Err(e) => next = Some(Page::Error(format!("Connection lost: {e}"))),
                        _ => {}
                    }
                }
            }
            Page::Error(message) => {
                canvas.text(95, 176, "CONNECTION STATUS", FG, 2);
                let short = message.chars().take(65).collect::<String>();
                canvas.text(95, 236, &short, MUTED, 1);
                canvas.button(PRIMARY, "BACK", BLUE);
                if click.is_some_and(|(x,y)| PRIMARY.contains(x,y)) {
                    next = Some(Page::Home);
                }
            }
        }
        if let Some(replacement) = next { page = replacement; }
        window.update_with_buffer(&canvas.pixels, W, H)?;
    }
    if let Page::Waiting(c) | Page::Approval(c,_) | Page::Sharing(c,_) = &page {
        stop_student(server,c);
    }
    if let Page::Sharing(_,running) = &page {
        running.store(false, Ordering::SeqCst);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::formatted;
    #[test]
    fn single_connection_code_is_grouped_for_copying() {
        assert_eq!(formatted("ABCDEFGHJKLMNPQR"), "ABCD-EFGH-JKLM-NPQR");
        assert_eq!(formatted("ABCD-EFGH-JKLM-NPQR"), "ABCD-EFGH-JKLM-NPQR");
        assert_eq!(formatted("ABCD"), "ABCD");
    }
}
