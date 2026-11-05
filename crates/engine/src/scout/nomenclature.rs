//! Nomenclature: CathIA's three tables, from the poles. The pieces (number, quantity, designation,
//! dimensions, joints, wood), the assemblages (how many of each) and the materials list (metres of
//! round wood, number of joints, rope, staples), with a CSV the sheets and the application share.

use serde::Serialize;

use super::pole::{Op, Pole, SplitMode, cut_list, joint_summary};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PieceRow {
    pub n: usize,
    pub qte: usize,
    pub designation: String,
    pub dimensions: String,
    pub assemblages: String,
    pub essence: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MatRow {
    pub quantite: String,
    pub designation: String,
    pub dimensions: String,
    pub operations: String,
    pub essence: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Nomenclature {
    pub pieces: Vec<PieceRow>,
    pub assemblages: Vec<(String, usize)>,
    pub materiel: Vec<MatRow>,
}

fn mm(v: f64) -> String {
    if (v - v.round()).abs() < 0.05 { format!("{}", v.round() as i64) } else { format!("{v:.1}") }
}

/// What a pole undergoes, in words.
fn operations(p: &Pole) -> String {
    let mut v: Vec<String> = Vec::new();
    for o in &p.ops {
        v.push(match o {
            Op::Tenon { style, d, w, .. } => if style.round() { format!("{} Ø{}", style.label(), mm(*d)) } else { format!("{} {}×{}", style.label(), mm(*w), mm(*d)) },
            Op::Mortise { d, .. } => format!("mortaise Ø{}", mm(*d)),
            Op::Notch { .. } => "mi-bois".to_string(),
            Op::Hole { d, .. } => format!("trou Ø{}", mm(*d)),
            Op::Split { mode } => if *mode == SplitMode::Demi { "fendu en deux".into() } else { "quart de bois".into() },
            Op::CutOff { .. } => "coupé".to_string(),
            Op::Point { len, .. } => format!("pointe {}", mm(*len)),
        });
    }
    v.sort();
    v.dedup();
    v.join(", ")
}

/// The three tables of a set of poles.
pub fn of(poles: &[Pole]) -> Nomenclature {
    let rows = cut_list(poles);
    let pieces: Vec<PieceRow> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let sample = poles.iter().find(|p| p.role == r.role && p.diameter == r.diameter && p.width == r.width && ((p.length * 10.0).round() / 10.0 - r.length).abs() < 0.05);
            PieceRow {
                n: i + 1,
                qte: r.count,
                designation: r.role.clone(),
                dimensions: if r.width > 0.0 { format!("{} × {} × {} mm", mm(r.diameter), mm(r.width), mm(r.length)) } else { format!("Ø{} × {} mm", mm(r.diameter), mm(r.length)) },
                assemblages: sample.map(operations).unwrap_or_default(),
                essence: if r.role == "ficelle" || r.role == "brêlage" { "ficelle".into() } else { "bois rond".into() },
            }
        })
        .collect();
    let assemblages = joint_summary(poles);
    let wood: Vec<&Pole> = poles.iter().filter(|p| p.role != "ficelle" && p.role != "brêlage").collect();
    let metres: f64 = wood.iter().map(|p| p.cut_length()).sum::<f64>() / 1000.0;
    let joints: usize = assemblages.iter().filter(|(k, _)| !k.starts_with("rondin fendu") && !k.starts_with("quart") && k != "pointe").map(|(_, c)| *c).sum();
    // rope: every turn of a lashing goes round the pole; the net is its ropes' length
    let rope_m: f64 = poles
        .iter()
        .map(|p| match p.role.as_str() {
            "brêlage" => (p.length / 6.0).max(1.0) * std::f64::consts::PI * p.diameter / 1000.0,
            "ficelle" => p.length / 1000.0,
            _ => 0.0,
        })
        .sum();
    let staples = 2 * poles.iter().filter(|p| p.role == "claie").count();
    let mut materiel = Vec::new();
    let mut by_section: Vec<(String, f64, usize)> = Vec::new();
    for p in &wood {
        let key = if p.width > 0.0 { format!("{} × {} mm", mm(p.diameter), mm(p.width)) } else { format!("Ø{} mm", mm(p.diameter)) };
        match by_section.iter_mut().find(|(k, _, _)| *k == key) {
            Some((_, m, c)) => {
                *m += p.cut_length() / 1000.0;
                *c += 1;
            }
            None => by_section.push((key, p.cut_length() / 1000.0, 1)),
        }
    }
    for (k, m, c) in by_section {
        materiel.push(MatRow { quantite: format!("{m:.2} m"), designation: "Bois rond".into(), dimensions: k, operations: format!("{c} pièces"), essence: "bois rond".into() });
    }
    materiel.push(MatRow { quantite: format!("{metres:.2} m"), designation: "Total bois".into(), dimensions: String::new(), operations: String::new(), essence: "bois rond".into() });
    materiel.push(MatRow { quantite: joints.to_string(), designation: "Assemblages".into(), dimensions: String::new(), operations: String::new(), essence: String::new() });
    if rope_m > 0.0 {
        materiel.push(MatRow { quantite: format!("{rope_m:.1} m"), designation: "Ficelle ou corde".into(), dimensions: String::new(), operations: "brêlages et filets".into(), essence: "ficelle".into() });
    }
    if staples > 0 {
        materiel.push(MatRow { quantite: staples.to_string(), designation: "Clous cavaliers".into(), dimensions: String::new(), operations: "fixation des claies".into(), essence: String::new() });
    }
    Nomenclature { pieces, assemblages, materiel }
}

fn cell(s: &str) -> String {
    let t = s.replace('"', "\"\"");
    if t.contains([';', '"', '\n']) { format!("\"{t}\"") } else { t }
}

/// The three tables as one CSV text (`;` separated, as French spreadsheets read it).
pub fn csv(n: &Nomenclature) -> String {
    let mut o = String::from("\u{feff}NOMENCLATURE DES PIÈCES\nN°;Qté;Désignation;Dimensions;Assemblages;Essence\n");
    for r in &n.pieces {
        o.push_str(&format!("{};{};{};{};{};{}\n", r.n, r.qte, cell(&r.designation), cell(&r.dimensions), cell(&r.assemblages), cell(&r.essence)));
    }
    o.push_str("\nNOMENCLATURE DES ASSEMBLAGES\nAssemblage;Nombre\n");
    for (k, c) in &n.assemblages {
        o.push_str(&format!("{};{}\n", cell(k), c));
    }
    o.push_str("\nLISTE DE MATÉRIEL\nQuantité;Désignation;Dimensions;Opérations;Essence\n");
    for r in &n.materiel {
        o.push_str(&format!("{};{};{};{};{}\n", cell(&r.quantite), cell(&r.designation), cell(&r.dimensions), cell(&r.operations), cell(&r.essence)));
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scout::models::{Params, build, model};

    fn poles_of(id: &str) -> Vec<Pole> {
        model(id).map(|m| build(id, &Params::of(m)).unwrap_or_default()).unwrap_or_default()
    }

    #[test]
    fn the_three_tables_are_filled() {
        let n = of(&poles_of("table"));
        assert!(!n.pieces.is_empty() && !n.assemblages.is_empty());
        assert!(n.materiel.iter().any(|r| r.designation == "Total bois"));
        assert_eq!(n.pieces.iter().map(|r| r.qte).sum::<usize>(), poles_of("table").len());
        assert!(n.pieces.iter().any(|r| r.assemblages.contains("mi-bois")));
    }

    #[test]
    fn the_csv_quotes_and_lists_every_table() {
        let text = csv(&of(&poles_of("prepa")));
        for h in ["NOMENCLATURE DES PIÈCES", "NOMENCLATURE DES ASSEMBLAGES", "LISTE DE MATÉRIEL", "Clous cavaliers"] {
            assert!(text.contains(h), "{h}");
        }
        assert_eq!(cell("a;b"), "\"a;b\"");
        assert_eq!(cell("c\"d"), "\"c\"\"d\"");
    }

    #[test]
    fn a_net_counts_its_rope() {
        let n = of(&poles_of("vaisselier"));
        assert!(n.materiel.iter().any(|r| r.designation.starts_with("Ficelle")));
    }
}
