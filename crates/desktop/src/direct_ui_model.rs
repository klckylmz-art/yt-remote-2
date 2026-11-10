//! Invitation editing for the native direct-mode launcher.
pub fn pasted_invitation(raw: &str) -> Result<String, &'static str> {
    let code = raw.trim();
    let payload = code
        .strip_prefix("YTR3.")
        .ok_or("Paste a YT Remote 3.0 invitation.")?;
    if code.len() > 8192
        || payload.is_empty()
        || !payload
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("The invitation is incomplete or invalid. Copy it again.");
    }
    Ok(code.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paste_preserves_case_sensitive_invitation_and_url_safe_symbols() {
        assert_eq!(
            pasted_invitation("  YTR3.AbCd-12_ef\r\n").unwrap(),
            "YTR3.AbCd-12_ef"
        );
    }

    #[test]
    fn paste_never_silently_truncates_a_long_certificate_bearing_code() {
        let code = format!("YTR3.{}", "Ab_cd-12".repeat(150));
        assert_eq!(pasted_invitation(&code).unwrap(), code);
        assert!(pasted_invitation(&format!("YTR3.{}", "a".repeat(8192))).is_err());
    }

    #[test]
    fn arbitrary_links_unicode_and_embedded_control_characters_are_rejected() {
        for value in [
            "https://example.org",
            "YTR3.abc\ndef",
            "YTR3.ab c",
            "YTR3.é",
            "YTR3.",
            "ytr3.abc",
        ] {
            assert!(pasted_invitation(value).is_err(), "{value:?}");
        }
    }
}
