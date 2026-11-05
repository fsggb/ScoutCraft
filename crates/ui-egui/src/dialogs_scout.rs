//! The scout dialogs (SoLeadWorks): the functions of CathIA, one dialog each. New furniture (the eleven
//! models and their settings), an assemblage between two poles picked in the view (half-lap, the
//! tenons, the dovetails, pegs, lashing), shaping one pole (split, quarter, cut, point), and the
//! simple pieces (a pole, a set square, a deck of claies, a net, a patrol corner). OK runs the
//! engine command; picking a pole in the view picks the body, which is named after its component.

use serde_json::{Value, json};
use solidcraft_engine::Sel;
use solidcraft_engine::scout::models::{MODELS, ParamDef};

use crate::SolveApp;
use crate::dialogs::{Kind, check};
use crate::selection::{BODIES, SelInput};
use crate::ui_kit::{choice_row, text_row};

const ROTATIONS: [&str; 4] = ["0°", "90°", "180°", "270°"];

/// The assemblages, as CathIA lists them: the engine's `kind` and the label.
pub const ASSEMBLAGES: &[(&str, &str)] = &[
    ("mibois", "Mi-bois"),
    ("hache", "Tenon classique (hachette)"),
    ("scie", "Tenon scié"),
    ("plat", "Tenon plat (hachette, embase conique)"),
    ("platscie", "Tenon plat scié"),
    ("aronde1", "Queue d’aronde classique"),
    ("aronde2", "Queue d’aronde à mi-bois (traversante)"),
    ("aronde3", "Queue d’aronde trapézoïdale dans l’axe"),
    ("cheville", "Assemblage par chevilles"),
    ("brelage", "Brêlage (dernier recours)"),
];

const FACONNAGES: &[(&str, &str)] = &[("demi", "Fendre en deux"), ("quart", "Quart de bois"), ("couper", "Couper à la longueur"), ("pointe", "Tailler une pointe")];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Which {
    Meuble,
    Assemblage,
    Faconner,
    Traverse,
    Equerre,
    Claies,
    Filet,
    Coin,
}

/// One dialog's values (the fields it does not use stay unused).
#[derive(Clone, Debug)]
pub struct Sc {
    pub which: Which,
    /// Meuble: the model; Assemblage / Façonner: the kind (an index).
    pub kind: usize,
    filled_for: Option<usize>,
    pub name: String,
    /// Meuble: one text per setting of the model (numbers), or the chosen option's index.
    pub vals: Vec<String>,
    pub x: String,
    pub y: String,
    pub rotation: usize,
    // assemblage and shaping
    pub through: bool,
    pub pc: String,
    pub d: String,
    pub turns: String,
    pub length: String,
    pub end: usize,
    // simple pieces
    pub a: String,
    pub b: String,
    pub c: String,
    pub dir: usize,
}

impl Sc {
    fn new(which: Which, kind: usize) -> Self {
        let (a, b, c) = match which {
            Which::Traverse => ("80".into(), "1500".into(), String::new()),
            Which::Equerre => ("300".into(), "50".into(), String::new()),
            Which::Claies => ("1000".into(), "600".into(), "20".into()),
            Which::Filet => ("1000".into(), "600".into(), "30".into()),
            _ => (String::new(), String::new(), String::new()),
        };
        let mut s = Sc {
            which,
            kind,
            filled_for: None,
            name: String::new(),
            vals: Vec::new(),
            x: "0".into(),
            y: "0".into(),
            rotation: 0,
            through: false,
            pc: "50".into(),
            d: String::new(),
            turns: "6".into(),
            length: "500".into(),
            end: 1,
            a,
            b,
            c,
            dir: 0,
        };
        s.refill();
        s
    }

    fn refill(&mut self) {
        if self.which != Which::Meuble {
            return;
        }
        if let Some(m) = MODELS.get(self.kind) {
            self.vals = m.params.iter().map(|p| format!("{}", p.default)).collect();
            self.filled_for = Some(self.kind);
        }
    }
}

/// The dialog a Scout ribbon button starts.
pub fn start(_app: &SolveApp, id: &str) -> Option<(Kind, Vec<SelInput>)> {
    let two = || vec![SelInput::new("Pièce 1 (porte le tenon)", BODIES, false), SelInput::new("Pièce 2 (reçoit la mortaise)", BODIES, false)];
    let (sc, inputs) = match id {
        "scout.meuble.create" => (Sc::new(Which::Meuble, 1), Vec::new()),
        "scout.coin.create" => (Sc::new(Which::Coin, 0), Vec::new()),
        "scout.piece.traverse" => (Sc::new(Which::Traverse, 0), Vec::new()),
        "scout.piece.equerre" => (Sc::new(Which::Equerre, 0), Vec::new()),
        "scout.piece.claies" => (Sc::new(Which::Claies, 0), Vec::new()),
        "scout.piece.filet" => (Sc::new(Which::Filet, 0), Vec::new()),
        _ => {
            if let Some(k) = id.strip_prefix("scout.assemblage.") {
                let kind = match k {
                    "mibois" => 0,
                    "tenon" => 1,
                    "aronde" => 5,
                    "cheville" => 8,
                    "brelage" => 9,
                    _ => return None,
                };
                (Sc::new(Which::Assemblage, kind), two())
            } else if let Some(k) = id.strip_prefix("scout.faconner.") {
                let kind = FACONNAGES.iter().position(|(n, _)| *n == k)?;
                (Sc::new(Which::Faconner, kind), vec![SelInput::new("Pièce à façonner", BODIES, false)])
            } else {
                return None;
            }
        }
    };
    Some((Kind::Scout(sc), inputs))
}

pub fn title(k: &Sc) -> &'static str {
    match k.which {
        Which::Meuble => "NOUVEAU MEUBLE",
        Which::Assemblage => "ASSEMBLAGE",
        Which::Faconner => "FAÇONNER",
        Which::Traverse => "TRAVERSE",
        Which::Equerre => "ÉQUERRE",
        Which::Claies => "PLATEAU DE CLAIES",
        Which::Filet => "FILET DE FICELLE",
        Which::Coin => "COIN DE PATROUILLE",
    }
}

pub fn command_id(k: &Sc) -> &'static str {
    match k.which {
        Which::Meuble => "scout.meuble.create",
        Which::Assemblage => "scout.assemblage",
        Which::Faconner => "scout.faconner",
        Which::Traverse => "scout.piece.traverse",
        Which::Equerre => "scout.piece.equerre",
        Which::Claies => "scout.piece.claies",
        Which::Filet => "scout.piece.filet",
        Which::Coin => "scout.coin.create",
    }
}

fn number(label: &str, text: &str) -> Result<f64, String> {
    text.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite()).ok_or_else(|| format!("{label} : un nombre est attendu"))
}

fn setting_row(ui: &mut egui::Ui, d: &ParamDef, val: &mut String) {
    if !d.options.is_empty() {
        let mut i = val.parse::<f64>().map_or(0, |v| v as usize).min(d.options.len() - 1);
        choice_row(ui, d.label, d.key, &mut i, d.options);
        *val = i.to_string();
    } else if d.check {
        let mut on = val != "0";
        crate::dialogs::form_row(ui, d.label, |ui| check(ui, &mut on));
        *val = if on { "1".into() } else { "0".into() };
    } else {
        text_row(ui, d.label, val, "");
    }
}

/// The dialog's rows.
pub fn rows(ui: &mut egui::Ui, k: &mut Sc) -> bool {
    match k.which {
        Which::Meuble => {
            let names: Vec<&str> = MODELS.iter().map(|m| m.name).collect();
            choice_row(ui, "Meuble", "sc_model", &mut k.kind, &names);
            if k.filled_for != Some(k.kind) {
                k.refill();
            }
            if let Some(m) = MODELS.get(k.kind) {
                ui.label(egui::RichText::new(m.summary).weak());
                text_row(ui, "Nom", &mut k.name, m.name);
                for (d, v) in m.params.iter().zip(k.vals.iter_mut()) {
                    setting_row(ui, d, v);
                }
            }
            text_row(ui, "Position X (mm)", &mut k.x, "");
            text_row(ui, "Position Y (mm)", &mut k.y, "");
            choice_row(ui, "Rotation", "sc_rot", &mut k.rotation, &ROTATIONS);
        }
        Which::Assemblage => {
            let labels: Vec<&str> = ASSEMBLAGES.iter().map(|(_, l)| *l).collect();
            choice_row(ui, "Assemblage", "sc_asm", &mut k.kind, &labels);
            let kind = ASSEMBLAGES.get(k.kind).map_or("", |(n, _)| *n);
            match kind {
                "mibois" => {
                    text_row(ui, "Part de la pièce 1 (%)", &mut k.pc, "50");
                }
                "cheville" => {
                    text_row(ui, "Diamètre de la cheville (mm)", &mut k.d, "20");
                }
                "brelage" => {
                    text_row(ui, "Nombre de tours", &mut k.turns, "6");
                }
                _ => {
                    text_row(ui, "Diamètre ou épaisseur du tenon (mm)", &mut k.d, "automatique");
                    crate::dialogs::form_row(ui, "Mortaise débouchante", |ui| check(ui, &mut k.through));
                }
            }
        }
        Which::Faconner => {
            let labels: Vec<&str> = FACONNAGES.iter().map(|(_, l)| *l).collect();
            choice_row(ui, "Façonnage", "sc_fac", &mut k.kind, &labels);
            match FACONNAGES.get(k.kind).map_or("", |(n, _)| *n) {
                "couper" => {
                    text_row(ui, "Nouvelle longueur (mm)", &mut k.length, "");
                }
                "pointe" => {
                    text_row(ui, "Longueur de la pointe (mm)", &mut k.length, "");
                    choice_row(ui, "Bout", "sc_end", &mut k.end, &["Début", "Fin"]);
                }
                _ => {}
            }
        }
        Which::Traverse => {
            text_row(ui, "Diamètre (mm)", &mut k.a, "");
            text_row(ui, "Longueur (mm)", &mut k.b, "");
            choice_row(ui, "Direction", "sc_dir", &mut k.dir, &["Longueur (X)", "Largeur (Y)", "Verticale (Z)"]);
            text_row(ui, "Position X (mm)", &mut k.x, "");
            text_row(ui, "Position Y (mm)", &mut k.y, "");
        }
        Which::Equerre => {
            text_row(ui, "Unité du 3-4-5 (mm)", &mut k.a, "");
            text_row(ui, "Diamètre des perches (mm)", &mut k.b, "");
        }
        Which::Claies => {
            text_row(ui, "Longueur (mm)", &mut k.a, "");
            text_row(ui, "Largeur (mm)", &mut k.b, "");
            text_row(ui, "Diamètre des claies (mm)", &mut k.c, "");
        }
        Which::Filet => {
            text_row(ui, "Longueur (mm)", &mut k.a, "");
            text_row(ui, "Largeur (mm)", &mut k.b, "");
            text_row(ui, "Maille (mm)", &mut k.c, "");
        }
        Which::Coin => {
            text_row(ui, "Nom du coin", &mut k.name, "Coin de patrouille");
            ui.label(egui::RichText::new("Une table, deux bancs, une étagère et une table de prépa : ajoutez ensuite d’autres meubles.").weak());
        }
    }
    false
}

fn body_name(inputs: &[SelInput], i: usize) -> Result<String, String> {
    match inputs.get(i).and_then(|inp| inp.items.first()) {
        Some(Sel::Body { name }) => Ok(name.clone()),
        Some(_) => Err("choisissez une pièce entière (cliquez sur le bois)".into()),
        None => Err(format!("choisissez la pièce {} dans la vue", i + 1)),
    }
}

/// The command OK runs.
pub fn commands(k: &Sc, inputs: &[SelInput]) -> Result<Vec<(String, Value)>, String> {
    let id = command_id(k).to_string();
    let p = match k.which {
        Which::Meuble => {
            let m = MODELS.get(k.kind).ok_or("choisissez un meuble")?;
            let rotation: u32 = [0, 90, 180, 270].get(k.rotation).copied().unwrap_or(0);
            let mut p = json!({"type": m.id, "at": [number("Position X", &k.x)?, number("Position Y", &k.y)?], "rotation": rotation});
            for (d, v) in m.params.iter().zip(&k.vals) {
                p[d.key] = json!(number(d.label, v)?);
            }
            if !k.name.trim().is_empty() {
                p["name"] = json!(k.name.trim());
            }
            p
        }
        Which::Assemblage => {
            let kind = ASSEMBLAGES.get(k.kind).map(|(n, _)| *n).ok_or("choisissez un assemblage")?;
            let mut p = json!({"kind": kind, "a": body_name(inputs, 0)?, "b": body_name(inputs, 1)?});
            if !k.d.trim().is_empty() {
                p["d"] = json!(number("Diamètre", &k.d)?);
            }
            match kind {
                "mibois" => p["pc"] = json!(number("Part", &k.pc)?),
                "brelage" => p["turns"] = json!(number("Tours", &k.turns)?),
                "cheville" => {}
                _ => p["through"] = json!(k.through),
            }
            p
        }
        Which::Faconner => {
            let kind = FACONNAGES.get(k.kind).map(|(n, _)| *n).ok_or("choisissez un façonnage")?;
            let mut p = json!({"kind": kind, "piece": body_name(inputs, 0)?});
            match kind {
                "couper" => p["length"] = json!(number("Longueur", &k.length)?),
                "pointe" => {
                    p["len"] = json!(number("Longueur de la pointe", &k.length)?);
                    p["end"] = json!(if k.end == 0 { "start" } else { "end" });
                }
                _ => {}
            }
            p
        }
        Which::Traverse => {
            let dir: [u8; 3] = [[1, 0, 0], [0, 1, 0], [0, 0, 1]].get(k.dir).copied().unwrap_or([1, 0, 0]);
            let d = number("Diamètre", &k.a)?;
            json!({"diameter": d, "length": number("Longueur", &k.b)?, "dir": dir, "at": [number("Position X", &k.x)?, number("Position Y", &k.y)?, d / 2.0]})
        }
        Which::Equerre => json!({"unit": number("Unité", &k.a)?, "d": number("Diamètre", &k.b)?}),
        Which::Claies => json!({"long": number("Longueur", &k.a)?, "larg": number("Largeur", &k.b)?, "dc": number("Diamètre", &k.c)?}),
        Which::Filet => json!({"long": number("Longueur", &k.a)?, "larg": number("Largeur", &k.b)?, "maille": number("Maille", &k.c)?}),
        Which::Coin => {
            if k.name.trim().is_empty() { json!({}) } else { json!({"name": k.name.trim()}) }
        }
    };
    Ok(vec![(id, p)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dialog_makes_a_command_the_engine_accepts() {
        for model in 0..MODELS.len() {
            let c = commands(&Sc::new(Which::Meuble, model), &[]).unwrap_or_default();
            assert_eq!(c.len(), 1);
            let mut s = solidcraft_engine::Session::default();
            assert!(s.execute(&c[0].0, &c[0].1).is_ok(), "{}", c[0].1);
        }
        for which in [Which::Traverse, Which::Equerre, Which::Claies, Which::Filet, Which::Coin] {
            let c = commands(&Sc::new(which, 0), &[]).unwrap_or_default();
            let mut s = solidcraft_engine::Session::default();
            assert!(s.execute(&c[0].0, &c[0].1).is_ok(), "{:?}: {}", which, c[0].1);
        }
    }

    #[test]
    fn an_assemblage_needs_its_two_poles() {
        let k = Sc::new(Which::Assemblage, 0);
        assert!(commands(&k, &[]).is_err());
        let inputs = [SelInput::new("a", BODIES, false), SelInput::new("b", BODIES, false)];
        assert!(commands(&k, &inputs).is_err());
    }

    #[test]
    fn a_typed_comma_and_a_bad_number() {
        let mut m = Sc::new(Which::Meuble, 1);
        m.vals[1] = "1500,5".into();
        assert!(commands(&m, &[]).is_ok());
        m.vals[1] = "long".into();
        assert!(commands(&m, &[]).is_err());
    }
}
