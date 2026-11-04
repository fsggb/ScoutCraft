//! Froissartage furniture, as lists of round poles. Lengths are millimetres, z is up, the
//! furniture's footprint corner is the origin, x runs along its length and y along its width.
//!
//! A pole is a cylinder along one world axis (`Pole::axis`) from `origin` (the centre of its
//! start face). The generators keep to a few honest rules of camp building: the legs are the
//! thickest poles, the rails are tenoned into them (a tenon is a third of the pole's diameter, the
//! "rule of thirds"), the top is a row of thin poles (claies) laid on the long rails.

use serde::{Deserialize, Serialize};

/// Most poles one piece of furniture may have (a hostile length cannot ask for millions).
pub const MAX_POLES: usize = 400;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub fn name(self) -> &'static str {
        match self {
            Axis::X => "X",
            Axis::Y => "Y",
            Axis::Z => "Z",
        }
    }
    pub fn parse(s: &str) -> Option<Axis> {
        match s {
            "X" => Some(Axis::X),
            "Y" => Some(Axis::Y),
            "Z" => Some(Axis::Z),
            _ => None,
        }
    }
    /// The row-major 3×4 rigid matrix that turns a pole built along +z into one along this axis
    /// starting at `origin`.
    pub fn matrix(self, origin: [f64; 3]) -> [f64; 12] {
        let [x, y, z] = origin;
        match self {
            Axis::Z => [1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, z],
            // rotation about y by +90°: z → x
            Axis::X => [0.0, 0.0, 1.0, x, 0.0, 1.0, 0.0, y, -1.0, 0.0, 0.0, z],
            // rotation about x by −90°: z → y
            Axis::Y => [1.0, 0.0, 0.0, x, 0.0, 0.0, 1.0, y, 0.0, -1.0, 0.0, z],
        }
    }
}

/// How a pole is held to the others (the assemblies of the plan's list).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Joint {
    /// Round tenon in a mortise through the leg.
    TenonMortaise,
    /// Laid on the rails and lashed or pegged.
    Posee,
}

impl Joint {
    pub fn label(self) -> &'static str {
        match self {
            Joint::TenonMortaise => "tenon et mortaise",
            Joint::Posee => "posée et brêlée",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Pole {
    /// Role: pied, traverse longue, traverse courte, claie, montant de dossier…
    pub role: &'static str,
    pub diameter: f64,
    pub length: f64,
    pub origin: [f64; 3],
    pub axis: Axis,
    pub joint: Joint,
    /// Diameter of the pole this one is tenoned into (0: none); its tenon goes to that pole's axis.
    pub receiver: f64,
}

impl Pole {
    /// What the pole is cut to from the log: its visible length plus a tenon at each end that has
    /// one, each reaching the axis of the pole it enters.
    pub fn cut_length(&self) -> f64 {
        self.length + self.receiver
    }
    /// The end point of the pole's axis.
    pub fn end(&self) -> [f64; 3] {
        let mut e = self.origin;
        let i = match self.axis {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        };
        e[i] += self.length;
        e
    }
    /// The tenon's diameter by the rule of thirds, rounded up to the millimetre.
    pub fn tenon(&self) -> f64 {
        (self.diameter / 3.0).ceil()
    }
}

/// What a template is called and takes.
#[derive(Clone, Copy, Debug)]
pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub summary: &'static str,
    /// length, width, height, in mm.
    pub defaults: (f64, f64, f64),
    /// leg, rail and claie diameters, in mm.
    pub poles: (f64, f64, f64),
}

pub const TEMPLATES: &[Template] = &[
    Template {
        id: "banc",
        name: "Banc",
        category: "Assises",
        summary: "Quatre pieds, deux traverses longues et deux courtes, assise en claies.",
        defaults: (1200.0, 300.0, 420.0),
        poles: (80.0, 60.0, 40.0),
    },
    Template {
        id: "banc_dossier",
        name: "Banc à dossier",
        category: "Assises",
        summary: "Un banc dont les pieds arrière montent pour tenir deux traverses de dossier.",
        defaults: (1200.0, 350.0, 420.0),
        poles: (80.0, 60.0, 40.0),
    },
    Template {
        id: "table",
        name: "Table",
        category: "Tables et plans de travail",
        summary: "Quatre pieds, traverses, plateau en claies.",
        defaults: (1500.0, 800.0, 750.0),
        poles: (80.0, 60.0, 40.0),
    },
    Template {
        id: "table_prepa",
        name: "Table de prépa avec étagère",
        category: "Tables et plans de travail",
        summary: "Une table de travail avec une étagère basse en claies.",
        defaults: (1500.0, 700.0, 850.0),
        poles: (80.0, 60.0, 40.0),
    },
    Template {
        id: "etagere",
        name: "Étagère",
        category: "Rangements",
        summary: "Quatre montants et des niveaux de claies sur traverses (4 niveaux).",
        defaults: (1000.0, 400.0, 1600.0),
        poles: (70.0, 50.0, 40.0),
    },
];

pub fn template(id: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.id == id)
}

/// The dimensions a furniture is made with.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dims {
    pub length: f64,
    pub width: f64,
    pub height: f64,
    pub leg: f64,
    pub rail: f64,
    pub claie: f64,
}

impl Dims {
    pub fn of(t: &Template) -> Dims {
        Dims { length: t.defaults.0, width: t.defaults.1, height: t.defaults.2, leg: t.poles.0, rail: t.poles.1, claie: t.poles.2 }
    }
    /// Sizes a person can build: poles between 20 and 200 mm, a furniture between 20 cm and 5 m.
    pub fn check(&self) -> Result<(), String> {
        for (n, v, lo, hi) in [
            ("length", self.length, 200.0, 5000.0),
            ("width", self.width, 200.0, 5000.0),
            ("height", self.height, 200.0, 3000.0),
            ("leg", self.leg, 20.0, 200.0),
            ("rail", self.rail, 20.0, 200.0),
            ("claie", self.claie, 15.0, 100.0),
        ] {
            if !(lo..=hi).contains(&v) || !v.is_finite() {
                return Err(format!("`{n}` must be between {lo} and {hi} mm"));
            }
        }
        if self.leg < self.rail {
            return Err("the legs must be at least as thick as the rails".into());
        }
        if self.length < 2.0 * self.leg + 50.0 || self.width < 2.0 * self.leg + 50.0 {
            return Err("the furniture is too small for its legs".into());
        }
        if self.height < 3.0 * self.rail + self.claie {
            return Err("the furniture is too low for its rails".into());
        }
        Ok(())
    }
}

/// How many claies fit on a top `span` long, with a gap of half a claie.
fn claie_count(span: f64, d: f64) -> usize {
    (((span + d * 0.5) / (d * 1.5)).floor().max(1.0)) as usize
}

fn pole(role: &'static str, diameter: f64, length: f64, origin: [f64; 3], axis: Axis, joint: Joint) -> Pole {
    Pole { role, diameter, length, origin, axis, joint, receiver: 0.0 }
}

/// A pole that runs between two others and is tenoned into both.
fn between(role: &'static str, diameter: f64, length: f64, origin: [f64; 3], axis: Axis, receiver: f64) -> Pole {
    Pole { role, diameter, length, origin, axis, joint: Joint::TenonMortaise, receiver }
}

/// The table frame all templates share: legs at the four corners, then for each level (z of the
/// top of the rails and whether short rails go in too) rails and claies.
struct Frame {
    d: Dims,
    poles: Vec<Pole>,
}

impl Frame {
    fn new(d: Dims) -> Frame {
        Frame { d, poles: Vec::new() }
    }
    fn a(&self) -> f64 {
        self.d.leg / 2.0
    }
    /// The four legs; the back ones (y = width − a) may be taller.
    fn legs(&mut self, front_len: f64, back_len: f64, back_role: &'static str) {
        let a = self.a();
        let (l, w, dl) = (self.d.length, self.d.width, self.d.leg);
        for (x, y, len, role) in [(a, a, front_len, "pied"), (l - a, a, front_len, "pied"), (a, w - a, back_len, back_role), (l - a, w - a, back_len, back_role)] {
            self.poles.push(pole(role, dl, len, [x, y, 0.0], Axis::Z, Joint::TenonMortaise));
        }
    }
    /// Rails whose top is at `z_top`: two long ones, and two short ones one rail-and-a-bit lower.
    fn rails(&mut self, z_top: f64, short: bool) {
        let a = self.a();
        let (l, w, dr) = (self.d.length, self.d.width, self.d.rail);
        let z = z_top - dr / 2.0;
        let dl = self.d.leg;
        for y in [a, w - a] {
            self.poles.push(between("traverse longue", dr, l - 2.0 * dl, [dl, y, z], Axis::X, dl));
        }
        if short {
            let z = z_top - dr * 1.7;
            for x in [a, l - a] {
                self.poles.push(between("traverse courte", dr, w - 2.0 * dl, [x, dl, z], Axis::Y, dl));
            }
        }
    }
    /// A row of claies lying across the width on rails whose top is at `z_top`.
    fn claies(&mut self, z_top: f64) {
        let (l, w, dc) = (self.d.length, self.d.width, self.d.claie);
        let n = claie_count(l, dc);
        let pitch = if n > 1 { (l - dc) / (n - 1) as f64 } else { 0.0 };
        for i in 0..n {
            let x = if n > 1 { dc / 2.0 + pitch * i as f64 } else { l / 2.0 };
            self.poles.push(pole("claie", dc, w, [x, 0.0, z_top + dc / 2.0], Axis::Y, Joint::Posee));
        }
    }
}

/// The poles of a template with the given dimensions.
pub fn build(id: &str, d: Dims) -> Result<Vec<Pole>, String> {
    d.check()?;
    template(id).ok_or_else(|| format!("unknown furniture `{id}` (scout.meuble.types lists them)"))?;
    let mut f = Frame::new(d);
    let leg_len = d.height - d.claie;
    match id {
        "banc" | "table" => {
            f.legs(leg_len, leg_len, "pied");
            f.rails(leg_len, true);
            f.claies(leg_len);
        }
        "banc_dossier" => {
            let back_h = (d.height * 0.9).min(600.0);
            f.legs(leg_len, leg_len + back_h, "montant de dossier");
            f.rails(leg_len, true);
            f.claies(leg_len);
            // the back rests: two rails along x through the back legs
            let a = f.a();
            for frac in [0.35, 0.8] {
                let z = leg_len + back_h * frac;
                f.poles.push(between("traverse de dossier", d.rail, d.length - 2.0 * d.leg, [d.leg, d.width - a, z], Axis::X, d.leg));
            }
        }
        "table_prepa" => {
            f.legs(leg_len, leg_len, "pied");
            f.rails(leg_len, true);
            f.claies(leg_len);
            let shelf_top = (d.height * 0.3).max(d.rail * 3.0);
            f.rails(shelf_top, false);
            f.claies(shelf_top);
        }
        "etagere" => {
            let levels = 4usize;
            let top = d.height - d.claie;
            f.legs(top, top, "montant");
            for i in 1..=levels {
                let z = top * i as f64 / levels as f64;
                f.rails(z, false);
                f.claies(z);
            }
        }
        _ => return Err(format!("unknown furniture `{id}`")),
    }
    if f.poles.len() > MAX_POLES {
        return Err(format!("{} poles: more than the {MAX_POLES} allowed", f.poles.len()));
    }
    Ok(f.poles)
}

/// The cut list: poles of one role, diameter and length together, in the order they first appear.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CutRow {
    pub role: &'static str,
    pub diameter: f64,
    pub length: f64,
    pub count: usize,
    pub joint: Joint,
    /// Length to cut from the log, tenons included.
    pub cut_length: f64,
}

pub fn cut_list(poles: &[Pole]) -> Vec<CutRow> {
    let mut rows: Vec<CutRow> = Vec::new();
    for p in poles {
        match rows.iter_mut().find(|r| r.role == p.role && r.diameter == p.diameter && r.length == p.length) {
            Some(r) => r.count += 1,
            None => rows.push(CutRow { role: p.role, diameter: p.diameter, length: p.length, count: 1, joint: p.joint, cut_length: p.cut_length() }),
        }
    }
    rows
}

/// Total length of wood in metres.
pub fn wood_metres(poles: &[Pole]) -> f64 {
    poles.iter().map(Pole::cut_length).sum::<f64>() / 1000.0
}

/// The overall size (x, y, z) of a set of poles.
pub fn extent(poles: &[Pole]) -> [f64; 3] {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for p in poles {
        let (s, e) = (p.origin, p.end());
        for i in 0..3 {
            let r = if (p.axis == Axis::X && i != 0) || (p.axis == Axis::Y && i != 1) || (p.axis == Axis::Z && i != 2) { p.diameter / 2.0 } else { 0.0 };
            lo[i] = lo[i].min(s[i].min(e[i]) - r);
            hi[i] = hi[i].max(s[i].max(e[i]) + r);
        }
    }
    if poles.is_empty() {
        return [0.0; 3];
    }
    [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_builds_with_its_defaults() {
        for t in TEMPLATES {
            let poles = build(t.id, Dims::of(t)).unwrap_or_default();
            assert!(!poles.is_empty(), "{}", t.id);
            assert!(poles.len() <= MAX_POLES);
        }
    }

    #[test]
    fn a_bench_has_four_legs_four_rails_and_claies() {
        let poles = build("banc", Dims::of(&TEMPLATES[0])).unwrap_or_default();
        let rows = cut_list(&poles);
        let count = |role: &str| rows.iter().filter(|r| r.role == role).map(|r| r.count).sum::<usize>();
        assert_eq!(count("pied"), 4);
        assert_eq!(count("traverse longue"), 2);
        assert_eq!(count("traverse courte"), 2);
        assert!(count("claie") >= 5);
    }

    #[test]
    fn the_overall_size_matches_the_dimensions() {
        let d = Dims::of(&TEMPLATES[2]);
        let e = extent(&build("table", d).unwrap_or_default());
        assert!((e[0] - d.length).abs() < 1.0 && (e[1] - d.width).abs() < 1.0, "{e:?}");
        assert!((e[2] - d.height).abs() < 1.0, "{e:?}");
    }

    #[test]
    fn hostile_sizes_are_refused() {
        let mut d = Dims::of(&TEMPLATES[0]);
        d.length = f64::NAN;
        assert!(build("banc", d).is_err());
        d.length = 1e12;
        assert!(build("banc", d).is_err());
        let mut d = Dims::of(&TEMPLATES[0]);
        d.claie = 15.0;
        d.length = 5000.0;
        assert!(build("banc", d).is_ok());
        assert!(build("fusée", Dims::of(&TEMPLATES[0])).is_err());
    }

    #[test]
    fn the_rule_of_thirds() {
        let p = pole("pied", 80.0, 400.0, [0.0; 3], Axis::Z, Joint::TenonMortaise);
        assert_eq!(p.tenon(), 27.0);
        let r = between("traverse longue", 60.0, 1040.0, [80.0, 40.0, 0.0], Axis::X, 80.0);
        assert_eq!(r.cut_length(), 1120.0);
    }
}
