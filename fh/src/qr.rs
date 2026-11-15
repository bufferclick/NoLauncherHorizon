//! The QR code on FH's sign-in page: Microsoft's own link page, dark modules on white.
//!
//! It is a constant, not a drawing: the code the player types changes every start and the page the
//! QR opens is the same one every time, so drawing it in the program would be work for nothing.
//! `nolauncher/icons/make_qr.py` prints this file again from `segno` (the module matrix here is
//! that generator's, checked by decoding it back to the address).

/// Microsoft's link page: where the sign-in's code is typed, for either kind of Microsoft app.
pub const LINK_PAGE: &str = "https://www.microsoft.com/link";

/// [`LINK_PAGE`] as a 29 × 29 QR code, error correction Q, with a two-module quiet zone.
const LINK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="33" height="33"><path fill="#fff" d="M0 0h33v33h-33z"/><path stroke="#000" d="M2 2.5h7m2 0h1m2 0h1m3 0h1m1 0h1m3 0h7m-29 1h1m5 0h1m3 0h2m1 0h2m1 0h2m1 0h2m1 0h1m5 0h1m-29 1h1m1 0h3m1 0h1m1 0h1m1 0h3m1 0h2m1 0h2m1 0h1m1 0h1m1 0h3m1 0h1m-29 1h1m1 0h3m1 0h1m2 0h2m1 0h2m5 0h2m1 0h1m1 0h3m1 0h1m-29 1h1m1 0h3m1 0h1m1 0h2m3 0h2m2 0h2m3 0h1m1 0h3m1 0h1m-29 1h1m5 0h1m1 0h3m2 0h1m1 0h2m1 0h2m2 0h1m5 0h1m-29 1h7m1 0h1m1 0h1m1 0h1m1 0h1m1 0h1m1 0h1m1 0h1m1 0h7m-20 1h5m1 0h6m-20 1h1m2 0h1m1 0h1m1 0h3m1 0h1m1 0h1m2 0h1m1 0h3m1 0h2m1 0h1m-27 1h3m2 0h1m2 0h1m2 0h2m1 0h1m2 0h2m3 0h7m-26 1h2m1 0h2m2 0h4m1 0h1m2 0h1m1 0h2m1 0h4m1 0h1m-27 1h3m2 0h3m3 0h2m2 0h1m1 0h3m2 0h2m1 0h2m-29 1h3m3 0h4m1 0h1m1 0h1m1 0h1m1 0h1m5 0h1m1 0h1m2 0h1m-29 1h1m1 0h1m1 0h1m2 0h1m1 0h1m1 0h2m1 0h1m2 0h2m3 0h1m1 0h1m1 0h3m-28 1h1m1 0h1m2 0h1m1 0h1m1 0h1m1 0h1m1 0h1m1 0h1m1 0h1m1 0h3m1 0h1m3 0h1m-29 1h1m1 0h3m2 0h4m1 0h1m1 0h4m1 0h2m2 0h1m1 0h1m-26 1h1m1 0h1m3 0h4m2 0h4m1 0h1m1 0h3m5 0h2m-29 1h3m4 0h4m5 0h3m1 0h1m1 0h1m1 0h1m3 0h1m-27 1h1m3 0h1m2 0h2m1 0h2m2 0h3m1 0h1m2 0h1m1 0h2m1 0h1m-25 1h2m2 0h1m2 0h2m1 0h1m2 0h2m1 0h1m4 0h1m1 0h1m-28 1h2m2 0h3m1 0h1m3 0h2m1 0h3m2 0h6m1 0h2m-21 1h4m3 0h4m1 0h1m3 0h1m1 0h1m1 0h1m-29 1h7m3 0h2m2 0h2m3 0h2m1 0h1m1 0h1m3 0h1m-29 1h1m5 0h1m5 0h3m1 0h5m3 0h2m1 0h1m-28 1h1m1 0h3m1 0h1m1 0h2m1 0h3m4 0h7m-25 1h1m1 0h3m1 0h1m5 0h1m1 0h1m2 0h2m4 0h1m2 0h2m-28 1h1m1 0h3m1 0h1m2 0h2m1 0h1m2 0h3m1 0h3m4 0h3m-29 1h1m5 0h1m1 0h4m2 0h5m1 0h1m2 0h1m1 0h1m1 0h2m-29 1h7m3 0h3m4 0h1m2 0h3m1 0h1m2 0h1"/></svg>"##;

/// Whether `link` is Microsoft's link page, however the service wrote it: a trailing slash, the
/// code in its query, capitals. Nothing else is (`microsoft.com/devicelogin` is the Azure apps'
/// page, and a look-alike host is a look-alike).
pub fn is_link_page(link: &str) -> bool {
    let address = link
        .split(|c| c == '?' || c == '#')
        .next()
        .unwrap_or_default();
    address.trim_end_matches('/').eq_ignore_ascii_case(LINK_PAGE)
}

/// The QR code for `link`: it is there when the address is Microsoft's link page, and left out
/// for any other verification page (the page shows that address to type instead).
pub fn for_link(link: &str) -> Option<&'static str> {
    if is_link_page(link) {
        Some(LINK)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{for_link, is_link_page, LINK, LINK_PAGE};

    /// The link page has one, and it is a drawing of a QR code: an SVG with dark modules on a
    /// white square, so it scans off a dark page.
    #[test]
    fn the_link_page_has_a_qr_code_beside_it() {
        let qr = for_link(LINK_PAGE).expect("the link page has a QR code");
        assert!(qr.starts_with("<svg") && qr.ends_with("</svg>"), "{qr}");
        assert!(qr.contains("fill=\"#fff\""), "no white background: {qr}");
        assert!(qr.contains("stroke=\"#000"), "no dark modules: {qr}");
        assert!(qr.contains("width=\"33\" height=\"33\""), "{qr}");
    }

    /// A page that is not the link page gets none — and neither does a look-alike address.
    #[test]
    fn only_microsofts_link_page_has_one() {
        assert!(for_link("https://microsoft.com/devicelogin").is_none());
        assert!(for_link("https://www.microsoft.com/link.example.net").is_none());
        assert!(for_link("https://www.microsoft.com/link/").is_some());
        assert!(for_link("HTTPS://WWW.MICROSOFT.COM/LINK").is_some());
        // The code in the address, which is how the browser is sent there, changes nothing.
        assert!(for_link("https://www.microsoft.com/link?otc=72J7EFQU").is_some());
        assert!(is_link_page("https://www.microsoft.com/link?otc=72J7EFQU"));
        assert!(!is_link_page("https://www.microsoft.com/link.example.net/link"));
        assert_eq!(for_link(LINK_PAGE), Some(LINK));
    }
}
