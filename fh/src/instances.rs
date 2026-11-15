//! The launcher's instances, as the page's rows.
//!
//! The list itself is `horizon-launcher`'s `instances::list()` — the same `instances.json` and the
//! same game folders a launcher installed beside this one has, so the page shows exactly the
//! instances Play shows, in its order. Nothing here creates or changes an instance: FH lists what
//! is there and starts one.
//!
//! Each row's picture is drawn here, as an SVG tile: an instance's own colour where it has one,
//! otherwise a colour taken from its name, with the name's first letter on it — the JL mark's
//! shape (white on a solid tile) at a row's size.

use crate::ui::{escape, Row};
use horizon_launcher::instances::{self, Icon, Instance, Kind};

/// One instance, as the page's row.
pub fn row(instance: &Instance) -> Row {
    Row {
        id: instance.id.clone(),
        name: instance.name.clone(),
        kind: kind_name(&instance.kind).to_owned(),
        version: instance.version.clone(),
        initial: initial(&instance.name),
        picture: true,
        java: instance.java.is_some(),
    }
}

/// The instance `id` names, as the page's row.
pub fn find(id: &str) -> Option<Instance> {
    instances::list().into_iter().find(|instance| instance.id == id)
}

/// What `played` says, when it has ever played: "2 h 40 min played".
pub fn played(instance: &Instance) -> Option<String> {
    instance
        .last_played
        .map(|_| instances::play_time(instance.played))
}

/// What a kind of instance is called on the page.
fn kind_name(kind: &Kind) -> &'static str {
    match kind {
        Kind::Default => "Horizon",
        Kind::Own => "Own instance",
        Kind::Port => "Ported pack",
        Kind::Pack => "Modpack",
    }
}

/// The letter a row shows when it has no picture of its own: the name's first letter, upper case.
fn initial(name: &str) -> String {
    name.trim()
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_owned())
}

/// A row's picture, as the SVG the page's `<img>` asks for: a rounded tile in the instance's
/// colour, its letter in white, sized to a 32-pixel row.
pub fn tile(instance: &Instance) -> String {
    let colour = colour(instance);
    let letter = escape(&initial(&instance.name));
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 32 32\" width=\"32\" height=\"32\">\
         <rect width=\"32\" height=\"32\" rx=\"7\" fill=\"{colour}\"/>\
         <text x=\"16\" y=\"16\" fill=\"#ffffff\" font-family=\"Segoe UI,system-ui,sans-serif\" \
         font-size=\"17\" font-weight=\"600\" text-anchor=\"middle\" dominant-baseline=\"central\">\
         {letter}</text></svg>"
    )
}

/// The tile's colour: the instance's own where it has a valid one, else one taken from its name,
/// so a row keeps its colour between starts.
fn colour(instance: &Instance) -> String {
    if let Some(Icon::Colour(colour)) = &instance.icon {
        if is_colour(colour) {
            return colour.clone();
        }
    }
    let seed = match &instance.icon {
        Some(Icon::Texture(texture)) => format!("{texture}{}", instance.id),
        _ => instance.id.clone(),
    };
    // A cheap hash, spread over the hues that read well on JLac's dark panes at full saturation.
    let mut hash: u32 = 2_166_136_261;
    for byte in seed.bytes() {
        hash = (hash ^ byte as u32).wrapping_mul(16_777_619);
    }
    let hue = hash % 360;
    format!("hsl({hue} 42% 46%)")
}

/// `#rrggbb`, the only colour an instance file may name (`Icon::parse`), and the only kind this
/// page puts in an attribute.
fn is_colour(text: &str) -> bool {
    match text.strip_prefix('#') {
        Some(hex) => hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_colour, tile};
    use horizon_launcher::instances::{Icon, Instance, Kind};
    use std::path::PathBuf;
    use std::time::Duration;

    /// The smallest instance the launcher's list could hold: a name, a kind, two folders.
    fn instance(id: &str, name: &str) -> Instance {
        Instance {
            id: id.into(),
            name: name.into(),
            kind: Kind::Own,
            work: PathBuf::new(),
            home: PathBuf::new(),
            icon: None,
            group: None,
            version: None,
            played: Duration::ZERO,
            last_played: None,
            launch: Default::default(),
            java: None,
        }
    }

    /// The tile's colour, as the page's `<img>` receives it.
    fn fill(svg: &str) -> String {
        let start = svg.find("fill=\"").expect("a fill") + 6;
        let rest = &svg[start..];
        rest[..rest.find('"').expect("a closing quote")].to_owned()
    }

    /// The page's pictures are text, and the text is an SVG a browser draws: an instance's own
    /// colour where it has one, and its name's first letter on it.
    #[test]
    fn a_row_draws_its_letter_on_its_colour() {
        let mut own = instance("own:my-pack", "My pack");
        own.icon = Some(Icon::Colour("#3FA7D6".into()));
        let svg = tile(&own);
        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""), "{svg}");
        assert_eq!(fill(&svg), "#3FA7D6");
        assert!(svg.contains(">M</text>"), "{svg}");
        assert!(!svg.contains("<script"), "{svg}");
    }

    /// An instance with no colour of its own keeps one from its name, between starts.
    #[test]
    fn a_name_gives_a_colour_that_stays_the_same() {
        let first = instance("own:first", "First");
        let second = instance("own:second", "Finn");
        let again = instance("own:first", "First");
        assert_eq!(tile(&first), tile(&again), "the same instance, the same picture");
        // Two names with the same letter still read apart by colour.
        assert_eq!(tile(&first).matches(">F</text>").count(), 1);
        assert_ne!(fill(&tile(&first)), fill(&tile(&second)));
        // A name of nothing still has a row, and its letter is a question mark.
        let blank = instance("own:blank", "  ");
        assert!(tile(&blank).contains(">?</text>"), "{}", tile(&blank));
    }

    /// A file written by hand may say anything: only a colour is put in the attribute.
    #[test]
    fn only_a_six_digit_hex_is_a_colour() {
        assert!(is_colour("#3fa7d6") && is_colour("#FFFFFF"));
        assert!(!is_colour("3fa7d6") && !is_colour("#3fa7") && !is_colour("#3fa7dz"));
        assert!(!is_colour("#3fa7d6;fill:url(#x)"));
        let mut odd = instance("own:odd", "Odd");
        odd.icon = Some(Icon::Colour("#fff\" onload=\"x".into()));
        assert!(fill(&tile(&odd)).starts_with("hsl("), "{}", tile(&odd));
    }
}
