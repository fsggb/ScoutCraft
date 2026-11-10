//! The eleven furniture of CathIA, rebuilt from poles with real joints: tenons and mortises, half-laps
//! and pegs (see `pole`). Each model has its parameters (the same as CathIA's, same defaults), and
//! `build` returns its poles. Lengths are millimetres, z is up, x runs along the length and y across.

use std::collections::BTreeMap;

use super::pole::{Build, End, MAX_POLES, Pole, SplitMode, Style, TenonOpts, V3, add, mul, sub};

/// One parameter of a model: a number, a choice among `options` (the value is its index) or a
/// check box (`check`, 0 or 1).
#[derive(Clone, Copy, Debug)]
pub struct ParamDef {
    pub key: &'static str,
    pub label: &'static str,
    pub default: f64,
    pub options: &'static [&'static str],
    pub check: bool,
}

const fn num(key: &'static str, label: &'static str, default: f64) -> ParamDef {
    ParamDef { key, label, default, options: &[], check: false }
}
const fn pick(key: &'static str, label: &'static str, default: f64, options: &'static [&'static str]) -> ParamDef {
    ParamDef { key, label, default, options, check: false }
}
const fn tick(key: &'static str, label: &'static str, default: f64) -> ParamDef {
    ParamDef { key, label, default, options: &[], check: true }
}

const TEN: ParamDef = pick("ten", "Tenons", 0.0, &["Tenons à la hachette", "Tenons sciés"]);
const TEN_SCIE: ParamDef = pick("ten", "Tenons", 1.0, &["Tenons à la hachette", "Tenons sciés"]);
const ASM: ParamDef = pick("asm", "Assemblage du cadre", 0.0, &["Optimisé : mi-bois dans le cadre, tenons aux pieds", "Tenons uniquement", "Mi-bois dans le cadre (chevillé par les pieds)"]);

#[derive(Clone, Copy, Debug)]
pub struct Model {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub summary: &'static str,
    pub params: &'static [ParamDef],
}

pub const MODELS: &[Model] = &[
    Model {
        id: "etabli",
        name: "Établi de camp",
        category: "Outils et ateliers",
        summary: "Deux perches inclinées tenues par une cheville, deux pieds en A et une pédale.",
        params: &[TEN, num("L", "Longueur des perches du corps (mm)", 2500.0), num("dP", "Diamètre des perches (mm)", 100.0), num("dPed", "Diamètre de la pédale (mm)", 60.0), num("ang", "Inclinaison du corps (°)", 20.0), num("ecart", "Écartement des pieds au sol (mm)", 1000.0)],
    },
    Model {
        id: "banc",
        name: "Banc court",
        category: "Assises",
        summary: "Un gros rondin posé sur quatre pieds tenonnés.",
        params: &[TEN, num("L", "Longueur du banc (mm)", 1500.0), num("dL", "Diamètre du rondin (mm, 200 au maximum)", 200.0), num("h", "Hauteur de l’assise (mm)", 450.0), tick("fendu", "Rondin fendu en deux (assise plate)", 1.0)],
    },
    Model {
        id: "fauteuil",
        name: "Fauteuil carré",
        category: "Assises",
        summary: "Quatre pieds, un cadre d'assise, des claies et un dossier.",
        params: &[TEN, num("larg", "Largeur d’assise (mm)", 650.0), num("prof", "Profondeur d’assise (mm)", 500.0), num("hs", "Hauteur de l’assise (mm)", 400.0), num("hd", "Hauteur totale du dossier (mm)", 900.0)],
    },
    Model {
        id: "bancdos",
        name: "Banc à dossier",
        category: "Assises",
        summary: "Assise en rondins fendus, montants arrière inclinés, dossier en rondin fendu.",
        params: &[TEN_SCIE, num("L", "Longueur du banc (mm)", 1400.0), num("hs", "Hauteur d’assise (mm)", 440.0), num("prof", "Profondeur d’assise (mm)", 420.0), num("Ht", "Hauteur du dossier (mm)", 850.0), num("ang", "Inclinaison des montants arrière (°)", 12.0), num("dP", "Diamètre des pieds et montants (mm)", 70.0), num("dS", "Diamètre des rondins d’assise (mm, fendus en deux)", 180.0), num("dd", "Diamètre du rondin du dossier (mm, fendu en deux)", 140.0)],
    },
    Model {
        id: "bancdiag",
        name: "Banc à dossier (diagonales)",
        category: "Assises",
        summary: "Les diagonales font le pied et le dossier ; l'assise les traverse.",
        params: &[TEN_SCIE, num("L", "Longueur du banc (mm)", 1400.0), num("hs", "Hauteur d’assise (mm)", 440.0), num("Ht", "Hauteur du dossier (mm)", 900.0), num("ang", "Inclinaison des diagonales depuis la verticale (°)", 25.0), num("angJ", "Inclinaison des jambes arrière (°)", 15.0), num("dD", "Diamètre des diagonales (mm)", 70.0), num("dS", "Diamètre du rondin d’assise (mm)", 180.0), num("dd", "Diamètre du rondin du dossier (mm, fendu en deux)", 140.0)],
    },
    Model {
        id: "table",
        name: "Table à pieds en X",
        category: "Tables et plans de travail",
        summary: "Deux paires de pieds croisés en mi-bois, deux lisses et un plateau de claies.",
        params: &[TEN_SCIE, num("long", "Longueur du plateau (mm)", 1800.0), num("larg", "Largeur du plateau (mm)", 800.0), num("h", "Hauteur du dessus du plateau (mm)", 750.0), pick("mode", "Plateau", 0.0, &["Claies tressées (sous, sur, sous)", "Claies droites dans des quarts de bois", "Filet de ficelle (maille serrée)", "Quarts de bois jointifs"]), num("maille", "Maille du filet (mm)", 30.0), num("dc", "Diamètre des claies (mm)", 20.0)],
    },
    Model {
        id: "prepa",
        name: "Table de prépa",
        category: "Tables et plans de travail",
        summary: "Quatre pieds, un cadre, un dessus de rondins fendus et un dessous de claies.",
        params: &[TEN_SCIE, ASM, num("long", "Longueur du plateau (mm)", 1600.0), num("larg", "Largeur du plateau (mm)", 700.0), num("h", "Hauteur du plan de travail (mm), 900 = standard cuisine", 900.0), num("dr", "Diamètre des rondins fendus du dessus (mm)", 70.0), num("dc", "Diamètre des claies du dessous (mm)", 20.0), tick("bas", "Dessous en claies", 1.0)],
    },
    Model {
        id: "cuisine",
        name: "Table de prépa avec étagère",
        category: "Tables et plans de travail",
        summary: "La table de prépa et, à côté, une colonne étagère.",
        params: &[TEN_SCIE, ASM, num("long", "Longueur du plan de travail (mm)", 1800.0), num("larg", "Profondeur du plan et de la colonne (mm)", 700.0), num("lc", "Largeur de la colonne étagère (mm)", 600.0), num("h", "Hauteur du plan de travail (mm), 900 = standard cuisine", 900.0), num("ht", "Hauteur de la colonne étagère (mm), ≈ 2× le plan", 1800.0), pick("cote", "Colonne étagère", 0.0, &["À droite du plan de travail", "À gauche du plan de travail"]), num("dr", "Diamètre des rondins fendus du dessus (mm)", 70.0), num("dc", "Diamètre des claies (mm)", 20.0)],
    },
    Model {
        id: "etagere",
        name: "Étagère",
        category: "Rangements",
        summary: "Quatre montants et trois étagères : claies, claies, filet.",
        params: &[TEN_SCIE, ASM, num("long", "Longueur (mm)", 1000.0), num("larg", "Profondeur (mm)", 500.0), num("h", "Hauteur totale (mm)", 1600.0), num("h1", "Hauteur de l’étagère basse, claies tressées (mm)", 400.0), num("h2", "Hauteur de l’étagère médiane, quart de bois (mm)", 900.0), num("dc", "Diamètre des claies (mm)", 20.0), num("maille", "Maille du filet du haut (mm)", 30.0)],
    },
    Model {
        id: "vaisselier",
        name: "Vaisselier",
        category: "Rangements",
        summary: "Une petite table haute au plateau en filet, pour poser la bassine.",
        params: &[TEN_SCIE, num("long", "Longueur du plateau (mm)", 900.0), num("larg", "Largeur du plateau (mm), à ajuster à la bassine", 380.0), num("h", "Hauteur du dessus (mm)", 750.0), num("maille", "Maille du filet (mm)", 30.0)],
    },
    Model {
        id: "feu",
        name: "Table à feu",
        category: "Tables et plans de travail",
        summary: "Une table basse au plateau de planches.",
        params: &[TEN_SCIE, ASM, num("long", "Longueur du plateau (mm)", 1000.0), num("larg", "Largeur du plateau (mm)", 800.0), num("h", "Hauteur du dessus (mm)", 700.0), num("ep", "Épaisseur des planches (mm)", 30.0), num("lp", "Largeur des planches (mm)", 100.0)],
    },
];

pub fn model(id: &str) -> Option<&'static Model> {
    MODELS.iter().find(|m| m.id == id)
}

/// The values of a model's parameters: the given ones over the defaults.
#[derive(Clone, Debug, Default)]
pub struct Params(pub BTreeMap<String, f64>);

impl Params {
    pub fn of(m: &Model) -> Params {
        Params(m.params.iter().map(|p| (p.key.to_string(), p.default)).collect())
    }
    pub fn get(&self, k: &str) -> f64 {
        self.0.get(k).copied().unwrap_or(0.0)
    }
    pub fn set(&mut self, k: &str, v: f64) {
        self.0.insert(k.to_string(), v);
    }
    /// The parameters as a JSON object (the numbers as given).
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.0).unwrap_or_default()
    }
}

/// Checks a parameter set against the model: only its keys, each within sane sizes.
pub fn check(m: &Model, p: &Params) -> Result<(), String> {
    for k in p.0.keys() {
        if !m.params.iter().any(|d| d.key == k) {
            return Err(format!("« {k} » n'est pas un réglage de « {} »", m.name));
        }
    }
    for d in m.params {
        let v = p.get(d.key);
        if !v.is_finite() {
            return Err(format!("« {} » n'est pas un nombre", d.label));
        }
        if !d.options.is_empty() {
            if v < 0.0 || v as usize >= d.options.len() || v.fract() != 0.0 {
                return Err(format!("« {} » : choix inconnu", d.label));
            }
        } else if d.check {
            if v != 0.0 && v != 1.0 {
                return Err(format!("« {} » : oui ou non", d.label));
            }
        } else if d.key == "ang" || d.key == "angJ" {
            if !(0.0..=45.0).contains(&v) {
                return Err(format!("« {} » : entre 0 et 45°", d.label));
            }
        } else if d.default > 0.0 && !(d.default / 20.0..=d.default * 5.0).contains(&v) {
            return Err(format!("« {} » : entre {} et {}", d.label, (d.default / 20.0).ceil(), d.default * 5.0));
        }
    }
    Ok(())
}

/// The poles of a model.
pub fn build(id: &str, p: &Params) -> Result<Vec<Pole>, String> {
    let m = model(id).ok_or_else(|| format!("meuble inconnu « {id} » (scout.meuble.types les liste)"))?;
    check(m, p)?;
    let mut b = Build::default();
    let style = if p.get("ten") >= 1.0 { Style::Scie } else { Style::Hache };
    let ten = TenonOpts::new(style);
    match id {
        "etabli" => etabli(&mut b, p, ten)?,
        "banc" => banc(&mut b, p, ten)?,
        "fauteuil" => fauteuil(&mut b, p, ten)?,
        "bancdos" => bancdos(&mut b, p, ten)?,
        "bancdiag" => bancdiag(&mut b, p, ten)?,
        "table" => table(&mut b, p, ten)?,
        "prepa" | "cuisine" => prepa(&mut b, p, ten, id == "cuisine")?,
        "etagere" => etagere(&mut b, p, ten)?,
        "vaisselier" => vaisselier(&mut b, p, ten)?,
        "feu" => feu(&mut b, p, ten)?,
        _ => return Err(format!("meuble inconnu « {id} »")),
    }
    if b.poles.len() > MAX_POLES {
        return Err(format!("{} pièces : plus que les {MAX_POLES} permises", b.poles.len()));
    }
    Ok(b.poles)
}

fn v(x: f64, y: f64, z: f64) -> V3 {
    [x, y, z]
}

/// How the corners of a frame are joined: tenons in the legs, or half-laps (pegged by the legs).
#[derive(Clone, Copy, PartialEq)]
enum Asm {
    Tenons,
    Mibois,
}

fn asm_of(p: &Params) -> Asm {
    if p.get("asm") == 1.0 { Asm::Tenons } else { Asm::Mibois }
}

/// A rectangular frame at height `z` (the axis of the lisses) between four legs (x0,y0), (x1,y0),
/// (x0,y1), (x1,y1) given in this order. Returns the two lisses (along x) and two traverses.
#[allow(dead_code)]
struct Frame {
    lisses: [usize; 2],
    traverses: [usize; 2],
}

fn frame(b: &mut Build, ten: TenonOpts, asm: Asm, (x0, x1, y0, y1): (f64, f64, f64, f64), z: f64, legs: [usize; 4], dl: f64, dt: f64) -> Result<Frame, String> {
    let r = dl / 2.0;
    match asm {
        Asm::Tenons => {
            let l0 = b.bar("lisse", dl, v(x0, y0, z), v(x1, y0, z));
            let l1 = b.bar("lisse", dl, v(x0, y1, z), v(x1, y1, z));
            // the traverses sit lower so that their tenons meet the legs beside the lisses' tenons
            let zt = z - dl * 0.6;
            let t0 = b.bar("traverse", dt, v(x0, y0, zt), v(x0, y1, zt));
            let t1 = b.bar("traverse", dt, v(x1, y0, zt), v(x1, y1, zt));
            b.tenon(l0, legs[0], ten)?;
            b.tenon(l0, legs[1], ten)?;
            b.tenon(l1, legs[2], ten)?;
            b.tenon(l1, legs[3], ten)?;
            b.tenon(t0, legs[0], ten)?;
            b.tenon(t0, legs[2], ten)?;
            b.tenon(t1, legs[1], ten)?;
            b.tenon(t1, legs[3], ten)?;
            Ok(Frame { lisses: [l0, l1], traverses: [t0, t1] })
        }
        Asm::Mibois => {
            // lisses and traverses cross over the legs; each traverse lies half a pole lower
            let e = 30.0 + r;
            let zt = z - r;
            let l0 = b.bar("lisse", dl, v(x0 - e, y0, z), v(x1 + e, y0, z));
            let l1 = b.bar("lisse", dl, v(x0 - e, y1, z), v(x1 + e, y1, z));
            let t0 = b.bar("traverse", dt, v(x0, y0 - e, zt), v(x0, y1 + e, zt));
            let t1 = b.bar("traverse", dt, v(x1, y0 - e, zt), v(x1, y1 + e, zt));
            for &t in &[t0, t1] {
                for &l in &[l0, l1] {
                    b.mibois(t, l, 50.0)?;
                }
            }
            // the legs' tenons rise through both poles of each corner
            for (leg, l) in [(legs[0], l0), (legs[1], l0), (legs[2], l1), (legs[3], l1)] {
                if b.poles[leg].end()[2] > z + 1.0 {
                    // a post that goes on above the frame: the lisse and its traverse pass through it
                    let t = if l == l0 { [t0, t1] } else { [t0, t1] };
                    b.through(l, leg)?;
                    for tr in t {
                        if closest_near(b, tr, leg) {
                            b.through(tr, leg)?;
                        }
                    }
                } else {
                    b.tenon(leg, l, ten.through())?;
                }
            }
            Ok(Frame { lisses: [l0, l1], traverses: [t0, t1] })
        }
    }
}

/// Whether pole `a`'s line passes within a pole's reach of pole `b` (they meet).
fn closest_near(b: &Build, a: usize, c: usize) -> bool {
    let (pa, pc) = (&b.poles[a], &b.poles[c]);
    match crate::scout::pole::closest(pa.origin, pa.dir, pc.origin, pc.dir) {
        Some((x, y, ta, _)) => {
            let d = crate::scout::pole::sub(x, y);
            crate::scout::pole::len(d) < pc.radius() && ta >= pa.start - pc.radius() && ta <= pa.start + pa.length + pc.radius()
        }
        None => false,
    }
}

/// Four legs of diameter `d` and heights `hs` for the corners (x0,y0), (x1,y0), (x0,y1), (x1,y1).
fn legs4(b: &mut Build, d: f64, (x0, x1, y0, y1): (f64, f64, f64, f64), hs: [f64; 4]) -> [usize; 4] {
    let pts = [(x0, y0), (x1, y0), (x0, y1), (x1, y1)];
    let mut out = [0; 4];
    for (i, (x, y)) in pts.iter().enumerate() {
        out[i] = b.bar(if hs[i] > hs[0] + 1.0 { "montant" } else { "pied" }, d, v(*x, *y, 0.0), v(*x, *y, hs[i]));
    }
    out
}

/// How a plateau is filled between its two rails (CathIA's `plateauWin`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fill {
    /// claies woven under, over, under the two rails and a thin central one
    Tresse,
    /// straight claies lying on the rails
    Droit,
    /// a net of rope
    Filet,
    /// quarter-logs side by side
    Quarts,
}

impl Fill {
    pub fn from_index(i: usize) -> Fill {
        match i {
            0 => Fill::Tresse,
            1 => Fill::Droit,
            2 => Fill::Filet,
            _ => Fill::Quarts,
        }
    }
    pub fn from_key(k: &str) -> Option<Fill> {
        match k {
            "tresse" => Some(Fill::Tresse),
            "droit" => Some(Fill::Droit),
            "filet" => Some(Fill::Filet),
            "quarts" => Some(Fill::Quarts),
            _ => None,
        }
    }
}

/// What a plateau needs besides its rectangle: the claie diameter, the net's mesh, the rails'
/// diameter, where the claying starts (`c0`) and how long it runs (`cl`, 0 = to the end).
#[derive(Clone, Copy, Debug)]
pub struct FillOpts {
    pub dc: f64,
    pub maille: f64,
    pub rail: f64,
    pub c0: f64,
    pub cl: f64,
}

/// Fills the rectangle `xs` × `ys`, resting on rails whose top is at `z_top` (the rails run along x
/// at the two ends of `ys`; the claies run across along y). Under 150 pieces; no joints, so cheap.
pub fn fill(b: &mut Build, mode: Fill, (xa, xb): (f64, f64), (y0, y1): (f64, f64), z_top: f64, o: FillOpts) -> Result<(), String> {
    let (xa, xb) = (xa + o.c0.max(0.0), if o.cl > 0.0 { (xa + o.c0.max(0.0) + o.cl).min(xb) } else { xb });
    if xb - xa < o.dc.max(10.0) {
        return Err("plateau trop court pour la longueur clayée demandée".into());
    }
    let d = o.dc;
    match mode {
        Fill::Filet => {
            net(b, (xa, xb, y0, y1), z_top + 2.0, o.maille);
        }
        Fill::Droit => {
            let n = (((xb - xa) / (d * 1.4)).floor().max(1.0)) as usize;
            let n = n.min(120);
            for i in 0..n {
                let x = xa + (xb - xa) * (i as f64 + 0.5) / n as f64;
                b.bar("claie", d, v(x, y0, z_top + d / 2.0), v(x, y1, z_top + d / 2.0));
            }
        }
        Fill::Quarts => {
            let n = (((xb - xa) / (d * 0.6)).floor().max(1.0)) as usize;
            let n = n.min(120);
            for i in 0..n {
                let x = xa + (xb - xa) * (i as f64 + 0.5) / n as f64;
                let k = b.bar("quart de bois", d, v(x, y0, z_top + d / 2.0), v(x, y1, z_top + d / 2.0));
                b.split(k, SplitMode::Quart)?;
            }
        }
        Fill::Tresse => {
            // a thin rail down the middle, and every claie goes under the outer rails and over the
            // middle one, or the other way round: two straight bars, bent at the middle
            let (ym, m, rail) = ((y0 + y1) / 2.0, d * 0.8, o.rail.max(d));
            let zc = z_top - rail / 2.0;
            b.bar("lien central", m, v(xa, ym, zc), v(xb, ym, zc));
            let n = (((xb - xa) / (d * 1.5)).floor().max(1.0)) as usize;
            let n = n.min(70);
            let (up, down) = (z_top + d / 2.0, z_top - rail - d / 2.0);
            let (over, under) = (zc + m / 2.0 + d / 2.0, zc - m / 2.0 - d / 2.0);
            for i in 0..n {
                let x = xa + (xb - xa) * (i as f64 + 0.5) / n as f64;
                let (e, c) = if i % 2 == 0 { (up, under) } else { (down, over) };
                b.bar("claie", d, v(x, y0, e), v(x, ym, c));
                b.bar("claie", d, v(x, ym, c), v(x, y1, e));
            }
        }
    }
    Ok(())
}

/// Straight claies across (along y) from x `xa` to `xb`, resting on top of rails at `z_top`.
fn claies(b: &mut Build, xs: (f64, f64), ys: (f64, f64), z_top: f64, d: f64) {
    let o = FillOpts { dc: d, maille: 30.0, rail: 50.0, c0: 0.0, cl: 0.0 };
    let _ = fill(b, Fill::Tresse, xs, ys, z_top, o);
}

/// A net of ropes (maille `m`) over a rectangle at height `z`.
fn net(b: &mut Build, (x0, x1, y0, y1): (f64, f64, f64, f64), z: f64, m: f64) {
    let m = m.max(((x1 - x0) + (y1 - y0)) / 160.0).max(10.0);
    let (nx, ny) = (((x1 - x0) / m).floor() as usize, ((y1 - y0) / m).floor() as usize);
    for i in 0..=nx {
        let x = x0 + (x1 - x0) * i as f64 / nx.max(1) as f64;
        b.bar("ficelle", 4.0, v(x, y0, z), v(x, y1, z));
    }
    for j in 0..=ny {
        let y = y0 + (y1 - y0) * j as f64 / ny.max(1) as f64;
        b.bar("ficelle", 4.0, v(x0, y, z + 4.0), v(x1, y, z + 4.0));
    }
}

/// Half-logs of diameter `d` across (along y), flat face up, resting on lisses whose top is at `z_l`.
fn half_logs(b: &mut Build, (xa, xb): (f64, f64), (y0, y1): (f64, f64), z_l: f64, d: f64) -> Result<(), String> {
    let n = (((xb - xa) / d).floor().max(1.0)) as usize;
    let n = n.min(120);
    for i in 0..n {
        let x = xa + (xb - xa) * (i as f64 + 0.5) / n as f64;
        let k = b.bar("rondin fendu", d, v(x, y0, z_l + d / 2.0), v(x, y1, z_l + d / 2.0));
        b.split(k, SplitMode::Demi)?;
    }
    Ok(())
}

fn banc(b: &mut Build, p: &Params, ten: TenonOpts) -> Result<(), String> {
    let (l, dl, h) = (p.get("L"), p.get("dL").min(200.0), p.get("h"));
    let flat = p.get("fendu") != 0.0;
    let zc = if flat { h } else { h - dl / 2.0 };
    let yc = dl / 2.0;
    let seat = b.bar("rondin d'assise", dl, v(0.0, yc, zc), v(l, yc, zc));
    if flat {
        b.split(seat, SplitMode::Demi)?;
    }
    let dp = (dl * 0.45).max(60.0);
    for x in [l * 0.15, l * 0.85] {
        for s in [-1.0, 1.0] {
            let y = yc + s * dl * 0.25;
            let leg = b.bar("pied", dp, v(x, y, 0.0), v(x, y, zc));
            b.tenon(leg, seat, ten)?;
        }
    }
    Ok(())
}

fn etabli(b: &mut Build, p: &Params, ten: TenonOpts) -> Result<(), String> {
    let (l, dp, dped) = (p.get("L"), p.get("dP"), p.get("dPed"));
    if p.get("ecart") < 800.0 {
        return Err("établi instable : écartez les pieds d'au moins 80 cm au sol, sinon il bascule sur le côté quand on appuie sur la pédale".into());
    }
    let a = p.get("ang").to_radians();
    let dir = v(a.cos(), 0.0, a.sin());
    let r = dp / 2.0;
    let yy = (dp + 14.0) / 2.0;
    let z0 = r.max(dped / 2.0);
    let base = |y: f64| v(0.0, y, z0);
    let at = |y: f64, s: f64| add(base(y), mul(dir, s));
    let pl = b.bar("perche gauche", dp, base(-yy), at(-yy, l));
    let pr = b.bar("perche droite", dp, base(yy), at(yy, l));
    // two pegs hold the perches together, at 0.40 m and 5 cm from the end, and one is the pedal's hinge
    for s in [150.0, 400.0, l - 50.0] {
        b.peg_line(pl, pr, at(0.0, s), v(0.0, 1.0, 0.0), 30.0, 2.0 * yy + dp + 20.0)?;
    }
    // the two A-legs, tenoned into the perches just under the upper peg
    let sc = l - 50.0 - 160.0;
    for (i, s) in [-1.0, 1.0].iter().enumerate() {
        let top = at(s * yy, sc);
        let foot = v(top[0] - 60.0, s * p.get("ecart") / 2.0, 0.0);
        let leg = b.bar("pied", 40.0, foot, top);
        b.tenon(leg, if i == 0 { pl } else { pr }, TenonOpts { d: 28.0, ..ten })?;
    }
    // the pedal, a pole lying under the body
    b.bar("pédale", dped, v(0.0, 0.0, dped / 2.0), v(l * 0.45, 0.0, dped / 2.0 + l * 0.45 * a.tan() * 0.0));
    Ok(())
}

fn fauteuil(b: &mut Build, p: &Params, ten: TenonOpts) -> Result<(), String> {
    let (w, d, hs, hd) = (p.get("larg"), p.get("prof"), p.get("hs"), p.get("hd"));
    let dl = 60.0;
    let bx = (0.0, w, 0.0, d);
    let zr = hs - 30.0;
    let legs = legs4(b, dl, bx, [hs, hs, hd, hd]);
    let f = frame(b, ten, Asm::Tenons, bx, zr, legs, 40.0, 40.0)?;
    let _ = f;
    // the back: two rails between the rear legs
    for z in [hd - 120.0, hd - 340.0] {
        let t = b.bar("traverse de dossier", 40.0, v(0.0, d, z), v(w, d, z));
        b.tenon(t, legs[2], ten)?;
        b.tenon(t, legs[3], ten)?;
    }
    claies(b, (0.0, w), (0.0, d), zr + 20.0 - 10.0, 25.0);
    Ok(())
}

fn bancdos(b: &mut Build, p: &Params, ten: TenonOpts) -> Result<(), String> {
    let (l, hs, prof, ht) = (p.get("L"), p.get("hs"), p.get("prof"), p.get("Ht"));
    let (dp, ds, dd) = (p.get("dP"), p.get("dS"), p.get("dd"));
    let tan = p.get("ang").to_radians().tan();
    let x0 = dp / 2.0;
    let x1 = l - dp / 2.0;
    let rear_y = |z: f64| prof + z * tan;
    let mut front = [0; 2];
    let mut rear = [0; 2];
    for (i, x) in [x0, x1].iter().enumerate() {
        front[i] = b.bar("pied", dp, v(*x, 0.0, 0.0), v(*x, 0.0, hs));
        rear[i] = b.bar("montant arrière", dp, v(*x, rear_y(0.0), 0.0), v(*x, rear_y(ht), ht));
    }
    // a rail on each side under the seat, from the front leg to the rear post
    let zr = hs - ds / 2.0 - dp / 2.0;
    for i in 0..2 {
        let rail = b.bar("traverse d'assise", dp * 0.8, v([x0, x1][i], 0.0, zr), v([x0, x1][i], rear_y(zr), zr));
        b.tenon(rail, front[i], ten)?;
        b.tenon(rail, rear[i], ten)?;
    }
    // the seat: split logs lying along x, flat face up
    let n = ((prof / ds).round().max(2.0)) as usize;
    for k in 0..n {
        let y = ds / 2.0 + (prof - ds) * k as f64 / (n - 1) as f64;
        let zc = zr + dp * 0.8 / 2.0 + ds / 2.0;
        let log = b.bar("rondin d'assise", ds, v(0.0, y, zc), v(l, y, zc));
        b.split(log, SplitMode::Demi)?;
    }
    // the back: a split log between the posts, flat face to the front
    let zb = ht - dd / 2.0;
    let mut back = Pole::bar("rondin de dossier", dd, v(x0, rear_y(zb), zb), v(x1, rear_y(zb), zb));
    back.up = v(0.0, -1.0, 0.0);
    b.poles.push(back);
    let bk = b.poles.len() - 1;
    b.split(bk, SplitMode::Demi)?;
    b.through(rear[0], bk).ok();
    Ok(())
}

fn bancdiag(b: &mut Build, p: &Params, ten: TenonOpts) -> Result<(), String> {
    let (l, hs, ht) = (p.get("L"), p.get("hs"), p.get("Ht"));
    let (dd_, ds, dd) = (p.get("dD"), p.get("dS"), p.get("dd"));
    let tan = p.get("ang").to_radians().tan();
    let tanj = p.get("angJ").to_radians().tan();
    let zlog = hs - ds / 2.0;
    let ylog = zlog * tan;
    let xs = [l * 0.1, l * 0.9];
    let log = b.bar("rondin d'assise", ds, v(0.0, ylog, zlog), v(l, ylog, zlog));
    b.split(log, SplitMode::Demi)?;
    let mut back = Pole::bar("rondin de dossier", dd, v(xs[0], ht * tan, ht - dd / 2.0), v(xs[1], ht * tan, ht - dd / 2.0));
    back.up = v(0.0, -1.0, 0.0);
    b.poles.push(back);
    let bk = b.poles.len() - 1;
    b.split(bk, SplitMode::Demi)?;
    for x in xs {
        // the diagonal is the foot and the back in one pole; it goes through the seat log
        let dg = b.bar("diagonale", dd_, v(x, 0.0, 0.0), v(x, (ht - dd / 2.0) * tan, ht - dd / 2.0));
        b.through(dg, log)?;
        b.tenon(dg, bk, ten)?;
        // the rear leg, tenoned into the log
        let leg = b.bar("jambe arrière", dd_ * 0.9, v(x, ylog + zlog * tanj, 0.0), v(x, ylog, zlog));
        b.tenon(leg, log, ten)?;
    }
    Ok(())
}

fn table(b: &mut Build, p: &Params, ten: TenonOpts) -> Result<(), String> {
    let (l, w, h, dc) = (p.get("long"), p.get("larg"), p.get("h"), p.get("dc"));
    let dl = 70.0;
    let ztop = h - dc - dl / 2.0;
    let (xa, xb) = (l * 0.12, l * 0.88);
    let (ya, yb) = (w * 0.08, w * 0.92);
    // the lisses carry the top
    let l0 = b.bar("lisse", dl, v(0.0, ya, ztop), v(l, ya, ztop));
    let l1 = b.bar("lisse", dl, v(0.0, yb, ztop), v(l, yb, ztop));
    for x in [xa, xb] {
        // two legs crossing as an X in the plane x, one a little each side, half-lapped
        let off = dl / 4.0;
        let a = b.bar("pied en X", dl, v(x - off, ya, ztop), v(x - off, yb, 0.0));
        let c = b.bar("pied en X", dl, v(x + off, yb, ztop), v(x + off, ya, 0.0));
        b.mibois(a, c, 50.0)?;
        b.tenon(a, l0, ten)?;
        b.tenon(c, l1, ten)?;
    }
    // a thin central rail holds the legs apart
    let o = FillOpts { dc, maille: p.get("maille"), rail: dl, c0: 0.0, cl: 0.0 };
    fill(b, Fill::from_index(p.get("mode") as usize), (0.0, l), (0.0, w), ztop + dl / 2.0, o)?;
    Ok(())
}

fn prepa(b: &mut Build, p: &Params, ten: TenonOpts, with_column: bool) -> Result<(), String> {
    let (l, w, h) = (p.get("long"), p.get("larg"), p.get("h"));
    let (dr, dc) = (p.get("dr"), p.get("dc"));
    let dl = 80.0;
    let dlis = 60.0;
    let zl = h - dr / 2.0 - dlis / 2.0 - dr / 2.0;
    let asm = asm_of(p);
    let r = dl / 2.0;
    let bx = (r, l - r, r, w - r);
    let tall = if with_column { p.get("ht") } else { zl };
    let right_tall = with_column && p.get("cote") == 0.0;
    let hs = if right_tall { [zl, tall, zl, tall] } else if with_column { [tall, zl, tall, zl] } else { [zl; 4] };
    let legs = legs4(b, dl, bx, hs);
    let f = frame(b, ten, asm, bx, zl, legs, dlis, dlis)?;
    half_logs(b, (0.0, l), (0.0, w), zl + dlis / 2.0, dr)?;
    if p.get("bas") != 0.0 || with_column {
        // the lower shelf: two rails tenoned into the legs, claies across
        let zb = 250.0;
        let y0 = bx.2;
        let y1 = bx.3;
        let r0 = b.bar("lisse basse", dlis, v(bx.0, y0, zb), v(bx.1, y0, zb));
        let r1 = b.bar("lisse basse", dlis, v(bx.0, y1, zb), v(bx.1, y1, zb));
        b.tenon(r0, legs[0], ten)?;
        b.tenon(r0, legs[1], ten)?;
        b.tenon(r1, legs[2], ten)?;
        b.tenon(r1, legs[3], ten)?;
        claies(b, (0.0, l), (0.0, w), zb + dlis / 2.0, dc);
    }
    let _ = f;
    if with_column {
        // the column: two more posts beyond the table and shelves of claies at three heights
        let lc = p.get("lc");
        let (cx0, cx1) = if right_tall { (bx.1, bx.1 + lc) } else { (-lc + r, r) };
        let (xa, xb) = (cx0.min(cx1), cx0.max(cx1));
        let c0 = b.bar("montant", dl, v(if right_tall { xb } else { xa }, bx.2, 0.0), v(if right_tall { xb } else { xa }, bx.2, tall));
        let c1 = b.bar("montant", dl, v(if right_tall { xb } else { xa }, bx.3, 0.0), v(if right_tall { xb } else { xa }, bx.3, tall));
        let shared = if right_tall { [legs[1], legs[3]] } else { [legs[0], legs[2]] };
        for z in [tall / 3.0, 2.0 * tall / 3.0, tall - dlis / 2.0] {
            for (a, c, y) in [(shared[0], c0, bx.2), (shared[1], c1, bx.3)] {
                let _ = (a, c);
                let rail = b.bar("lisse d'étagère", dlis, v(xa, y, z), v(xb, y, z));
                b.tenon(rail, shared[usize::from(y > bx.2)], ten)?;
                b.tenon(rail, if y > bx.2 { c1 } else { c0 }, ten)?;
            }
            claies(b, (xa, xb), (0.0, w), z + dlis / 2.0, dc);
        }
    }
    Ok(())
}

fn etagere(b: &mut Build, p: &Params, ten: TenonOpts) -> Result<(), String> {
    let (l, w, h) = (p.get("long"), p.get("larg"), p.get("h"));
    let (h1, h2, dc) = (p.get("h1"), p.get("h2"), p.get("dc"));
    let dl = 70.0;
    let dr = 50.0;
    let r = dl / 2.0;
    let bx = (r, l - r, r, w - r);
    let legs = legs4(b, dl, bx, [h; 4]);
    for (z, kind) in [(h1, 0), (h2, 1), (h - dr, 2)] {
        let f = frame(b, ten, asm_of(p), bx, z, legs, dr, dr)?;
        let _ = f;
        let o = FillOpts { dc, maille: p.get("maille"), rail: dr, c0: 0.0, cl: 0.0 };
        let mode = [Fill::Tresse, Fill::Quarts, Fill::Filet][kind];
        fill(b, mode, (0.0, l), (0.0, w), z + dr / 2.0, o)?;
    }
    Ok(())
}

fn vaisselier(b: &mut Build, p: &Params, ten: TenonOpts) -> Result<(), String> {
    let (l, w, h) = (p.get("long"), p.get("larg"), p.get("h"));
    let dl = 60.0;
    let dr = 40.0;
    let r = dl / 2.0;
    let bx = (r, l - r, r, w - r);
    let z = h - dr / 2.0 - 4.0;
    let legs = legs4(b, dl, bx, [z; 4]);
    frame(b, ten, Asm::Tenons, bx, z, legs, dr, dr)?;
    net(b, (0.0, l, 0.0, w), z + dr / 2.0, p.get("maille"));
    // a lower rail ties each pair of legs
    for (a, c) in [(0, 1), (2, 3)] {
        let (pa, pc) = (b.poles[legs[a]].p_start(), b.poles[legs[c]].p_start());
        let t = b.bar("traverse basse", dr, add(pa, v(0.0, 0.0, 200.0)), add(pc, v(0.0, 0.0, 200.0)));
        b.tenon(t, legs[a], ten)?;
        b.tenon(t, legs[c], ten)?;
    }
    Ok(())
}

fn feu(b: &mut Build, p: &Params, ten: TenonOpts) -> Result<(), String> {
    let (l, w, h) = (p.get("long"), p.get("larg"), p.get("h"));
    let (ep, lp) = (p.get("ep"), p.get("lp"));
    let dl = 70.0;
    let dlis = 50.0;
    let r = dl / 2.0;
    let bx = (r + 20.0, l - r - 20.0, r + 20.0, w - r - 20.0);
    let zl = h - ep - dlis / 2.0;
    let legs = legs4(b, dl, bx, [zl; 4]);
    frame(b, ten, asm_of(p), bx, zl, legs, dlis, dlis)?;
    let n = (l / (lp + 4.0)).floor().max(1.0).min(120.0) as usize;
    let pitch = l / n as f64;
    for i in 0..n {
        let x = pitch * (i as f64 + 0.5);
        let z = zl + dlis / 2.0 + ep / 2.0;
        b.board("planche", ep, pitch.min(lp) - 4.0, v(x, 0.0, z), v(x, w, z), v(0.0, 0.0, 1.0));
    }
    Ok(())
}

/// Silences an unused-import lint when a model does not need every helper.
#[allow(dead_code)]
fn _unused(_: End, _: V3) -> V3 {
    sub([0.0; 3], [0.0; 3])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scout::pole::{cut_list, extent, wood_metres};

    #[test]
    fn the_eleven_models_of_cathia_are_all_there() {
        let names: Vec<&str> = MODELS.iter().map(|m| m.name).collect();
        for n in ["Établi de camp", "Banc court", "Fauteuil carré", "Banc à dossier", "Banc à dossier (diagonales)", "Table à pieds en X", "Table de prépa", "Table de prépa avec étagère", "Étagère", "Vaisselier", "Table à feu"] {
            assert!(names.contains(&n), "{n}");
        }
        assert_eq!(MODELS.len(), 11);
    }

    #[test]
    fn every_model_builds_with_its_defaults() {
        for m in MODELS {
            let poles = build(m.id, &Params::of(m)).unwrap_or_else(|e| panic!("{}: {e}", m.id));
            assert!(poles.len() >= 5 && poles.len() <= MAX_POLES, "{}: {}", m.id, poles.len());
            assert!(poles.iter().all(|p| p.length > 0.0 && p.length.is_finite()), "{}", m.id);
            assert!(wood_metres(&poles) > 1.0, "{}", m.id);
            assert!(!cut_list(&poles).is_empty());
            let e = extent(&poles);
            assert!(e.iter().all(|x| x.is_finite() && *x > 100.0), "{}: {e:?}", m.id);
        }
    }

    #[test]
    fn every_pole_record_fits_its_property() {
        for m in MODELS {
            for p in build(m.id, &Params::of(m)).unwrap_or_default() {
                let text = serde_json::to_string(&p).unwrap_or_default();
                assert!(text.len() < 4000, "{} {} {}", m.id, p.role, text.len());
            }
        }
    }

    #[test]
    fn the_models_have_real_joints() {
        let poles = build("table", &Params::of(model("table").unwrap_or(&MODELS[0]))).unwrap_or_default();
        let joints = crate::scout::pole::joint_summary(&poles);
        assert!(joints.iter().any(|(k, _)| k == "mi-bois"), "{joints:?}");
        assert!(joints.iter().any(|(k, _)| k.contains("tenon")), "{joints:?}");
    }

    #[test]
    fn the_overall_size_follows_the_parameters() {
        let m = model("prepa").unwrap_or(&MODELS[0]);
        let mut p = Params::of(m);
        p.set("long", 2000.0);
        let e = extent(&build("prepa", &p).unwrap_or_default());
        assert!((e[0] - 2000.0).abs() < 120.0, "{e:?}");
        assert!((e[2] - p.get("h")).abs() < 120.0, "{e:?}");
    }

    #[test]
    fn hostile_parameters_are_refused() {
        let m = model("banc").unwrap_or(&MODELS[0]);
        for (k, val) in [("L", f64::NAN), ("L", 1e12), ("L", -5.0), ("h", f64::INFINITY), ("fendu", 3.0), ("ten", 7.0), ("nope", 1.0)] {
            let mut p = Params::of(m);
            p.set(k, val);
            assert!(build("banc", &p).is_err(), "{k}={val}");
        }
        assert!(build("fusée", &Params::default()).is_err());
        let e = model("etabli").unwrap_or(&MODELS[0]);
        let mut p = Params::of(e);
        p.set("ecart", 400.0);
        assert!(build("etabli", &p).is_err());
    }
}
