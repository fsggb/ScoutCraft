//! The two workshops of CathIA that are windows: **Dessin technique** (the sheet, painted from the
//! engine's primitives, with its views, scale, dimensions and title block; exported as SVG or in
//! the HTML page) and **Nomenclature** (the pieces, the assemblages and the materials list, with a
//! CSV). Both read the engine (`scout.atelier.dessin`, `scout.atelier.nomenclature`) and are
//! drawn again only when the design or a control changes.

use egui::{Color32, FontId, Pos2, Rect, Shape, Stroke, vec2};
use serde_json::{Value, json};
use solidcraft_engine::scout::dessin::{Anchor, ECHELLES, Kind, Prim, View};

use crate::SolveApp;

/// What the windows remember.
#[derive(Default)]
pub struct State {
    pub dessin: Option<Dessin>,
    pub nomen: Option<Nomen>,
}

pub struct Dessin {
    /// 0: the whole design's only furniture (or corner); else the subject by name.
    subject: usize,
    format: usize,
    echelle: usize,
    vues: [bool; 7],
    cotes: bool,
    cotes_pieces: bool,
    tableau: bool,
    titre: String,
    societe: String,
    dessinateur: String,
    date: String,
    materiau: String,
    reference: String,
    indice: String,
    zoom: f32,
    cache: Option<(u64, String, Result<Value, String>)>,
    message: Option<String>,
}

impl Dessin {
    fn new() -> Self {
        Dessin {
            subject: 0,
            format: 0,
            echelle: 0,
            vues: [true, false, true, false, true, false, true],
            cotes: true,
            cotes_pieces: false,
            tableau: true,
            titre: String::new(),
            societe: String::new(),
            dessinateur: String::new(),
            date: String::new(),
            materiau: "Bois rond".into(),
            reference: String::new(),
            indice: "A".into(),
            zoom: 1.0,
            cache: None,
            message: None,
        }
    }
}

pub struct Nomen {
    subject: usize,
    cache: Option<(u64, usize, Result<Value, String>)>,
    message: Option<String>,
}

/// The subjects a design offers: (label, params key, name), the whole design last-but-not-least.
fn subjects(app: &SolveApp) -> Vec<(String, &'static str, String)> {
    let mut v = Vec::new();
    for c in &app.session.doc.components {
        if c.properties.contains_key("scout_coin") {
            v.push((format!("Coin : {}", c.name), "coin", c.name.clone()));
        }
    }
    for c in &app.session.doc.components {
        if c.properties.contains_key("scout_meuble") {
            v.push((format!("Meuble : {}", c.name), "meuble", c.name.clone()));
        }
    }
    v
}

pub fn open_dessin(app: &mut SolveApp) {
    if app.scout_win.dessin.is_none() {
        let mut d = Dessin::new();
        d.dessinateur = app.session.doc.metadata.get("author").and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
        app.scout_win.dessin = Some(d);
    }
}

pub fn open_nomenclature(app: &mut SolveApp) {
    if app.scout_win.nomen.is_none() {
        app.scout_win.nomen = Some(Nomen { subject: 0, cache: None, message: None });
    }
}

fn colour(k: Kind) -> (Color32, Color32) {
    // (fill, stroke)
    match k {
        Kind::Pole => (Color32::from_rgb(233, 211, 168), Color32::from_rgb(74, 47, 18)),
        Kind::PoleCut => (Color32::from_rgb(201, 164, 106), Color32::from_rgb(74, 47, 18)),
        Kind::Floor => (Color32::from_rgb(233, 211, 168), Color32::from_rgb(74, 47, 18)),
        Kind::Dim => (Color32::TRANSPARENT, Color32::from_rgb(194, 106, 0)),
        Kind::Text => (Color32::from_rgb(29, 35, 48), Color32::from_rgb(29, 35, 48)),
        Kind::Title => (Color32::from_rgb(0, 7, 97), Color32::from_rgb(0, 7, 97)),
        Kind::Frame | Kind::Rule => (Color32::TRANSPARENT, Color32::from_rgb(29, 35, 48)),
        Kind::Guide => (Color32::TRANSPARENT, Color32::from_rgb(111, 135, 156)),
    }
}

/// Paints the primitives on a sheet drawn into `rect` (sheet millimetres → points).
fn paint_sheet(painter: &egui::Painter, rect: Rect, size: (f64, f64), prims: &[Prim]) {
    let k = rect.width() / size.0 as f32;
    let at = |p: [f64; 2]| Pos2::new(rect.left() + p[0] as f32 * k, rect.top() + p[1] as f32 * k);
    painter.rect_filled(rect, 0.0, Color32::WHITE);
    for p in prims {
        match p {
            Prim::Line { a, b, w, k: kind } => {
                painter.line_segment([at(*a), at(*b)], Stroke::new(((*w as f32) * k).max(0.6), colour(*kind).1));
            }
            Prim::Poly { pts, k: kind } => {
                let (fill, stroke) = colour(*kind);
                let pts: Vec<Pos2> = pts.iter().map(|q| at(*q)).collect();
                painter.add(Shape::convex_polygon(pts, fill, Stroke::new(0.9, stroke)));
            }
            Prim::Circle { c, r, k: kind } => {
                let (fill, stroke) = colour(*kind);
                painter.circle(at(*c), (*r as f32 * k).max(1.0), fill, Stroke::new(0.9, stroke));
            }
            Prim::Text { at: pos, s, h, anchor, rot, k: kind } => {
                let align = match anchor {
                    Anchor::Start => egui::Align2::LEFT_BOTTOM,
                    Anchor::Middle => egui::Align2::CENTER_BOTTOM,
                    Anchor::End => egui::Align2::RIGHT_BOTTOM,
                };
                let size = (*h as f32 * k * 1.15).max(5.0);
                let colour = colour(*kind).1;
                if *rot {
                    let galley = painter.layout_no_wrap(s.clone(), FontId::proportional(size), colour);
                    let w = galley.size().x;
                    let origin = at(*pos) + vec2(-size * 0.4, w / 2.0);
                    painter.add(Shape::Text(egui::epaint::TextShape::new(origin, galley, colour).with_angle(-std::f32::consts::FRAC_PI_2)));
                } else {
                    painter.text(at(*pos), align, s, FontId::proportional(size), colour);
                }
            }
        }
    }
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::from_gray(160)), egui::StrokeKind::Outside);
}

fn pick_name(subject: usize, list: &[(String, &'static str, String)]) -> Option<(&'static str, String)> {
    subject.checked_sub(1).and_then(|i| list.get(i)).map(|(_, k, n)| (*k, n.clone()))
}

fn dessin_params(d: &Dessin, list: &[(String, &'static str, String)]) -> Value {
    let vues: Vec<&str> = View::ALL.iter().zip(d.vues.iter()).filter(|(_, on)| **on).map(|(v, _)| v.id()).collect();
    let mut p = json!({
        "format": if d.format == 0 { "A3" } else { "A4" },
        "echelle": if d.echelle == 0 { 0.0 } else { ECHELLES.get(d.echelle - 1).copied().unwrap_or(0.0) },
        "vues": if vues.is_empty() { vec!["face"] } else { vues },
        "cotes": d.cotes, "cotes_pieces": d.cotes_pieces, "tableau": d.tableau,
        "societe": d.societe, "dessinateur": d.dessinateur, "date": d.date, "materiau": d.materiau, "reference": d.reference, "indice": d.indice,
    });
    if !d.titre.trim().is_empty() {
        p["titre"] = json!(d.titre);
    }
    if let Some((k, n)) = pick_name(d.subject, list) {
        p[k] = json!(n);
    }
    p
}

fn text_field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::TextEdit::singleline(value).desired_width(150.0));
    });
}

/// The Dessin technique window.
pub fn show_dessin(app: &mut SolveApp, ctx: &egui::Context) {
    let Some(mut d) = app.scout_win.dessin.take() else { return };
    let list = subjects(app);
    if d.subject > list.len() {
        d.subject = 0;
    }
    let mut open = true;
    let mut save: Option<(&'static str, &'static str, &'static str, Value)> = None;
    egui::Window::new("Dessin technique").id(egui::Id::new("sc_dessin")).open(&mut open).default_size([980.0, 700.0]).resizable(true).show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            let mut names = vec!["Automatique".to_string()];
            names.extend(list.iter().map(|(l, _, _)| l.clone()));
            let current = names.get(d.subject).cloned().unwrap_or_default();
            egui::ComboBox::from_label("Sujet").selected_text(current).show_ui(ui, |ui| {
                for (i, n) in names.iter().enumerate() {
                    ui.selectable_value(&mut d.subject, i, n);
                }
            });
            egui::ComboBox::from_label("Format").selected_text(["A3", "A4"][d.format.min(1)]).show_ui(ui, |ui| {
                ui.selectable_value(&mut d.format, 0, "A3 (420 × 297)");
                ui.selectable_value(&mut d.format, 1, "A4 (297 × 210)");
            });
            let label = if d.echelle == 0 { "Automatique".to_string() } else { format!("1:{}", ECHELLES.get(d.echelle - 1).copied().unwrap_or(1.0)) };
            egui::ComboBox::from_label("Échelle").selected_text(label).show_ui(ui, |ui| {
                ui.selectable_value(&mut d.echelle, 0, "Automatique");
                for (i, e) in ECHELLES.iter().enumerate() {
                    ui.selectable_value(&mut d.echelle, i + 1, format!("1:{e}"));
                }
            });
            ui.add(egui::Slider::new(&mut d.zoom, 0.5..=3.0).text("Zoom"));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Vues :");
            for (v, on) in View::ALL.iter().zip(d.vues.iter_mut()) {
                ui.checkbox(on, v.label().trim_start_matches("Vue de ").trim_start_matches("Vue "));
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut d.cotes, "Cotes d'ensemble");
            ui.checkbox(&mut d.cotes_pieces, "Diamètre des pièces");
            ui.checkbox(&mut d.tableau, "Nomenclature sur la feuille");
        });
        egui::CollapsingHeader::new("Cartouche").default_open(false).show(ui, |ui| {
            text_field(ui, "Titre", &mut d.titre);
            text_field(ui, "Société", &mut d.societe);
            text_field(ui, "Dessinateur", &mut d.dessinateur);
            text_field(ui, "Date", &mut d.date);
            text_field(ui, "Matériau", &mut d.materiau);
            text_field(ui, "Référence", &mut d.reference);
            text_field(ui, "Indice", &mut d.indice);
        });
        let params = dessin_params(&d, &list);
        let key = params.to_string();
        let revision = app.session.revision;
        if !matches!(&d.cache, Some((r, k, _)) if *r == revision && *k == key) {
            let res = app.session.execute("scout.atelier.dessin", &params).map_err(|e| e.to_string());
            d.cache = Some((revision, key, res));
        }
        ui.horizontal(|ui| {
            if ui.button("Exporter la feuille (SVG)…").clicked() {
                save = Some(("plan.svg", "svg", "scout.atelier.dessin", params.clone()));
            }
            if ui.button("Exporter toutes les pages (HTML)…").clicked() {
                save = Some(("plans.html", "html", "scout.export.html", json!({"title": if d.titre.is_empty() { "Plans du camp" } else { d.titre.as_str() }, "author": d.dessinateur})));
            }
            if let Some(m) = &d.message {
                ui.label(egui::RichText::new(m).weak());
            }
        });
        ui.separator();
        match d.cache.as_ref().map(|c| &c.2) {
            Some(Ok(v)) => {
                let size = (v["sheet"]["w"].as_f64().unwrap_or(420.0), v["sheet"]["h"].as_f64().unwrap_or(297.0));
                if let Some(e) = v["sheet"]["echelle"].as_f64() {
                    ui.label(egui::RichText::new(format!("Échelle 1:{e}")).weak());
                }
                let prims: Vec<Prim> = v["prims"].as_array().map(|a| a.iter().filter_map(|x| serde_json::from_value(x.clone()).ok()).collect()).unwrap_or_default();
                egui::ScrollArea::both().show(ui, |ui| {
                    let w = (ui.available_width() - 12.0).max(300.0) * d.zoom;
                    let (rect, _) = ui.allocate_exact_size(vec2(w, w * (size.1 / size.0) as f32), egui::Sense::hover());
                    paint_sheet(ui.painter(), rect, size, &prims);
                });
            }
            Some(Err(e)) => {
                ui.label(egui::RichText::new(e).color(Color32::from_rgb(200, 60, 60)));
                ui.label("Créez un meuble (onglet Créer) pour dessiner son plan.");
            }
            None => {}
        }
    });
    if let Some((name, ext, command, params)) = save {
        app.pick_save_command(name, &[ext], command, params);
        d.message = Some("Choisissez où enregistrer le fichier.".into());
    }
    if open {
        app.scout_win.dessin = Some(d);
    }
}

/// The Nomenclature window.
pub fn show_nomenclature(app: &mut SolveApp, ctx: &egui::Context) {
    let Some(mut n) = app.scout_win.nomen.take() else { return };
    let list = subjects(app);
    if n.subject > list.len() {
        n.subject = 0;
    }
    let mut open = true;
    let mut save = false;
    let mut copy: Option<String> = None;
    egui::Window::new("Nomenclature").id(egui::Id::new("sc_nomenclature")).open(&mut open).default_size([760.0, 560.0]).resizable(true).show(ctx, |ui| {
        let mut names = vec!["Tout le projet".to_string()];
        names.extend(list.iter().map(|(l, _, _)| l.clone()));
        egui::ComboBox::from_label("Sujet").selected_text(names.get(n.subject).cloned().unwrap_or_default()).show_ui(ui, |ui| {
            for (i, name) in names.iter().enumerate() {
                ui.selectable_value(&mut n.subject, i, name);
            }
        });
        let revision = app.session.revision;
        if !matches!(&n.cache, Some((r, s, _)) if *r == revision && *s == n.subject) {
            let params = match pick_name(n.subject, &list) {
                Some((k, name)) => json!({ k: name }),
                None => json!({}),
            };
            let res = app.session.execute("scout.atelier.nomenclature", &params).map_err(|e| e.to_string());
            n.cache = Some((revision, n.subject, res));
        }
        ui.horizontal(|ui| {
            if ui.button("Exporter (CSV)…").clicked() {
                save = true;
            }
            if ui.button("Copier").clicked()
                && let Some(Ok(v)) = n.cache.as_ref().map(|c| &c.2)
            {
                copy = v["csv"].as_str().map(str::to_string);
            }
            if let Some(m) = &n.message {
                ui.label(egui::RichText::new(m).weak());
            }
        });
        ui.separator();
        match n.cache.as_ref().map(|c| &c.2) {
            Some(Ok(v)) => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Pièces");
                    egui::Grid::new("sc_nom_pieces").striped(true).show(ui, |ui| {
                        for h in ["N°", "Qté", "Désignation", "Dimensions", "Assemblages", "Essence"] {
                            ui.strong(h);
                        }
                        ui.end_row();
                        for r in v["pieces"].as_array().into_iter().flatten() {
                            ui.label(r["n"].to_string());
                            ui.label(r["qte"].to_string());
                            for k in ["designation", "dimensions", "assemblages", "essence"] {
                                ui.label(r[k].as_str().unwrap_or(""));
                            }
                            ui.end_row();
                        }
                    });
                    ui.add_space(8.0);
                    ui.heading("Assemblages");
                    egui::Grid::new("sc_nom_asm").striped(true).show(ui, |ui| {
                        for a in v["assemblages"].as_array().into_iter().flatten() {
                            ui.label(a[0].as_str().unwrap_or(""));
                            ui.label(a[1].to_string());
                            ui.end_row();
                        }
                    });
                    ui.add_space(8.0);
                    ui.heading("Liste de matériel");
                    egui::Grid::new("sc_nom_mat").striped(true).show(ui, |ui| {
                        for h in ["Quantité", "Désignation", "Dimensions", "Opérations", "Essence"] {
                            ui.strong(h);
                        }
                        ui.end_row();
                        for r in v["materiel"].as_array().into_iter().flatten() {
                            for k in ["quantite", "designation", "dimensions", "operations", "essence"] {
                                ui.label(r[k].as_str().unwrap_or(""));
                            }
                            ui.end_row();
                        }
                    });
                });
            }
            Some(Err(e)) => {
                ui.label(egui::RichText::new(e).color(Color32::from_rgb(200, 60, 60)));
                ui.label("Créez un meuble (onglet Créer) pour établir sa nomenclature.");
            }
            None => {}
        }
    });
    if let Some(text) = copy {
        ctx.copy_text(text);
        n.message = Some("Copié.".into());
    }
    if save {
        let params = match pick_name(n.subject, &list) {
            Some((k, name)) => json!({ k: name }),
            None => json!({}),
        };
        app.pick_save_command("nomenclature.csv", &["csv"], "scout.atelier.nomenclature", params);
        n.message = Some("Choisissez où enregistrer le fichier.".into());
    }
    if open {
        app.scout_win.nomen = Some(n);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_asks_for_what_its_controls_say() {
        let mut d = Dessin::new();
        d.format = 1;
        d.echelle = 3;
        d.vues = [false; 7];
        let p = dessin_params(&d, &[]);
        assert_eq!(p["format"], "A4");
        assert_eq!(p["echelle"], 5.0);
        assert_eq!(p["vues"], json!(["face"]));
        let list = vec![("Meuble : Banc".to_string(), "meuble", "Banc".to_string())];
        d.subject = 1;
        assert_eq!(dessin_params(&d, &list)["meuble"], "Banc");
        assert!(pick_name(0, &list).is_none() && pick_name(9, &list).is_none());
    }

    #[test]
    fn a_sheet_made_by_the_engine_is_read_back() {
        let mut s = solidcraft_engine::Session::default();
        assert!(s.execute("scout.meuble.create", &json!({"type": "banc"})).is_ok());
        let v = s.execute("scout.atelier.dessin", &dessin_params(&Dessin::new(), &[])).unwrap_or_default();
        let prims: Vec<Prim> = v["prims"].as_array().map(|a| a.iter().filter_map(|x| serde_json::from_value(x.clone()).ok()).collect()).unwrap_or_default();
        assert!(prims.len() > 40, "{}", prims.len());
        assert_eq!(v["prims"].as_array().map(Vec::len), Some(prims.len()));
    }
}
