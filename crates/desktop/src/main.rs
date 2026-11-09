//! Development desktop transport, not a production-grade remote desktop release.
//! Local OS screen-capture and input permissions are required.
use enigo::{Button, Coordinate, Direction, Enigo, Key as EKey, Keyboard, Mouse, Settings};
use image::{imageops::FilterType, DynamicImage, ImageFormat};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use serde_json::{json, Value};
use std::{
    env, error::Error, io::{self, Cursor},
    sync::{atomic::{AtomicBool, Ordering}, mpsc, Arc},
    thread, time::{Duration, Instant},
};
use tungstenite::{client::IntoClientRequest, stream::MaybeTlsStream, Error as WsError, Message};
use xcap::Monitor;
mod gui;
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn api(base: &str, route: &str) -> String {
    format!("{}{}", base.trim_end_matches('/'), route)
}
fn token_get(url: &str, token: &str) -> Result<Value> {
    Ok(ureq::get(url).set("Authorization", &format!("Bearer {token}"))
        .call()?.into_json()?)
}
fn token_post(url: &str, token: &str, data: Value) -> Result<()> {
    ureq::post(url)
        .set("Authorization", &format!("Bearer {token}"))
        .send_json(data)?;
    Ok(())
}
fn websocket_url(base: &str, code: &str, role: &str) -> Result<String> {
    let base = base.trim_end_matches('/');
    if let Some(host) = base.strip_prefix("https://") {
        Ok(format!("wss://{host}/v1/sessions/{code}/ws/{role}"))
    } else if let Some(host) = base.strip_prefix("http://") {
        Ok(format!("ws://{host}/v1/sessions/{code}/ws/{role}"))
    } else {
        Err("YT_REMOTE_SERVER must start with https:// or http://".into())
    }
}
fn connect_socket(url: &str, token: &str) -> Result<tungstenite::WebSocket<MaybeTlsStream<std::net::TcpStream>>> {
    let mut request = url.into_client_request()?;
    request.headers_mut().insert(
        "Authorization", format!("Bearer {token}").parse()?,
    );
    let (mut socket, _) = tungstenite::connect(request)?;
    match socket.get_mut() {
        MaybeTlsStream::Plain(s) => s.set_read_timeout(Some(Duration::from_millis(8)))?,
        MaybeTlsStream::NativeTls(s) => s.get_mut().set_read_timeout(Some(Duration::from_millis(8)))?,
        _ => {}
    }
    Ok(socket)
}
fn no_message(err: &WsError) -> bool {
    matches!(err, WsError::Io(ioe)
        if ioe.kind() == io::ErrorKind::TimedOut || ioe.kind() == io::ErrorKind::WouldBlock)
}
fn press_key(enigo: &mut Enigo, name: &str, pressed: bool) {
    let direction = if pressed { Direction::Press } else { Direction::Release };
    let key = match name {
        "enter" => EKey::Return, "escape" => EKey::Escape,
        "backspace" => EKey::Backspace, "tab" => EKey::Tab,
        "space" => EKey::Space, "left" => EKey::LeftArrow,
        "right" => EKey::RightArrow, "up" => EKey::UpArrow,
        "down" => EKey::DownArrow, "delete" => EKey::Delete,
        "leftctrl" | "rightctrl" => EKey::Control,
        "leftshift" | "rightshift" => EKey::Shift,
        "leftalt" | "rightalt" => EKey::Alt,
        _ if name.len() == 1 => EKey::Unicode(name.chars().next().unwrap()),
        _ => return,
    };
    let _ = enigo.key(key, direction);
}
fn apply_input(enigo: &mut Enigo, command: Value) {
    let kind = command["kind"].as_str().unwrap_or("");
    match kind {
        "move" | "button" => {
            let (x, y) = (command["x"].as_f64(), command["y"].as_f64());
            if let (Some(x), Some(y)) = (x, y) {
                if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) { return; }
                if let Ok((w,h)) = enigo.main_display() {
                    let _ = enigo.move_mouse((x * f64::from(w - 1)) as i32,
                        (y * f64::from(h - 1)) as i32, Coordinate::Abs);
                }
            }
            if kind == "button" {
                let b = match command["button"].as_str().unwrap_or("") {
                    "left" => Button::Left, "right" => Button::Right,
                    "middle" => Button::Middle, _ => return,
                };
                let _ = enigo.button(b, if command["pressed"] == true {
                    Direction::Press
                } else { Direction::Release });
            }
        }
        "scroll" => {
            let delta = command["delta"].as_i64().unwrap_or(0).clamp(-5, 5);
            let _ = enigo.scroll(delta as i32, enigo::Axis::Vertical);
        }
        "key" => {
            if let Some(name) = command["name"].as_str() {
                press_key(enigo, name, command["pressed"] == true);
            }
        }
        _ => {}
    }
}
fn student(server: &str) -> Result<()> {
    let session: Value = ureq::post(&api(server, "/v1/sessions")).call()?.into_json()?;
    let code = session["code"].as_str().ok_or("missing code")?.to_owned();
    let token = session["student_token"].as_str().ok_or("missing student token")?.to_owned();
    println!("\nYOUR CONNECTION CODE: {code}");
    println!("Send this code to your teacher. No screen data is shared yet.");
    let request = loop {
        let status = token_get(&api(server, &format!("/v1/sessions/{code}/student")), &token)?;
        match status["status"].as_str().unwrap_or("") {
            "requested" => break status["request_id"].as_str().ok_or("missing request id")?.to_owned(),
            "expired" | "ended" => return Err("Connection code expired".into()),
            "waiting" => thread::sleep(Duration::from_millis(500)),
            _ => return Err("Session no longer waiting".into()),
        }
    };
    println!("\nTeacher requests SCREEN + MOUSE + KEYBOARD access.");
    println!("Type ALLOW to permit the session, or anything else to DENY:");
    let mut approval = String::new();
    io::stdin().read_line(&mut approval)?;
    let allow = approval.trim() == "ALLOW";
    token_post(&api(server, &format!("/v1/sessions/{code}/{}",
        if allow {"allow"} else {"deny"})), &token, json!({"request_id":request}))?;
    if !allow { println!("Access denied. No screen was shared."); return Ok(()); }
    println!("Sharing approved. Press Enter to STOP SHARING NOW.");
    let running = Arc::new(AtomicBool::new(true));
    let stop = Arc::clone(&running);
    thread::spawn(move || {
        let mut input = String::new(); let _ = io::stdin().read_line(&mut input);
        stop.store(false, Ordering::SeqCst);
    });
    stream_student(server, &code, &token, &running)

}
fn teacher(server: &str, code: &str) -> Result<()> {
    let code = code.trim().to_ascii_uppercase();
    let session: Value = ureq::post(&api(server, &format!("/v1/sessions/{code}/request")))
        .call()?.into_json()?;
    let token = session["teacher_token"].as_str().ok_or("missing token")?.to_owned();
    println!("Waiting for student permission...");
    loop {
        let status = token_get(&api(server, &format!("/v1/sessions/{code}/teacher")), &token)?;
        match status["status"].as_str().unwrap_or("") {
            "approved" => break,
            "requested" => thread::sleep(Duration::from_millis(500)),
            "denied" => return Err("The student denied access".into()),
            _ => return Err("Session expired or ended".into()),
        }
    }
    teacher_viewer(server, &code, token)

}
fn stream_student(server: &str, code: &str, token: &str, running: &Arc<AtomicBool>) -> Result<()> {
    let mut socket = connect_socket(&websocket_url(server, &code, "student")?, &token)?;
    let monitor = Monitor::all()?.into_iter().next().ok_or("No monitor found")?;
    let mut enigo = Enigo::new(&Settings::default())?;
    let mut last = Instant::now() - Duration::from_secs(1);
    let sharing_result: Result<()> = (|| {
    while running.load(Ordering::SeqCst) {
        if last.elapsed() >= Duration::from_millis(66) {
            let shot = monitor.capture_image()?;
            let mut bytes = Cursor::new(Vec::new());
            DynamicImage::ImageRgba8(shot)
                .resize(1280, 720, FilterType::Triangle)
                .write_to(&mut bytes, ImageFormat::Jpeg)?;
            socket.send(Message::Binary(bytes.into_inner()))?;
            last = Instant::now();
        }
        match socket.read() {
            Ok(Message::Text(msg)) => {
                if let Ok(command) = serde_json::from_str::<Value>(&msg) {
                    apply_input(&mut enigo, command);
                }
            }
            Ok(Message::Close(_)) => break,
            Err(e) if no_message(&e) => {}
            Err(e) => return Err(e.into()),
            _ => {}
        }
    }
    Ok(())
    })();
    running.store(false, Ordering::SeqCst);
    let _ = socket.close(None);
    // Always revoke the approved session, including on capture/transport error.
    let _ = token_post(&api(server, &format!("/v1/sessions/{code}/end")), &token, json!({}));
    println!("Screen sharing stopped.");
    sharing_result
}
fn teacher_viewer(server: &str, code: &str, token: String) -> Result<()> {
    let (to_ui, from_network) = mpsc::sync_channel::<Vec<u32>>(2);
    let (to_network, from_ui) = mpsc::channel::<String>();
    let address = websocket_url(server, &code, "teacher")?;
    let active = Arc::new(AtomicBool::new(true));
    let network_active = Arc::clone(&active);
    thread::spawn(move || {
        let Ok(mut socket) = connect_socket(&address, &token) else { network_active.store(false, Ordering::SeqCst); return; };
        while network_active.load(Ordering::SeqCst) {
            while let Ok(cmd) = from_ui.try_recv() {
                if socket.send(Message::Text(cmd)).is_err() { network_active.store(false, Ordering::SeqCst); break; }
            }
            match socket.read() {
                Ok(Message::Binary(data)) => {
                    if let Ok(img) = image::load_from_memory(&data) {
                        let img = img.resize_exact(1280, 720, FilterType::Triangle).to_rgb8();
                        let pixels = img.pixels().map(|p| {
                            ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | p[2] as u32
                        }).collect();
                        let _ = to_ui.try_send(pixels);
                    }
                }
                Ok(Message::Close(_)) => { network_active.store(false, Ordering::SeqCst); break; }
                Err(e) if no_message(&e) => {}
                Err(_) => { network_active.store(false, Ordering::SeqCst); break; }
                _ => {}
            }
        }
        let _ = socket.close(None);
    });
    let mut window = Window::new("YT Remote — Esc to Disconnect",1280,720,
        WindowOptions::default())?;
    window.set_target_fps(60);
    let mut pixels = vec![0_u32; 1280 * 720];
    let mut previous_mouse = None;
    let mut old_left = false;
    let mut old_right = false;
    while active.load(Ordering::SeqCst) && window.is_open() && !window.is_key_down(Key::Escape) {
        while let Ok(frame) = from_network.try_recv() { pixels = frame; }
        if let Some((x,y)) = window.get_mouse_pos(MouseMode::Clamp) {
            let x = (f64::from(x) / 1280.0).clamp(0.0,1.0);
            let y = (f64::from(y) / 720.0).clamp(0.0,1.0);
            if previous_mouse != Some((x.to_bits(), y.to_bits())) {
                let _ = to_network.send(json!({"kind":"move","x":x,"y":y}).to_string());
                previous_mouse = Some((x.to_bits(), y.to_bits()));
            }
            for (button,name,old) in [(MouseButton::Left,"left",&mut old_left),
                                      (MouseButton::Right,"right",&mut old_right)] {
                let down = window.get_mouse_down(button);
                if down != *old {
                    let _ = to_network.send(json!({"kind":"button","x":x,"y":y,
                        "button":name,"pressed":down}).to_string());
                    *old = down;
                }
            }
        }
        for key in window.get_keys_pressed(KeyRepeat::No) {
            if key == Key::Escape { continue; }
            let name = format!("{key:?}").to_ascii_lowercase();
            let _ = to_network.send(json!({"kind":"key","name":name,"pressed":true}).to_string());
        }
        for key in window.get_keys_released() {
            let name = format!("{key:?}").to_ascii_lowercase();
            let _ = to_network.send(json!({"kind":"key","name":name,"pressed":false}).to_string());
        }
        window.update_with_buffer(&pixels, 1280, 720)?;
    }
    active.store(false, Ordering::SeqCst);
    println!("Disconnected.");
    Ok(())
}
fn main() {
    let server = env::var("YT_REMOTE_SERVER").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    let args: Vec<String> = env::args().collect();
    let result = match args.as_slice() {
        [_, role] if role == "student" => student(&server),
        [_, role, code] if role == "teacher" => teacher(&server, code),
        [_] => gui::launch(&server),
        _ => {
            eprintln!("YT Remote: launch without arguments for the graphical interface.");
            eprintln!("Developer mode: student | teacher STUDENT_CODE");
        }
    };
    if let Err(err) = result { eprintln!("YT Remote error: {err}"); std::process::exit(1); }
}
