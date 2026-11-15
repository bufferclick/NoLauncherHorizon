//! The window: Fabric n Horizon's page, drawn the way JLac draws its own.
//!
//! There is no widget toolkit behind it — the program has a browser and a loopback server — so the
//! window is HTML with JLac's palette and shape: the bar across the top with the mark and the
//! signed-in account, the instance list down the left in a darker pane, and the selected
//! instance's page on the right with one lime Launch button, the colour JLac highlights with.
//!
//! Everything here is a pure function of what it is given, so the pages can be rendered and read
//! without a server, a browser or a game.

use crate::qr;

/// One row of the instance list.
pub struct Row {
    pub id: String,
    pub name: String,
    /// "Default", "Own", "Modpack", "Java" — what kind of instance it is.
    pub kind: String,
    /// The build it plays, when it is pinned to one.
    pub version: Option<String>,
    /// The letter shown when there is no picture.
    pub initial: String,
    /// Whether it has a picture the server can serve (`/icon?id=`).
    pub picture: bool,
    /// A Java (Minecraft) instance: FH lists it, but does not start it yet.
    pub java: bool,
}

/// The window's front page: the list, and the instance in it that is chosen.
pub struct Home<'a> {
    pub token: &'a str,
    pub account: Option<&'a str>,
    pub rows: &'a [Row],
    pub selected: Option<&'a Row>,
    pub home: Option<&'a str>,
    pub played: Option<&'a str>,
    /// A line under the buttons: the last thing that happened, or nothing.
    pub note: Option<&'a str>,
    pub footer: &'a str,
}

/// What the work page is doing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Signing in, downloading, preparing: the page keeps reloading.
    Working,
    /// The game was started; the tab has done its job.
    Started,
    /// The game ended on its own: `line` says how it went.
    Done,
    /// It stopped: `error` says why.
    Failed,
}

/// The page a Launch turns into: the sign-in's code, then the work, then the game.
pub struct Job<'a> {
    pub account: Option<&'a str>,
    pub instance: &'a str,
    pub title: &'a str,
    pub line: &'a str,
    pub fraction: Option<f32>,
    /// The code to type at Microsoft's page, while one is waiting.
    pub code: Option<(&'a str, &'a str)>,
    pub stage: Stage,
    pub error: Option<&'a str>,
    pub footer: &'a str,
}

/// The mark, inline: white F and H on black, the tile JLac's own icon is drawn in.
const MARK: &str = "\
<svg viewBox=\"0 0 64 64\" width=\"26\" height=\"26\" aria-hidden=\"true\">\
<rect width=\"64\" height=\"64\" rx=\"14\" fill=\"#0b0b0b\"/>\
<g fill=\"#ffffff\">\
<rect x=\"12\" y=\"18\" width=\"6\" height=\"28\"/>\
<rect x=\"12\" y=\"18\" width=\"20\" height=\"6\"/>\
<rect x=\"12\" y=\"29\" width=\"16\" height=\"6\"/>\
<rect x=\"38\" y=\"18\" width=\"6\" height=\"28\"/>\
<rect x=\"50\" y=\"18\" width=\"6\" height=\"28\"/>\
<rect x=\"38\" y=\"29\" width=\"18\" height=\"6\"/>\
</g></svg>";

/// The front page.
pub fn home(page: &Home) -> String {
    let rows: String = page.rows.iter().map(|row| self::row(row, page)).collect();
    let pane = match page.selected {
        None => "<h1>No instances yet</h1>\
                 <p class=\"note\">Horizon makes its own instance the first time the game runs. \
                 Start the launcher once, or launch from a terminal, and it appears here.</p>"
            .to_owned(),
        Some(row) => pane(row, page),
    };
    let body = format!(
        "<div class=\"main\">\
           <aside class=\"side\">\
             <div class=\"sideHead\">Instances</div>\
             <nav class=\"list\">{rows}</nav>\
             <div class=\"soon\">Fabric instances join this list next.</div>\
           </aside>\
           <section class=\"pane\">{pane}</section>\
         </div>",
    );
    window("Fabric n Horizon", "", page.account, &body, page.footer)
}

/// One row: its picture (or its letter), its name, and what it is.
fn row(row: &Row, page: &Home) -> String {
    let selected = page
        .selected
        .map(|chosen| chosen.id == row.id)
        .unwrap_or(false);
    let picture = if row.picture {
        format!(
            "<img class=\"ico\" src=\"/icon?k={token}&id={id}\" alt=\"\">",
            token = escape(page.token),
            id = encode_uri(&row.id),
        )
    } else {
        format!(
            "<span class=\"ico letter\">{initial}</span>",
            initial = escape(&row.initial),
        )
    };
    let under = match (&row.version, row.java) {
        (_, true) => format!("Java · {}", escape(&row.kind)),
        (Some(version), false) => format!("{} · {}", escape(&row.kind), escape(version)),
        (None, false) => escape(&row.kind),
    };
    format!(
        "<a class=\"row{selected}\" href=\"/?k={token}&id={id}\">{picture}\
         <span class=\"names\"><b>{name}</b><small>{under}</small></span></a>",
        selected = if selected { " selected" } else { "" },
        token = escape(page.token),
        id = encode_uri(&row.id),
        picture = picture,
        name = escape(&row.name),
        under = under,
    )
}

/// The right-hand page of the chosen instance.
fn pane(row: &Row, page: &Home) -> String {
    let meta = {
        let mut parts = vec![escape(&row.kind)];
        if let Some(version) = &row.version {
            parts.push(escape(version));
        }
        if let Some(played) = page.played {
            parts.push(format!("{played} played"));
        }
        parts.join(" · ")
    };
    let launch = if row.java {
        "<span class=\"launch off\">Not yet</span>\
         <span class=\"why\">FH starts Horizon instances. Minecraft ones — Fabric included — \
         join it next.</span>"
            .to_owned()
    } else {
        format!(
            "<a class=\"launch\" href=\"/launch?k={token}&id={id}\">Launch</a>\
             <a class=\"quiet\" href=\"/folder?k={token}&id={id}\">Open folder</a>",
            token = escape(page.token),
            id = encode_uri(&row.id),
        )
    };
    let note = match page.note {
        Some(note) if !note.is_empty() => format!(
            "<p class=\"note after\">{note}</p>",
            note = escape(note)
        ),
        _ => String::new(),
    };
    format!(
        "<h1>{name}</h1><p class=\"meta\">{meta}</p><p class=\"path\">{path}</p>\
         <div class=\"actions\">{launch}</div>{note}",
        name = escape(&row.name),
        path = escape(page.home.unwrap_or("")),
    )
}

/// The work page.
pub fn job(page: &Job) -> String {
    let refresh = match page.stage {
        Stage::Working => " working",
        Stage::Started | Stage::Done | Stage::Failed => "",
    };
    let bar = match page.fraction {
        Some(fraction) => format!(
            "<div class=\"progress\"><i style=\"width:{:.0}%\"></i></div>",
            fraction.clamp(0., 1.) * 100.
        ),
        None => String::new(),
    };
    let sign_in = match page.code {
        Some((code, link)) => sign_in_block(code, link),
        None => String::new(),
    };
    let end = match (page.stage, page.error) {
        (Stage::Started, _) => "<p class=\"wait\">Horizon is starting. This tab can be closed.</p>\
                               <p class=\"after\"><a class=\"quiet\" href=\"/\">Back to the \
                               instances</a></p>"
            .to_owned(),
        (Stage::Done, _) => format!(
            "<p class=\"wait\">{line}</p>\
             <p class=\"after\"><a class=\"quiet\" href=\"/\">Back to the instances</a></p>",
            line = escape(page.line)
        ),
        (Stage::Failed, Some(error)) => format!(
            "<p class=\"note danger\">{error}</p>\
             <p class=\"after\"><a class=\"quiet\" href=\"/\">Back to the instances</a></p>",
            error = escape(error)
        ),
        (Stage::Failed, None) => "<p class=\"note danger\">It stopped.</p>".to_owned(),
        (Stage::Working, _) => format!(
            "<p class=\"wait\">{line}</p>",
            line = escape(page.line)
        ),
    };
    let body = format!(
        "<div class=\"main\">\
           <aside class=\"side\">\
             <div class=\"sideHead\">Launching</div>\
             <div class=\"chosen\">{name}</div>\
             <div class=\"soon\"><a href=\"/\">All instances</a></div>\
           </aside>\
           <section class=\"pane\">\
             <h1>{title}</h1>{sign_in}{bar}{end}\
           </section>\
         </div>",
        name = escape(page.instance),
        title = escape(page.title),
    );
    window(
        &format!("{} — FH", page.title),
        refresh,
        page.account,
        &body,
        page.footer,
    )
}

/// The code, the Copy button, the QR code of Microsoft's page and the address itself.
fn sign_in_block(code: &str, link: &str) -> String {
    let mut block = format!(
        "<p class=\"note\">Enter this code on Microsoft's page:</p>\
         <p class=\"codeline\"><span class=\"code\">{code}</span>\
         <button class=\"copy\" type=\"button\" data-code=\"{code}\" \
         onclick=\"copyCode(this)\">Copy</button></p>",
        code = escape(code),
    );
    if let Some(drawing) = qr::for_link(link) {
        block.push_str("<p class=\"qr\">");
        block.push_str(drawing);
        block.push_str("</p>");
    }
    block.push_str(&format!(
        "<p class=\"note\">Open <a href=\"{open}\" target=\"_blank\" rel=\"noreferrer\">{link}</a> \
         or scan the QR and enter the code above if needed.</p>",
        open = escape(&open_url(link, code)),
        link = escape(link),
    ));
    block
}

/// The address the sign-in opens on: Microsoft's link page takes the code in the address
/// (`?otc=`), so the player signs in without typing it.
pub fn open_url(link: &str, code: &str) -> String {
    if code.is_empty() || !qr::is_link_page(link) {
        return link.to_owned();
    }
    let separator = if link.contains('?') { '&' } else { '?' };
    format!("{link}{separator}otc={}", uri(code))
}

/// A page that is not one of the window's.
pub fn missing(asked: &str, footer: &str) -> String {
    let body = format!(
        "<div class=\"main\"><section class=\"pane\"><h1>Not here</h1>\
         <p class=\"note\">{asked} is not a page this window has.</p>\
         <p class=\"after\"><a class=\"quiet\" href=\"/\">Back to the instances</a></p>\
         </section></div>",
        asked = escape(asked)
    );
    window("Not here", "", None, &body, footer)
}

/// The window every page sits in: JLac's bars, JLac's panes, the account on the right.
fn window(title: &str, refresh: &str, account: Option<&str>, body: &str, footer: &str) -> String {
    let meta = if refresh.is_empty() {
        String::new()
    } else {
        "<meta http-equiv=\"refresh\" content=\"1\">".to_owned()
    };
    let account = match account {
        Some(name) => format!(
            "<span class=\"chip\"><span class=\"dot\"></span>{name}</span>",
            name = escape(name)
        ),
        None => "<span class=\"chip dim\">not signed in</span>".to_owned(),
    };
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n{meta}\n\
         <title>{title}</title>\n<style>{css}</style>\n</head>\n<body>\n\
         <header class=\"bar\"><span class=\"mark\">{mark}</span>\
         <span class=\"app\">Fabric <i>n</i> Horizon</span>\
         <span class=\"grow\"></span>{account}</header>\n\
         {body}\n<footer class=\"foot\">{footer}</footer>\n{script}\n</body>\n</html>\n",
        title = escape(title),
        css = CSS,
        mark = MARK,
        account = account,
        body = body,
        footer = escape(footer),
        script = SCRIPT,
    )
}

/// `text` with the characters HTML reads as markup.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// `text` safe to put in an address's query: an instance id is a folder name.
fn uri(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b':') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The same, for an address's path and query alike: `:` and `.` survive, a space does not.
pub fn encode_uri(text: &str) -> String {
    uri(text)
}

/// The look: JLac's own palette — its dark greys, its lime highlight and black text on it, its
/// teal links — with the window filling the browser and the two panes a launcher has.
const CSS: &str = "\
:root{color-scheme:dark}\
*{box-sizing:border-box}\
body{margin:0;min-height:100vh;display:flex;flex-direction:column;background:#222222;\
color:#ffffff;font:14px/1.45 system-ui,\"Segoe UI\",sans-serif}\
a{color:#2fa3c6;text-decoration:none}\
.bar{display:flex;align-items:center;gap:10px;padding:8px 12px;background:#313131;\
border-bottom:1px solid #3c3c3c}\
.bar .mark{display:flex}\
.app{font-weight:600;letter-spacing:.01em}\
.app i{color:#96db59;font-style:normal}\
.grow{flex:1}\
.chip{display:inline-flex;align-items:center;gap:8px;padding:5px 10px;border-radius:4px;\
background:#3c3c3c;color:#ffffff}\
.chip.dim{color:#a6a6a6}\
.dot{width:8px;height:8px;border-radius:50%;background:#96db59}\
.main{flex:1;display:flex;min-height:0}\
.side{width:300px;flex:none;display:flex;flex-direction:column;background:#1b1b1b;\
border-right:1px solid #3c3c3c}\
.sideHead{padding:10px 14px 6px;font-size:11px;letter-spacing:.14em;text-transform:uppercase;\
color:#a6a6a6}\
.list{flex:1;overflow:auto;padding:4px 6px}\
.row{display:flex;align-items:center;gap:10px;padding:7px 8px;border-radius:4px;\
color:#ffffff;border:1px solid transparent}\
.row:hover{background:#2a2a2a}\
.row.selected{background:#96db59;color:#101010}\
.row.selected small{color:#20400a}\
.ico{width:32px;height:32px;flex:none;border-radius:3px;background:#2a2a2a;object-fit:cover}\
.ico.letter{display:grid;place-items:center;font-weight:700;color:#96db59}\
.row.selected .ico.letter{color:#101010}\
.names{display:flex;flex-direction:column;min-width:0}\
.names b{font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}\
.names small{color:#a6a6a6;font-size:12px}\
.soon{padding:10px 14px;font-size:12px;color:#8a8a8a;border-top:1px solid #2c2c2c}\
.chosen{padding:0 14px 14px;font-weight:600}\
.pane{flex:1;overflow:auto;padding:26px 30px;min-width:0}\
.pane h1{margin:0 0 6px;font-size:20px;font-weight:600}\
.meta{margin:0 0 4px;color:#a6a6a6}\
.path{margin:0 0 22px;color:#8a8a8a;font-size:12px;word-break:break-all}\
.actions{display:flex;align-items:center;gap:12px;margin:0 0 22px}\
.launch{display:inline-block;padding:9px 26px;border-radius:4px;background:#96db59;\
color:#101010;font-weight:700}\
.launch.off{background:#3c3c3c;color:#a6a6a6}\
.quiet{display:inline-block;padding:8px 14px;border-radius:4px;background:#3c3c3c;\
color:#ffffff}\
.why{color:#a6a6a6}\
.note{margin:0 0 14px;color:#c9c9c9}\
.note.after{margin-top:18px}\
.danger{color:#ff9a8a}\
.after{margin:18px 0 0}\
.wait{margin:16px 0 0;color:#8a8a8a}\
.codeline{display:flex;align-items:center;gap:10px}\
.code{padding:8px 14px;border-radius:4px;border:1px solid #3c3c3c;background:#1b1b1b;\
color:#ffffff;letter-spacing:.12em;font:600 26px/1.2 ui-monospace,Consolas,monospace}\
.copy{padding:7px 12px;border-radius:4px;border:1px solid #3c3c3c;background:#313131;\
color:#ffffff;font:inherit;cursor:pointer}\
.qr{margin:0 0 14px}\
.qr svg{width:150px;height:150px;border-radius:4px}\
.progress{height:6px;margin:0 0 14px;border-radius:99px;background:#2a2a2a;overflow:hidden}\
.progress i{display:block;height:100%;background:#96db59}\
.foot{padding:7px 12px;background:#313131;border-top:1px solid #3c3c3c;color:#8a8a8a;\
font-size:12px;word-break:break-word}";

/// The page's only script: the Copy button beside a sign-in code.
const SCRIPT: &str = r#"<script>
function copyCode(button) {
  var code = button.getAttribute('data-code');
  var copied = function () {
    button.textContent = 'Copied';
    setTimeout(function () { button.textContent = 'Copy'; }, 1500);
  };
  var fallback = function () {
    var area = document.createElement('textarea');
    area.value = code;
    document.body.appendChild(area);
    area.select();
    try { document.execCommand('copy'); copied(); } catch (e) {}
    document.body.removeChild(area);
  };
  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(code).then(copied, fallback);
  } else {
    fallback();
  }
}
</script>"#;

#[cfg(test)]
mod tests {
    use super::{escape, home, job, open_url, Job, Row, Stage, Home};

    fn rows() -> Vec<Row> {
        vec![
            Row {
                id: "horizon".into(),
                name: "Horizon".into(),
                kind: "Default".into(),
                version: Some("nightly-2026-11-15-abc1234".into()),
                initial: "H".into(),
                picture: false,
                java: false,
            },
            Row {
                id: "instances/my-pack".into(),
                name: "My <Pack> & co".into(),
                kind: "Modpack".into(),
                version: None,
                initial: "M".into(),
                picture: true,
                java: false,
            },
            Row {
                id: "java:1.21".into(),
                name: "Fabric 1.21".into(),
                kind: "Own".into(),
                version: None,
                initial: "F".into(),
                picture: false,
                java: true,
            },
        ]
    }

    fn page(selected: usize) -> String {
        let rows = rows();
        home(&Home {
            token: "TOKEN",
            account: Some("Notch"),
            rows: &rows,
            selected: Some(&rows[selected]),
            home: Some("C:\\Users\\Doue\\AppData\\Roaming\\horizon-launcher\\horizon"),
            played: Some("3 h 12 min"),
            note: None,
            footer: "FH v0.1 · log: fh.log",
        })
    }

    /// The window is the list, the chosen instance, its Launch, and the account: JLac's shape.
    #[test]
    fn the_window_lists_the_instances_and_the_one_that_can_be_launched() {
        let html = page(0);
        assert!(html.contains("Horizon"), "{html}");
        assert!(html.contains("My &lt;Pack&gt; &amp; co"), "a name is text: {html}");
        assert!(html.contains("class=\"row selected\""), "{html}");
        // The chosen one's Launch carries the token and its id; the row does too.
        assert!(
            html.contains("href=\"/launch?k=TOKEN&amp;id=horizon\"")
                || html.contains("href=\"/launch?k=TOKEN&id=horizon\""),
            "{html}"
        );
        assert!(html.contains("nightly-2026-11-15-abc1234"), "{html}");
        assert!(html.contains("3 h 12 min"), "{html}");
        assert!(html.contains("Notch"), "{html}");
        assert!(html.contains("Fabric instances join this list next."), "{html}");
    }

    /// A Java instance is listed and never started as Horizon — the page says so.
    #[test]
    fn a_java_instance_is_shown_but_not_launchable() {
        let html = page(2);
        assert!(html.contains("Not yet"), "{html}");
        assert!(!html.contains("/launch?k=TOKEN&id=java"), "{html}");
        assert!(html.contains("Fabric included"), "{html}");
    }

    /// An instance with a picture asks the server for it, by id.
    #[test]
    fn a_picture_is_fetched_from_the_server() {
        let html = page(1);
        assert!(
            html.contains("src=\"/icon?k=TOKEN&amp;id=instances%2Fmy-pack\"")
                || html.contains("src=\"/icon?k=TOKEN&id=instances%2Fmy-pack\""),
            "{html}"
        );
    }

    /// A sign-in's code is on the page with the Copy button and the QR code, and the work page
    /// keeps reloading until the game starts.
    #[test]
    fn a_sign_in_on_the_work_page_has_the_code_and_the_qr() {
        let working = job(&Job {
            account: None,
            instance: "Horizon",
            title: "Signing in with Microsoft",
            line: "Asking Microsoft for a code…",
            fraction: None,
            code: Some(("72J7EFQU", "https://www.microsoft.com/link")),
            stage: Stage::Working,
            error: None,
            footer: "FH v0.1",
        });
        assert!(working.contains("72J7EFQU"), "{working}");
        assert!(working.contains("http-equiv=\"refresh\""), "{working}");
        assert!(
            working.contains("<svg") && working.contains("stroke=\"#000"),
            "the QR code is not there"
        );
        assert!(
            working.contains("otc=72J7EFQU"),
            "the address does not carry the code: {working}"
        );
        // Started and failed are the end of the work: no more reloading.
        for (stage, error) in [
            (Stage::Started, None),
            (Stage::Failed, Some("Horizon ended (exit status: 1).")),
        ] {
            let html = job(&Job {
                account: Some("Notch"),
                instance: "Horizon",
                title: "Starting Horizon",
                line: "Starting",
                fraction: Some(0.5),
                code: None,
                stage,
                error,
                footer: "FH v0.1",
            });
            assert!(!html.contains("http-equiv=\"refresh\""), "{html}");
            assert!(html.contains("/\">Back to the instances") || html.contains("href=\"/\""), "{html}");
        }
    }

    /// The address the browser is sent to carries the code; another page keeps its own.
    #[test]
    fn the_link_page_is_opened_with_the_code_in_it() {
        assert_eq!(
            open_url("https://www.microsoft.com/link", "72J7EFQU"),
            "https://www.microsoft.com/link?otc=72J7EFQU"
        );
        assert_eq!(
            open_url("https://microsoft.com/devicelogin", "72J7EFQU"),
            "https://microsoft.com/devicelogin"
        );
        assert_eq!(escape("\"'&<>"), "&quot;&#39;&amp;&lt;&gt;");
    }
}
