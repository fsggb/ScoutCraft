//! Nouveau meuble : the scout furniture dialog (SoLeadWorks). A template, a name, the three overall
//! dimensions, the three pole diameters, and where it goes (position and quarter turn). OK runs
//! `scout.meuble.create`; every template button of the Scout tab opens this dialog on its type.

use serde_json::{Value, json};
use solidcraft_engine::scout::templates::{Dims, TEMPLATES, template};

use crate::SolveApp;
use crate::dialogs::Kind;
use crate::selection::SelInput;
use crate::ui_kit::{choice_row, text_row};

const ROTATIONS: [&str; 4] = ["0°", "90°", "180°", "270°"];

#[derive(Clone, Debug)]
pub struct Mb {
    pub kind: usize,
    /// The template the fields hold the defaults of (they refill when the type changes).
    filled_for: Option<usize>,
    pub name: String,
    pub length: String,
    pub width: String,
    pub height: String,
    pub leg: String,
    pub rail: String,
    pub claie: String,
    pub x: String,
    pub y: String,
    pub rotation: usize,
}

impl Mb {
    fn new(kind: usize) -> Self {
        let mut m = Mb {
            kind,
            filled_for: None,
            name: String::new(),
            length: String::new(),
            width: String::new(),
            height: String::new(),
            leg: String::new(),
            rail: String::new(),
            claie: String::new(),
            x: "0".into(),
            y: "0".into(),
            rotation: 0,
        };
        m.refill();
        m
    }

    fn refill(&mut self) {
        let Some(t) = TEMPLATES.get(self.kind) else { return };
        let d = Dims::of(t);
        let n = |v: f64| format!("{v}");
        (self.length, self.width, self.height) = (n(d.length), n(d.width), n(d.height));
        (self.leg, self.rail, self.claie) = (n(d.leg), n(d.rail), n(d.claie));
        self.filled_for = Some(self.kind);
    }
}

/// The dialog for the Scout tab's furniture buttons.
pub fn start(_app: &SolveApp, id: &str) -> Option<(Kind, Vec<SelInput>)> {
    let wanted = id.strip_prefix("scout.meuble.")?;
    let kind = if wanted == "create" { 0 } else { TEMPLATES.iter().position(|t| t.id == wanted)? };
    Some((Kind::Meuble(Mb::new(kind)), Vec::new()))
}

pub fn title() -> &'static str {
    "NOUVEAU MEUBLE"
}

fn number(label: &str, text: &str) -> Result<f64, String> {
    text.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite()).ok_or_else(|| format!("{label} : un nombre en millimètres est attendu"))
}

/// The dialog's rows.
pub fn rows(ui: &mut egui::Ui, k: &mut Mb) -> bool {
    let names: Vec<&str> = TEMPLATES.iter().map(|t| t.name).collect();
    choice_row(ui, "Meuble", "mb_type", &mut k.kind, &names);
    if k.filled_for != Some(k.kind) {
        k.refill();
    }
    if let Some(t) = TEMPLATES.get(k.kind) {
        ui.label(egui::RichText::new(t.summary).weak());
    }
    text_row(ui, "Nom", &mut k.name, template_name(k.kind));
    text_row(ui, "Longueur (mm)", &mut k.length, "");
    text_row(ui, "Largeur (mm)", &mut k.width, "");
    text_row(ui, "Hauteur (mm)", &mut k.height, "");
    text_row(ui, "Ø pieds (mm)", &mut k.leg, "");
    text_row(ui, "Ø traverses (mm)", &mut k.rail, "");
    text_row(ui, "Ø claies (mm)", &mut k.claie, "");
    text_row(ui, "Position X (mm)", &mut k.x, "");
    text_row(ui, "Position Y (mm)", &mut k.y, "");
    choice_row(ui, "Rotation", "mb_rot", &mut k.rotation, &ROTATIONS);
    false
}

fn template_name(kind: usize) -> &'static str {
    TEMPLATES.get(kind).map_or("Meuble", |t| t.name)
}

/// The command OK runs.
pub fn commands(k: &Mb) -> Result<Vec<(String, Value)>, String> {
    let t = TEMPLATES.get(k.kind).and_then(|t| template(t.id)).ok_or("choisissez un meuble")?;
    let rotation: u32 = [0, 90, 180, 270].get(k.rotation).copied().unwrap_or(0);
    let mut p = json!({
        "type": t.id,
        "length": number("Longueur", &k.length)?,
        "width": number("Largeur", &k.width)?,
        "height": number("Hauteur", &k.height)?,
        "leg": number("Ø pieds", &k.leg)?,
        "rail": number("Ø traverses", &k.rail)?,
        "claie": number("Ø claies", &k.claie)?,
        "at": [number("Position X", &k.x)?, number("Position Y", &k.y)?],
        "rotation": rotation,
    });
    if !k.name.trim().is_empty() {
        p["name"] = json!(k.name.trim());
    }
    Ok(vec![("scout.meuble.create".into(), p)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_make_a_valid_command() {
        for kind in 0..TEMPLATES.len() {
            let c = commands(&Mb::new(kind)).unwrap_or_default();
            assert_eq!(c.len(), 1);
            let mut s = solidcraft_engine::Session::default();
            assert!(s.execute(&c[0].0, &c[0].1).is_ok(), "{}", c[0].1);
        }
    }

    #[test]
    fn a_typed_comma_and_a_bad_number() {
        let mut m = Mb::new(0);
        m.length = "1500,5".into();
        assert!(commands(&m).is_ok());
        m.length = "long".into();
        assert!(commands(&m).is_err());
    }
}
