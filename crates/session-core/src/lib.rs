//! Explicit-consent connection lifecycle. A connection code is NOT authorization.
use rand::{rngs::OsRng, RngCore};
use std::{collections::HashMap, sync::{Arc, Mutex}, time::{Duration, Instant}};

const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const LIMIT: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status { Waiting, Requested, Approved, Denied, Ended, Expired }
#[derive(Debug, Clone)]
pub struct StudentSession { pub code: String, pub student_secret: String }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeacherRequest { pub request_id: String, pub teacher_secret: String }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error { NotFound, Unauthorized, InvalidCode, InvalidRequest, InvalidState, Expired, Capacity }
struct Entry {
    student_secret: String,
    teacher_secret: Option<String>,
    request_id: Option<String>,
    status: Status,
    expires_at: Instant,
}
#[derive(Clone, Default)]
pub struct SessionStore { inner: Arc<Mutex<HashMap<String, Entry>>> }
fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0_u8; bytes]; OsRng.fill_bytes(&mut buf);
    let mut out = String::with_capacity(bytes * 2);
    for b in buf { out.push_str(&format!("{b:02x}")); }
    out
}
fn random_code() -> String {
    let mut buf = [0_u8; 16]; OsRng.fill_bytes(&mut buf);
    let letters: String = buf.iter().map(|b| ALPHABET[(b & 31) as usize] as char).collect();
    format!("{}-{}-{}-{}", &letters[0..4], &letters[4..8], &letters[8..12], &letters[12..16])
}
/// Compare bearer secrets without an early exit on a mismatching byte.
/// Secrets are generated as 64-character lowercase hex strings; malformed
/// values are rejected before comparison, without indexing user-controlled data.
fn secret_matches(expected: &str, supplied: &str) -> bool {
    if expected.len() != 64 || supplied.len() != 64 {
        return false;
    }
    let mut difference = 0_u8;
    for (&a, &b) in expected.as_bytes().iter().zip(supplied.as_bytes()) {
        difference |= a ^ b;
    }
    difference == 0
}
fn normalized(raw: &str) -> Result<String, Error> {
    let code = raw.replace('-', "").to_ascii_uppercase();
    if code.len() != 16 || !code.bytes().all(|c| ALPHABET.contains(&c)) {
        return Err(Error::InvalidCode);
    }
    Ok(format!("{}-{}-{}-{}", &code[0..4], &code[4..8], &code[8..12], &code[12..16]))
}
impl SessionStore {
    pub fn new() -> Self { Self::default() }
    pub fn create(&self, ttl: Duration) -> Result<StudentSession, Error> {
        let mut all = self.inner.lock().expect("mutex poisoned");
        all.retain(|_, e| e.expires_at > Instant::now() && e.status != Status::Ended);
        if all.len() >= LIMIT { return Err(Error::Capacity); }
        let mut code = random_code();
        while all.contains_key(&code) { code = random_code(); }
        let secret = random_hex(32);
        all.insert(code.clone(), Entry {
            student_secret: secret.clone(), teacher_secret: None, request_id: None,
            status: Status::Waiting, expires_at: Instant::now() + ttl,
        });
        Ok(StudentSession { code, student_secret: secret })
    }
    pub fn request(&self, raw: &str) -> Result<TeacherRequest, Error> {
        let code = normalized(raw)?;
        let mut all = self.inner.lock().expect("mutex poisoned");
        let e = all.get_mut(&code).ok_or(Error::NotFound)?;
        if Instant::now() >= e.expires_at { e.status = Status::Expired; return Err(Error::Expired); }
        if e.status != Status::Waiting { return Err(Error::InvalidState); }
        let token = random_hex(32);
        let req = random_hex(16);
        e.teacher_secret = Some(token.clone());
        e.request_id = Some(req.clone());
        e.status = Status::Requested;
        Ok(TeacherRequest { request_id: req, teacher_secret: token })
    }
    pub fn student_status(&self, raw: &str, secret: &str) -> Result<(Status, Option<String>), Error> {
        let code = normalized(raw)?;
        let all = self.inner.lock().expect("mutex poisoned");
        let e = all.get(&code).ok_or(Error::NotFound)?;
        if !secret_matches(&e.student_secret, secret) { return Err(Error::Unauthorized); }
        if Instant::now() >= e.expires_at { return Ok((Status::Expired, None)); }
        Ok((e.status.clone(), e.request_id.clone()))
    }
    pub fn teacher_status(&self, raw: &str, secret: &str) -> Result<Status, Error> {
        let code = normalized(raw)?;
        let all = self.inner.lock().expect("mutex poisoned");
        let e = all.get(&code).ok_or(Error::NotFound)?;
        if !e.teacher_secret.as_deref().is_some_and(|expected| secret_matches(expected, secret)) { return Err(Error::Unauthorized); }
        if Instant::now() >= e.expires_at { return Ok(Status::Expired); }
        Ok(e.status.clone())
    }
    pub fn decide(&self, raw: &str, student_secret: &str, request_id: &str, allow: bool) -> Result<(), Error> {
        let code = normalized(raw)?;
        let mut all = self.inner.lock().expect("mutex poisoned");
        let e = all.get_mut(&code).ok_or(Error::NotFound)?;
        if !secret_matches(&e.student_secret, student_secret) { return Err(Error::Unauthorized); }
        if Instant::now() >= e.expires_at { e.status = Status::Expired; return Err(Error::Expired); }
        if e.status != Status::Requested || e.request_id.as_deref() != Some(request_id) { return Err(Error::InvalidRequest); }
        e.status = if allow { Status::Approved } else { Status::Denied };
        Ok(())
    }
    /// Cancel a pending teacher request without invalidating the student's code.
    /// A canceled teacher token cannot authorize a later approval.
    pub fn cancel_request(&self, raw: &str, teacher_secret: &str) -> Result<(), Error> {
        let code = normalized(raw)?;
        let mut all = self.inner.lock().expect("mutex poisoned");
        let e = all.get_mut(&code).ok_or(Error::NotFound)?;
        if !e.teacher_secret.as_deref().is_some_and(|expected| secret_matches(expected, teacher_secret)) {
            return Err(Error::Unauthorized);
        }
        if Instant::now() >= e.expires_at { e.status = Status::Expired; return Err(Error::Expired); }
        if e.status != Status::Requested { return Err(Error::InvalidState); }
        e.teacher_secret = None;
        e.request_id = None;
        e.status = Status::Waiting;
        Ok(())
    }
    pub fn can_stream(&self, raw: &str, teacher_secret: &str) -> bool {
        self.teacher_status(raw, teacher_secret) == Ok(Status::Approved)
    }
    pub fn end(&self, raw: &str, student_secret: &str) -> Result<(), Error> {
        let code = normalized(raw)?;
        let mut all = self.inner.lock().expect("mutex poisoned");
        let e = all.get_mut(&code).ok_or(Error::NotFound)?;
        if !secret_matches(&e.student_secret, student_secret) { return Err(Error::Unauthorized); }
        // Revoke the bearer and any pending request atomically with STOP.
        // A previously approved teacher must never authenticate again.
        e.teacher_secret = None;
        e.request_id = None;
        e.status = Status::Ended;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bearer_secrets_require_exact_length_and_bytes() {
        let sessions = SessionStore::new();
        let student = sessions.create(Duration::from_secs(900)).unwrap();
        let teacher = sessions.request(&student.code).unwrap();
        assert!(secret_matches(&student.student_secret, &student.student_secret));
        assert!(!secret_matches(&student.student_secret, ""));
        assert!(!secret_matches(&student.student_secret, "a"));
        assert!(!secret_matches(&student.student_secret, &student.student_secret.to_uppercase()));
        let mut altered = student.student_secret.clone();
        let replacement = if &altered[63..64] == "a" { "b" } else { "a" };
        altered.replace_range(63..64, replacement);
        assert!(!secret_matches(&student.student_secret, &altered));
        assert_eq!(sessions.student_status(&student.code, &altered), Err(Error::Unauthorized));
        assert_eq!(sessions.student_status(&student.code, &student.student_secret).unwrap().0, Status::Requested);
        assert_eq!(sessions.teacher_status(&student.code, &teacher.teacher_secret.to_uppercase()), Err(Error::Unauthorized));
        assert_eq!(sessions.cancel_request(&student.code, &teacher.teacher_secret.to_uppercase()), Err(Error::Unauthorized));
        assert_eq!(sessions.decide(&student.code, &altered, &teacher.request_id, true), Err(Error::Unauthorized));
        assert!(!sessions.can_stream(&student.code, &teacher.teacher_secret));
    }
    #[test]
    fn explicit_approval_is_required_and_revocable() {
        let s = SessionStore::new();
        let st = s.create(Duration::from_secs(900)).unwrap();
        let teacher = s.request(&st.code).unwrap();
        assert!(!s.can_stream(&st.code, &teacher.teacher_secret));
        assert_eq!(s.decide(&st.code, "wrong", &teacher.request_id, true), Err(Error::Unauthorized));
        assert!(!s.can_stream(&st.code, &teacher.teacher_secret));
        s.decide(&st.code, &st.student_secret, &teacher.request_id, true).unwrap();
        assert!(s.can_stream(&st.code, &teacher.teacher_secret));
        s.end(&st.code, &st.student_secret).unwrap();
        assert!(!s.can_stream(&st.code, &teacher.teacher_secret));
    }
    #[test]
    fn denied_requests_cannot_be_replayed() {
        let s = SessionStore::new();
        let st = s.create(Duration::from_secs(900)).unwrap();
        let teacher = s.request(&st.code).unwrap();
        s.decide(&st.code, &st.student_secret, &teacher.request_id, false).unwrap();
        assert!(!s.can_stream(&st.code, &teacher.teacher_secret));
        assert_eq!(s.decide(&st.code, &st.student_secret, &teacher.request_id, true), Err(Error::InvalidRequest));
    }
    #[test]
    fn cancelled_teacher_request_cannot_be_approved_or_reused() {
        let s = SessionStore::new();
        let st = s.create(Duration::from_secs(900)).unwrap();
        let first = s.request(&st.code).unwrap();
        assert_eq!(s.cancel_request(&st.code, "wrong"), Err(Error::Unauthorized));
        assert_eq!(s.student_status(&st.code, &st.student_secret).unwrap().0, Status::Requested);
        s.cancel_request(&st.code, &first.teacher_secret).unwrap();
        assert_eq!(s.student_status(&st.code, &st.student_secret).unwrap(), (Status::Waiting, None));
        assert_eq!(s.teacher_status(&st.code, &first.teacher_secret), Err(Error::Unauthorized));
        assert_eq!(s.decide(&st.code, &st.student_secret, &first.request_id, true), Err(Error::InvalidRequest));
        let second = s.request(&st.code).unwrap();
        assert_ne!(first.teacher_secret, second.teacher_secret);
        assert!(!s.can_stream(&st.code, &first.teacher_secret));
        s.decide(&st.code, &st.student_secret, &second.request_id, true).unwrap();
        assert!(s.can_stream(&st.code, &second.teacher_secret));
        assert!(!s.can_stream(&st.code, &first.teacher_secret));
    }
    #[test]
    fn concurrent_teachers_cannot_bypass_one_pending_request() {
        use std::sync::Barrier;
        use std::thread;
        let sessions = SessionStore::new();
        let student = sessions.create(Duration::from_secs(900)).unwrap();
        let start = Arc::new(Barrier::new(3));
        let workers: Vec<_> = (0..2).map(|_| {
            let sessions = sessions.clone();
            let code = student.code.clone();
            let start = Arc::clone(&start);
            thread::spawn(move || {
                start.wait();
                sessions.request(&code)
            })
        }).collect();
        start.wait();
        let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|r| **r == Err(Error::InvalidState)).count(), 1);
        let winner = results.into_iter().find_map(Result::ok).unwrap();
        assert!(!sessions.can_stream(&student.code, &winner.teacher_secret));
        sessions.cancel_request(&student.code, &winner.teacher_secret).unwrap();
        assert_eq!(sessions.student_status(&student.code, &student.student_secret).unwrap(),
                   (Status::Waiting, None));
        let next = sessions.request(&student.code).unwrap();
        assert_ne!(winner.teacher_secret, next.teacher_secret);
        assert!(!sessions.can_stream(&student.code, &winner.teacher_secret));
        sessions.decide(&student.code, &student.student_secret, &next.request_id, true).unwrap();
        assert!(sessions.can_stream(&student.code, &next.teacher_secret));
    }

    #[test]
    fn expired_approval_never_authorizes_streaming() {
        let sessions = SessionStore::new();
        let student = sessions.create(Duration::from_millis(500)).unwrap();
        let teacher = sessions.request(&student.code).unwrap();
        sessions.decide(&student.code, &student.student_secret, &teacher.request_id, true).unwrap();
        assert!(sessions.can_stream(&student.code, &teacher.teacher_secret));
        std::thread::sleep(Duration::from_millis(650));
        assert!(!sessions.can_stream(&student.code, &teacher.teacher_secret));
        assert_eq!(sessions.teacher_status(&student.code, &teacher.teacher_secret), Ok(Status::Expired));
        assert_eq!(sessions.request(&student.code), Err(Error::Expired));
    }

    #[test]
    fn invalid_and_expired_codes_fail() {
        let s = SessionStore::new();
        assert_eq!(s.request("127.0.0.1:8080"), Err(Error::InvalidCode));
        let st = s.create(Duration::ZERO).unwrap();
        assert_eq!(s.request(&st.code), Err(Error::Expired));
    }
    #[test]
    fn stop_revokes_teacher_bearer_and_pending_request() {
        let sessions = SessionStore::new();
        let student = sessions.create(Duration::from_secs(900)).unwrap();
        let teacher = sessions.request(&student.code).unwrap();
        sessions
            .decide(
                &student.code,
                &student.student_secret,
                &teacher.request_id,
                true,
            )
            .unwrap();
        assert!(sessions.can_stream(&student.code, &teacher.teacher_secret));
        sessions.end(&student.code, &student.student_secret).unwrap();
        assert!(!sessions.can_stream(&student.code, &teacher.teacher_secret));
        assert_eq!(
            sessions.teacher_status(&student.code, &teacher.teacher_secret),
            Err(Error::Unauthorized)
        );
        assert_eq!(
            sessions.student_status(&student.code, &student.student_secret),
            Ok((Status::Ended, None))
        );
        assert_eq!(
            sessions.decide(&student.code, &student.student_secret, &teacher.request_id, true),
            Err(Error::InvalidRequest)
        );
        assert_eq!(sessions.request(&student.code), Err(Error::InvalidState));
    }

    #[test]
    fn stop_clears_unapproved_request_and_blocks_replay() {
        let sessions = SessionStore::new();
        let student = sessions.create(Duration::from_secs(900)).unwrap();
        let teacher = sessions.request(&student.code).unwrap();
        sessions.end(&student.code, &student.student_secret).unwrap();
        assert_eq!(
            sessions.teacher_status(&student.code, &teacher.teacher_secret),
            Err(Error::Unauthorized)
        );
        assert_eq!(
            sessions.student_status(&student.code, &student.student_secret),
            Ok((Status::Ended, None))
        );
        assert_eq!(
            sessions.decide(&student.code, &student.student_secret, &teacher.request_id, true),
            Err(Error::InvalidRequest)
        );
    }

}
