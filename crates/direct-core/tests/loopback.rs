use direct_core::{
    start_student, start_teacher, Event, Frame, InputCommand, Session, TeacherOptions,
};
use std::{
    sync::atomic::Ordering,
    thread,
    time::{Duration, Instant},
};

fn wait_event(s: &Session, predicate: impl Fn(&Event) -> bool) -> Event {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if let Some(e) = s.try_event() {
            if predicate(&e) {
                return e;
            }
        }
        assert!(Instant::now() < deadline, "event timeout");
        thread::sleep(Duration::from_millis(5));
    }
}
fn pair() -> (Session, Session, String) {
    let teacher = start_teacher(TeacherOptions::loopback());
    let Event::Invitation(code) = wait_event(&teacher, |e| matches!(e, Event::Invitation(_)))
    else {
        panic!()
    };
    let student = start_student(&code).unwrap();
    wait_event(&teacher, |e| matches!(e, Event::Connected));
    wait_event(&student, |e| matches!(e, Event::Connected));
    (teacher, student, code)
}
fn frame() -> Frame {
    Frame {
        sequence: 1,
        width: 1920,
        height: 1080,
        jpeg: vec![1, 2, 3],
    }
}

#[test]
fn local_consent_controls_direction_and_stop_is_synchronous() {
    let (teacher, student, _) = pair();
    let mut applied = 0;
    assert!(student.with_consent(|| applied += 1).is_none());
    assert_eq!(applied, 0);
    assert!(!student.publish_frame(frame()));
    assert!(!teacher.send_input(InputCommand::Move { x: 0.5, y: 0.5 }));
    assert!(!teacher.allow());
    assert!(student.allow());
    assert!(student.with_consent(|| applied += 1).is_some());
    assert_eq!(applied, 1);
    wait_event(&teacher, |e| matches!(e, Event::Approved));
    assert!(teacher.send_input(InputCommand::Move { x: 0.5, y: 0.5 }));
    assert!(student.publish_frame(frame()));
    let until = Instant::now() + Duration::from_secs(3);
    while teacher.take_frame().is_none() {
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(5));
    }
    student.stop();
    assert!(!student.active.load(Ordering::SeqCst));
    assert!(!student.approved.load(Ordering::SeqCst));
    assert!(student.try_input().is_none());
    assert!(!student.publish_frame(frame()));
    wait_event(&teacher, |e| matches!(e, Event::Ended(_)));
}
#[test]
fn deny_disconnect_and_replay_fail_closed() {
    let (teacher, student, code) = pair();
    student.deny();
    assert!(!student.approved.load(Ordering::SeqCst));
    wait_event(&teacher, |e| matches!(e, Event::Ended(_)));
    let replay = start_student(&code).unwrap();
    wait_event(&replay, |e| matches!(e, Event::Error(_) | Event::Ended(_)));
    let (teacher, student, _) = pair();
    student.allow();
    wait_event(&teacher, |e| matches!(e, Event::Approved));
    drop(teacher);
    wait_event(&student, |e| matches!(e, Event::Ended(_)));
    assert!(!student.approved.load(Ordering::SeqCst));
}
#[test]
fn invalid_invitation_and_invalid_or_oversize_work_are_rejected() {
    assert!(start_student("garbage").is_err());
    let (teacher, student, _) = pair();
    student.allow();
    wait_event(&teacher, |e| matches!(e, Event::Approved));
    assert!(!teacher.send_input(InputCommand::Move {
        x: f64::NAN,
        y: 0.5
    }));
    assert!(!teacher.send_input(InputCommand::Key {
        name: "arbitrary system command".into(),
        pressed: true
    }));
    assert!(InputCommand::Key {
        name: "=".into(),
        pressed: true
    }
    .valid());
    let mut huge = frame();
    huge.jpeg = vec![0; 3 * 1024 * 1024];
    assert!(!student.publish_frame(huge));
    let mut huge = frame();
    huge.width = 4000;
    assert!(!student.publish_frame(huge));
}

fn edited_code(code: &str, edit: impl FnOnce(&mut serde_json::Value)) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let raw = URL_SAFE_NO_PAD
        .decode(code.strip_prefix("YTR3.").unwrap())
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    edit(&mut value);
    format!(
        "YTR3.{}",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&value).unwrap())
    )
}

#[test]
fn expired_codes_wrong_tokens_and_replaced_certificates_fail_closed() {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let teacher = start_teacher(TeacherOptions::loopback());
    let Event::Invitation(code) = wait_event(&teacher, |e| matches!(e, Event::Invitation(_)))
    else {
        panic!()
    };
    assert!(start_student(&edited_code(&code, |v| v["expires"] = 1.into())).is_err());
    let wrong = edited_code(&code, |v| {
        v["token"] = URL_SAFE_NO_PAD.encode([0u8; 32]).into()
    });
    let student = start_student(&wrong).unwrap();
    wait_event(&student, |e| matches!(e, Event::Error(_)));
    assert!(!teacher.approved.load(Ordering::SeqCst));
    let other = rcgen::generate_simple_self_signed(vec!["direct-session".into()]).unwrap();
    let replaced = edited_code(&code, |v| {
        v["cert"] = URL_SAFE_NO_PAD.encode(other.cert.der()).into()
    });
    let student = start_student(&replaced).unwrap();
    wait_event(&student, |e| matches!(e, Event::Error(_)));
    assert!(!teacher.approved.load(Ordering::SeqCst));
    let student = start_student(&code).unwrap();
    wait_event(&student, |e| matches!(e, Event::Connected));
    assert!(student.allow());
    wait_event(&teacher, |e| matches!(e, Event::Approved));
}

#[test]
fn queued_key_edges_are_delivered_in_order_and_never_after_stop() {
    let (teacher, student, _) = pair();
    student.allow();
    wait_event(&teacher, |e| matches!(e, Event::Approved));
    for _ in 0..4 {
        for pressed in [true, false] {
            assert!(teacher.send_input(InputCommand::Key {
                name: "control".into(),
                pressed
            }));
        }
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut states = Vec::new();
    while states.len() < 8 {
        if let Some(InputCommand::Key { name, pressed }) = student.try_input() {
            assert_eq!(name, "control");
            states.push(pressed);
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        states,
        vec![true, false, true, false, true, false, true, false]
    );
    student.stop();
    assert!(student.try_input().is_none());
    wait_event(&teacher, |e| matches!(e, Event::Ended(_)));
    assert!(!teacher.send_input(InputCommand::Key {
        name: "control".into(),
        pressed: true
    }));
}

#[test]
fn slow_viewer_limits_in_flight_frames_and_newest_ack_resumes_sending() {
    let (teacher, student, _) = pair();
    student.allow();
    wait_event(&teacher, |e| matches!(e, Event::Approved));
    for sequence in 1..=3 {
        let mut f = frame();
        f.sequence = sequence;
        assert!(student.publish_frame(f));
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            if teacher.take_frame().is_some() {
                break;
            }
            assert!(Instant::now() < until);
            thread::sleep(Duration::from_millis(2));
        }
    }
    let mut f = frame();
    f.sequence = 4;
    assert!(student.publish_frame(f));
    let until = Instant::now() + Duration::from_millis(100);
    while Instant::now() < until {
        assert!(
            teacher.take_frame().is_none(),
            "fourth frame bypassed the viewer's bounded window"
        );
        thread::sleep(Duration::from_millis(2));
    }
    teacher.ack_frame(3);
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(f) = teacher.take_frame() {
            assert_eq!(f.sequence, 4);
            break;
        }
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(2));
    }
    student.stop();
    let mut injected = 0;
    assert!(student.with_consent(|| injected += 1).is_none());
    assert_eq!(injected, 0);
}
