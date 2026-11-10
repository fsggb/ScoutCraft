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

/// The groups CathIA lists the assemblages in: (heading, first kind, number of kinds).
const GROUPES: &[(&str, usize, usize)] = &[("Tenons et mortaises", 1, 4), ("Mi-bois", 0, 1), ("Queues d’aronde", 5, 3), ("Chevilles et brêlage", 8, 2)];

/// What each assemblage blocks, in CathIA's words.
fn degres(kind: &str) -> &'static str {
    match kind {
        "mibois" => "Bloque 5 degrés de liberté : reste le décollement perpendiculaire aux pièces (ajoutez une cheville).",
        "hache" | "scie" => "Bloque 4 degrés de liberté : reste la rotation autour du tenon et le glissement hors de la mortaise.",
        "plat" | "platscie" => "Bloque 5 degrés de liberté : reste le glissement hors de la mortaise.",
        "aronde1" | "aronde2" | "aronde3" => "Bloque 5 degrés de liberté : reste le glissement latéral dans la rainure.",
        "cheville" => "Bloque la translation dans le plan perpendiculaire à la cheville ; reste la rotation autour d’elle.",
        _ => "Dernier recours : le brêlage tient les pièces mais ne les bloque pas comme un assemblage taillé.",
    }
}

const FACONNAGES: &[(&str, &str)] = &[("demi", "Fendre en deux"), ("quart", "Quart de bois"), ("couper", "Couper à la longueur"), ("pointe", "Tailler une pointe")];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Which {
    Meuble,
    Assemblage,
    Faconner,
    Traverse,
    Equerre,
    Plateau,
    Modifier,
    MeubleModif,
    Coin,
    Profil,
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
    /// Assemblage: the pieces swap their roles; mi-bois: add a peg.
    pub invert: bool,
    pub chev: bool,
    /// Meuble: cut the hidden joints now (slow). Assemblage: wolf's-mouth ends (slow).
    pub detail: bool,
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
            Which::Plateau => ("1000".into(), "600".into(), "20".into()),
            Which::Modifier => (String::new(), String::new(), String::new()),
            Which::Profil => ("40".into(), "3".into(), "1000".into()),
            _ => (String::new(), String::new(), String::new()),
        };
        let mut s = Sc {
            which,
            kind,
            filled_for: None,
            name: String::new(),
            vals: Vec::new(),
            x: if which == Which::Modifier { String::new() } else { "0".into() },
            y: if which == Which::Modifier { String::new() } else { "0".into() },
            rotation: 0,
            through: false,
            invert: false,
            chev: false,
            detail: false,
            pc: if which == Which::Plateau { "0".into() } else { "50".into() },
            d: if which == Which::Plateau { "30".into() } else { String::new() },
            turns: "6".into(),
            length: if which == Which::Plateau { "0".into() } else { "500".into() },
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
        if !matches!(self.which, Which::Meuble | Which::MeubleModif) {
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
        "scout.plateau" => (Sc::new(Which::Plateau, 0), Vec::new()),
        "scout.piece.modifier" => (Sc::new(Which::Modifier, 0), vec![SelInput::new("Pièce à modifier", BODIES, false)]),
        "scout.meuble.modifier" => {
            // the last furniture made, with the settings it was made with
            let c = _app.session.doc.components.iter().rev().find(|c| c.properties.contains_key("scout_meuble"))?;
            let kind = MODELS.iter().position(|m| Some(m.id) == c.properties.get("scout_meuble").map(String::as_str))?;
            let mut sc = Sc::new(Which::MeubleModif, kind);
            sc.refill();
            let have: serde_json::Map<String, Value> = c.properties.get("scout_reglages").and_then(|t| serde_json::from_str(t).ok()).unwrap_or_default();
            for (d, v) in MODELS[kind].params.iter().zip(sc.vals.iter_mut()) {
                if let Some(x) = have.get(d.key).and_then(Value::as_f64) {
                    *v = format!("{x}");
                }
            }
            sc.name = c.name.clone();
            (sc, Vec::new())
        }
        "scout.profil.tube_rond" => (Sc::new(Which::Profil, 0), Vec::new()),
        "scout.profil.tube_carre" => (Sc::new(Which::Profil, 1), Vec::new()),
        "scout.profil.corniere" => (Sc::new(Which::Profil, 2), Vec::new()),
        "scout.profil.u" => (Sc::new(Which::Profil, 3), Vec::new()),
        "scout.profil.i" => (Sc::new(Which::Profil, 4), Vec::new()),
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
        Which::Plateau => "PLATEAU",
        Which::Modifier => "MODIFIER UNE PIÈCE",
        Which::MeubleModif => "MODIFIER LE MEUBLE",
        Which::Coin => "COIN DE PATROUILLE",
        Which::Profil => "PROFILÉ MÉTAL",
    }
}

pub fn command_id(k: &Sc) -> &'static str {
    match k.which {
        Which::Meuble => "scout.meuble.create",
        Which::Assemblage => "scout.assemblage",
        Which::Faconner => "scout.faconner",
        Which::Traverse => "scout.piece.traverse",
        Which::Equerre => "scout.piece.equerre",
        Which::Plateau => "scout.plateau",
        Which::Modifier => "scout.piece.modifier",
        Which::MeubleModif => "scout.meuble.modifier",
        Which::Coin => "scout.coin.create",
        Which::Profil => "scout.profil",
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
            crate::dialogs::form_row(ui, "Tailler tous les assemblages (plus lent)", |ui| check(ui, &mut k.detail));
        }
        Which::MeubleModif => {
            ui.label(egui::RichText::new(format!("Modifier « {} » : le meuble est refait à la même place, et les assemblages que vous y avez ajoutés sont refaits.", k.name)).weak());
            if let Some(m) = MODELS.get(k.kind) {
                for (d, v) in m.params.iter().zip(k.vals.iter_mut()) {
                    setting_row(ui, d, v);
                }
            }
        }
        Which::Modifier => {
            ui.label(egui::RichText::new("Laissez vide ce qui ne change pas. Les assemblages qui dépendent de cette pièce sont refaits.").weak());
            text_row(ui, "Nouveau diamètre (mm)", &mut k.a, "");
            text_row(ui, "Nouvelle longueur (mm)", &mut k.b, "");
            text_row(ui, "Déplacer en X (mm)", &mut k.x, "");
            text_row(ui, "Déplacer en Y (mm)", &mut k.y, "");
            text_row(ui, "Déplacer en Z (mm)", &mut k.c, "");
        }
        Which::Assemblage => {
            ui.label(egui::RichText::new("Type d’assemblage").strong());
            for (titre, first, n) in GROUPES {
                ui.label(egui::RichText::new(*titre).weak());
                for i in *first..*first + *n {
                    if let Some((_, label)) = ASSEMBLAGES.get(i) {
                        ui.selectable_value(&mut k.kind, i, *label);
                    }
                }
            }
            let kind = ASSEMBLAGES.get(k.kind).map_or("", |(n, _)| *n);
            ui.add_space(4.0);
            ui.label(egui::RichText::new(degres(kind)).italics());
            ui.add_space(4.0);
            match kind {
                "mibois" => {
                    text_row(ui, "Part de la pièce 1 (%)", &mut k.pc, "50");
                    crate::dialogs::form_row(ui, "Ajouter une cheville", |ui| check(ui, &mut k.chev));
                    if k.chev {
                        text_row(ui, "Diamètre de la cheville (mm)", &mut k.d, "20");
                    }
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
                    crate::dialogs::form_row(ui, "Bout en gueule de loup (plus lent)", |ui| check(ui, &mut k.detail));
                }
            }
            if !matches!(kind, "mibois" | "cheville" | "brelage") {
                crate::dialogs::form_row(ui, "⇄ Inverser (la pièce 2 porte le tenon)", |ui| check(ui, &mut k.invert));
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
        Which::Plateau => {
            choice_row(ui, "Forme du plateau", "sc_pl", &mut k.kind, &["Claies tressées (sous, sur, sous)", "Claies droites", "Filet de ficelle", "Quarts de bois jointifs"]);
            text_row(ui, "Longueur (mm)", &mut k.a, "");
            text_row(ui, "Largeur entre les lisses (mm)", &mut k.b, "");
            if k.kind == 2 {
                text_row(ui, "Maille du filet (mm)", &mut k.d, "");
            } else {
                text_row(ui, "Diamètre des claies (mm)", &mut k.c, "");
            }
            text_row(ui, "Début du clayage (mm)", &mut k.pc, "");
            text_row(ui, "Longueur clayée (mm, 0 = tout)", &mut k.length, "");
        }
        Which::Profil => {
            let kinds = ["Tube rond", "Tube carré", "Cornière", "Profilé en U", "Poutre en I"];
            choice_row(ui, "Profilé", "sc_prof", &mut k.kind, &kinds);
            text_row(ui, "Dimension extérieure (mm)", &mut k.a, "");
            text_row(ui, "Épaisseur de la paroi (mm)", &mut k.b, "");
            text_row(ui, "Longueur (mm)", &mut k.c, "");
            choice_row(ui, "Direction", "sc_dir2", &mut k.dir, &["Longueur (X)", "Largeur (Y)", "Verticale (Z)"]);
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
            p["detail"] = json!(k.detail);
            p
        }
        Which::MeubleModif => {
            let m = MODELS.get(k.kind).ok_or("choisissez un meuble")?;
            let mut p = json!({"meuble": k.name});
            for (d, v) in m.params.iter().zip(&k.vals) {
                p[d.key] = json!(number(d.label, v)?);
            }
            p
        }
        Which::Modifier => {
            let mut p = json!({"piece": body_name(inputs, 0)?});
            for (key, label, text) in [("diameter", "Diamètre", &k.a), ("length", "Longueur", &k.b), ("dx", "X", &k.x), ("dy", "Y", &k.y), ("dz", "Z", &k.c)] {
                if !text.trim().is_empty() {
                    p[key] = json!(number(label, text)?);
                }
            }
            p
        }
        Which::Assemblage => {
            let kind = ASSEMBLAGES.get(k.kind).map(|(n, _)| *n).ok_or("choisissez un assemblage")?;
            let (first, second) = (body_name(inputs, 0)?, body_name(inputs, 1)?);
            let tenon = !matches!(kind, "mibois" | "cheville" | "brelage");
            let (a, b) = if tenon && k.invert { (second, first) } else { (first, second) };
            let mut p = json!({"kind": kind, "a": a, "b": b});
            if !k.d.trim().is_empty() {
                p["d"] = json!(number("Diamètre", &k.d)?);
            }
            match kind {
                "mibois" => {
                    p["pc"] = json!(number("Part", &k.pc)?);
                    p["chev"] = json!(k.chev);
                }
                "brelage" => p["turns"] = json!(number("Tours", &k.turns)?),
                "cheville" => {}
                _ => {
                    p["through"] = json!(k.through);
                    p["gueule"] = json!(k.detail);
                }
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
        Which::Plateau => {
            let mode = ["tresse", "droit", "filet", "quarts"].get(k.kind).copied().unwrap_or("tresse");
            let maille = if k.d.trim().is_empty() { 30.0 } else { number("Maille", &k.d)? };
            json!({"mode": mode, "long": number("Longueur", &k.a)?, "larg": number("Largeur", &k.b)?, "dc": number("Diamètre", &k.c)?, "maille": maille, "c0": number("Début du clayage", &k.pc)?, "cl": number("Longueur clayée", &k.length)?})
        }
        Which::Profil => {
            let kinds = ["tube_rond", "tube_carre", "corniere", "u", "i"];
            let dir = ["X", "Y", "Z"].get(k.dir).copied().unwrap_or("X");
            json!({"kind": kinds.get(k.kind).copied().unwrap_or("tube_carre"), "size": number("Dimension", &k.a)?, "epaisseur": number("Épaisseur", &k.b)?, "length": number("Longueur", &k.c)?, "dir": dir})
        }
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
        for which in [Which::Traverse, Which::Equerre, Which::Plateau, Which::Coin, Which::Profil] {
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
