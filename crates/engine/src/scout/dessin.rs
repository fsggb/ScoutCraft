//! Dessin technique: the drawing workshop of CathIA, from the poles. A sheet (A3 or A4, landscape) holds
//! views (face, back, right, left, top, bottom, isometric), laid out automatically at a usual
//! scale, the overall dimensions and, if asked, the dimensions of the pieces; a table of the pieces;
//! and the title block (cartouche). A drawing is a list of primitives ([`Prim`]) in sheet millimetres,
//! which the application paints and [`svg`] writes, so what is on screen is what is exported.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use super::pole::{Pole, V3, cut_list, joint_summary, len, sub, wood_metres};

pub type P2 = [f64; 2];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Pole,
    PoleCut,
    Dim,
    Text,
    Title,
    Frame,
    Rule,
    Guide,
    Floor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "p")]
pub enum Prim {
    Line { a: P2, b: P2, w: f64, k: Kind },
    Poly { pts: Vec<P2>, k: Kind },
    Circle { c: P2, r: f64, k: Kind },
    Text { at: P2, s: String, h: f64, anchor: Anchor, rot: bool, k: Kind },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    Face,
    Arriere,
    Droite,
    Gauche,
    Dessus,
    Dessous,
    Iso,
}

impl View {
    pub const ALL: [View; 7] = [View::Face, View::Arriere, View::Droite, View::Gauche, View::Dessus, View::Dessous, View::Iso];
    pub fn label(self) -> &'static str {
        match self {
            View::Face => "Vue de face",
            View::Arriere => "Vue arrière",
            View::Droite => "Vue de droite",
            View::Gauche => "Vue de gauche",
            View::Dessus => "Vue de dessus",
            View::Dessous => "Vue de dessous",
            View::Iso => "Vue isométrique",
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            View::Face => "face",
            View::Arriere => "arriere",
            View::Droite => "droite",
            View::Gauche => "gauche",
            View::Dessus => "dessus",
            View::Dessous => "dessous",
            View::Iso => "iso",
        }
    }
    pub fn parse(s: &str) -> Option<View> {
        View::ALL.into_iter().find(|v| v.id() == s)
    }
    /// A model point on the sheet plane of this view (u right, v up), model mm.
    pub fn project(self, p: V3) -> P2 {
        let [x, y, z] = p;
        match self {
            View::Face => [x, z],
            View::Arriere => [-x, z],
            View::Droite => [y, z],
            View::Gauche => [-y, z],
            View::Dessus => [x, y],
            View::Dessous => [x, -y],
            View::Iso => [(x - y) * 0.866_025, z - (x + y) * 0.5],
        }
    }
    /// The direction towards the viewer, to draw what is far first.
    fn towards(self) -> V3 {
        match self {
            View::Face => [0.0, -1.0, 0.0],
            View::Arriere => [0.0, 1.0, 0.0],
            View::Droite => [1.0, 0.0, 0.0],
            View::Gauche => [-1.0, 0.0, 0.0],
            View::Dessus => [0.0, 0.0, 1.0],
            View::Dessous => [0.0, 0.0, -1.0],
            View::Iso => [1.0, 1.0, 1.0],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Format {
    A3,
    A4,
}

impl Format {
    pub fn size(self) -> (f64, f64) {
        match self {
            Format::A3 => (420.0, 297.0),
            Format::A4 => (297.0, 210.0),
        }
    }
}

/// The title block's fields, as CathIA's.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Cartouche {
    pub titre: String,
    pub societe: String,
    pub dessinateur: String,
    pub date: String,
    pub materiau: String,
    pub reference: String,
    pub indice: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SheetCfg {
    pub format: Format,
    /// 1:n, or 0 for the largest usual scale that fits.
    pub echelle: f64,
    pub vues: Vec<View>,
    pub cotes_ensemble: bool,
    pub cotes_pieces: bool,
    pub tableau: bool,
    pub cartouche: Cartouche,
}

impl SheetCfg {
    pub fn standard(titre: &str, dessinateur: &str) -> SheetCfg {
        SheetCfg {
            format: Format::A3,
            echelle: 0.0,
            vues: vec![View::Face, View::Droite, View::Dessus, View::Iso],
            cotes_ensemble: true,
            cotes_pieces: false,
            tableau: true,
            cartouche: Cartouche { titre: titre.to_string(), dessinateur: dessinateur.to_string(), materiau: "Bois rond".into(), ..Cartouche::default() },
        }
    }
}

pub struct Sheet {
    pub w: f64,
    pub h: f64,
    pub prims: Vec<Prim>,
    /// The scale's denominator.
    pub den: f64,
}

/// Usual drawing scales (the denominator).
pub const ECHELLES: [f64; 12] = [1.0, 2.0, 5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 50.0, 75.0, 100.0, 200.0];

fn mm(v: f64) -> String {
    if (v - v.round()).abs() < 0.05 { format!("{}", v.round() as i64) } else { format!("{v:.1}") }
}

fn text(prims: &mut Vec<Prim>, at: P2, s: &str, h: f64, anchor: Anchor, k: Kind) {
    prims.push(Prim::Text { at, s: s.to_string(), h, anchor, rot: false, k });
}

fn rect(prims: &mut Vec<Prim>, x: f64, y: f64, w: f64, h: f64, k: Kind) {
    prims.push(Prim::Poly { pts: vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]], k });
}

fn hline(prims: &mut Vec<Prim>, a: P2, b: P2, w: f64, k: Kind) {
    prims.push(Prim::Line { a, b, w, k });
}

/// A horizontal dimension under a box and a vertical one at its left, with the value.
fn dim_h(prims: &mut Vec<Prim>, x0: f64, x1: f64, y: f64, s: &str) {
    hline(prims, [x0, y], [x1, y], 0.25, Kind::Dim);
    for x in [x0, x1] {
        hline(prims, [x, y - 2.0], [x, y + 2.0], 0.25, Kind::Dim);
        hline(prims, [x, y], [x + if x == x0 { 2.5 } else { -2.5 }, y - 0.9], 0.25, Kind::Dim);
        hline(prims, [x, y], [x + if x == x0 { 2.5 } else { -2.5 }, y + 0.9], 0.25, Kind::Dim);
    }
    text(prims, [(x0 + x1) / 2.0, y + 4.2], s, 3.2, Anchor::Middle, Kind::Dim);
}

fn dim_v(prims: &mut Vec<Prim>, y0: f64, y1: f64, x: f64, s: &str) {
    hline(prims, [x, y0], [x, y1], 0.25, Kind::Dim);
    for y in [y0, y1] {
        hline(prims, [x - 2.0, y], [x + 2.0, y], 0.25, Kind::Dim);
        hline(prims, [x, y], [x - 0.9, y + if y == y0 { 2.5 } else { -2.5 }], 0.25, Kind::Dim);
        hline(prims, [x, y], [x + 0.9, y + if y == y0 { 2.5 } else { -2.5 }], 0.25, Kind::Dim);
    }
    prims.push(Prim::Text { at: [x - 1.8, (y0 + y1) / 2.0], s: s.to_string(), h: 3.2, anchor: Anchor::Middle, rot: true, k: Kind::Dim });
}

/// Model extent of the poles in a view: (lo, hi) in view mm.
fn extent2(view: View, poles: &[Pole]) -> (P2, P2) {
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for p in poles.iter().filter(|p| p.role != "ficelle") {
        let r = p.radius();
        for q in [p.p_start(), p.end()] {
            let u = view.project(q);
            for i in 0..2 {
                lo[i] = lo[i].min(u[i] - r);
                hi[i] = hi[i].max(u[i] + r);
            }
        }
    }
    if poles.is_empty() { ([0.0; 2], [1.0; 2]) } else { (lo, hi) }
}

/// One view of the poles, its box's top-left at `(ox, oy)` on the sheet.
fn draw_view(prims: &mut Vec<Prim>, view: View, poles: &[Pole], [ox, oy]: P2, scale: f64, (lo, hi): (P2, P2)) {
    let sx = |u: f64| ox + (u - lo[0]) * scale;
    let sy = |v: f64| oy + (hi[1] - v) * scale;
    let tw = view.towards();
    let mut order: Vec<&Pole> = poles.iter().filter(|p| p.role != "ficelle").collect();
    let depth = |p: &Pole| {
        let (a, b) = (p.p_start(), p.end());
        let c = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, (a[2] + b[2]) / 2.0];
        c[0] * tw[0] + c[1] * tw[1] + c[2] * tw[2]
    };
    order.sort_by(|a, b| depth(a).partial_cmp(&depth(b)).unwrap_or(std::cmp::Ordering::Equal));
    for p in order {
        let (a, b) = (view.project(p.p_start()), view.project(p.end()));
        let r = p.radius();
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l = (dx * dx + dy * dy).sqrt();
        let k = if p.role == "brêlage" { Kind::PoleCut } else { Kind::Pole };
        if l * scale < 0.5 {
            prims.push(Prim::Circle { c: [sx(a[0]), sy(a[1])], r: (r * scale).max(0.3), k });
        } else {
            let (nx, ny) = (-dy / l * r, dx / l * r);
            let pts = [[a[0] + nx, a[1] + ny], [b[0] + nx, b[1] + ny], [b[0] - nx, b[1] - ny], [a[0] - nx, a[1] - ny]].iter().map(|q| [sx(q[0]), sy(q[1])]).collect();
            prims.push(Prim::Poly { pts, k });
        }
    }
    // the net, as one light line per rope
    for p in poles.iter().filter(|p| p.role == "ficelle") {
        let (a, b) = (view.project(p.p_start()), view.project(p.end()));
        hline(prims, [sx(a[0]), sy(a[1])], [sx(b[0]), sy(b[1])], 0.15, Kind::Guide);
    }
}

fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// The title block, bottom right: three rows of fields, the scale and the sheet.
fn cartouche(prims: &mut Vec<Prim>, (w, h): (f64, f64), c: &Cartouche, den: f64, format: Format, feuille: (usize, usize)) {
    let (bw, bh) = (150.0, 34.0);
    let (x, y) = (w - 8.0 - bw, h - 8.0 - bh);
    rect(prims, x, y, bw, bh, Kind::Frame);
    hline(prims, [x, y + 12.0], [x + bw, y + 12.0], 0.3, Kind::Rule);
    hline(prims, [x, y + 23.0], [x + bw, y + 23.0], 0.3, Kind::Rule);
    hline(prims, [x + 90.0, y + 12.0], [x + 90.0, y + bh], 0.3, Kind::Rule);
    text(prims, [x + 3.0, y + 8.0], &c.titre, 5.0, Anchor::Start, Kind::Title);
    text(prims, [x + 3.0, y + 19.0], &format!("Dessiné par : {}", c.dessinateur), 3.2, Anchor::Start, Kind::Text);
    text(prims, [x + 3.0, y + 30.0], &format!("{} {}", c.societe, c.date), 3.2, Anchor::Start, Kind::Text);
    text(prims, [x + 93.0, y + 19.0], &format!("Échelle 1:{}   {:?}", mm(den), format), 3.2, Anchor::Start, Kind::Text);
    text(prims, [x + 93.0, y + 30.0], &format!("Feuille {}/{}   Ind. {}", feuille.0, feuille.1, if c.indice.is_empty() { "A" } else { &c.indice }), 3.2, Anchor::Start, Kind::Text);
    if !c.materiau.is_empty() || !c.reference.is_empty() {
        text(prims, [x + 93.0, y + 8.0], &format!("{} {}", c.materiau, c.reference), 3.0, Anchor::Start, Kind::Text);
    }
}

fn frame(prims: &mut Vec<Prim>, (w, h): (f64, f64)) {
    rect(prims, 5.0, 5.0, w - 10.0, h - 10.0, Kind::Frame);
}

/// The table of the pieces, top right of the sheet.
fn tableau(prims: &mut Vec<Prim>, poles: &[Pole], x: f64, y: f64, max_rows: usize) -> f64 {
    text(prims, [x, y], "Nomenclature", 4.5, Anchor::Start, Kind::Title);
    let cols = [0.0, 8.0, 18.0, 62.0];
    let mut yy = y + 6.0;
    for (i, h) in ["N°", "Qté", "Désignation", "Dimensions"].iter().enumerate() {
        text(prims, [x + cols[i], yy], h, 3.0, Anchor::Start, Kind::Title);
    }
    hline(prims, [x, yy + 1.5], [x + 100.0, yy + 1.5], 0.25, Kind::Rule);
    yy += 5.5;
    for (i, r) in cut_list(poles).iter().take(max_rows).enumerate() {
        let dim = if r.width > 0.0 { format!("{}×{}×{}", mm(r.diameter), mm(r.width), mm(r.length)) } else { format!("Ø{} × {}", mm(r.diameter), mm(r.length)) };
        for (j, cell) in [format!("{}", i + 1), r.count.to_string(), r.role.clone(), dim].iter().enumerate() {
            text(prims, [x + cols[j], yy], cell, 3.0, Anchor::Start, Kind::Text);
        }
        yy += 4.6;
    }
    yy
}

/// The drawing sheet of a furniture (or of any set of poles).
pub fn sheet(poles: &[Pole], cfg: &SheetCfg, feuille: (usize, usize)) -> Sheet {
    let (w, h) = cfg.format.size();
    let mut prims = Vec::new();
    frame(&mut prims, (w, h));
    let table_w = if cfg.tableau && cfg.format == Format::A3 { 112.0 } else { 0.0 };
    let (margin, gap, dim_room) = (12.0, 22.0, 12.0);
    let area_w = w - 2.0 * margin - table_w - dim_room;
    let area_h = h - 2.0 * margin - 38.0 - 8.0;
    let views: Vec<View> = if cfg.vues.is_empty() { vec![View::Face] } else { cfg.vues.clone() };
    let ext: Vec<(P2, P2)> = views.iter().map(|v| extent2(*v, poles)).collect();
    let size = |i: usize| (ext[i].1[0] - ext[i].0[0], ext[i].1[1] - ext[i].0[1]);
    let cols = if views.len() == 1 { 1 } else { 2 };
    let rows = views.len().div_ceil(cols);
    let col_w: Vec<f64> = (0..cols).map(|c| (0..views.len()).filter(|i| i % cols == c).map(|i| size(i).0).fold(0.0, f64::max)).collect();
    let row_h: Vec<f64> = (0..rows).map(|r| (0..views.len()).filter(|i| i / cols == r).map(|i| size(i).1).fold(0.0, f64::max)).collect();
    let (sum_w, sum_h): (f64, f64) = (col_w.iter().sum(), row_h.iter().sum());
    let fits = |den: f64| sum_w / den + gap * (cols as f64 - 1.0) <= area_w && sum_h / den + gap * (rows as f64 - 1.0) <= area_h;
    let den = if cfg.echelle > 0.0 { cfg.echelle } else { ECHELLES.iter().copied().find(|d| fits(*d)).unwrap_or(500.0) };
    let scale = 1.0 / den;
    // top-left of each cell
    let mut y = margin + 6.0;
    let mut cell_y = vec![0.0; rows];
    for r in 0..rows {
        cell_y[r] = y;
        y += row_h[r] * scale + gap;
    }
    let mut x = margin + dim_room;
    let mut cell_x = vec![0.0; cols];
    for c in 0..cols {
        cell_x[c] = x;
        x += col_w[c] * scale + gap;
    }
    for (i, v) in views.iter().enumerate() {
        let (c, r) = (i % cols, i / cols);
        let (ox, oy) = (cell_x[c], cell_y[r]);
        let (sw, sh) = (size(i).0 * scale, size(i).1 * scale);
        text(&mut prims, [ox, oy - 2.5], v.label(), 3.6, Anchor::Start, Kind::Title);
        draw_view(&mut prims, *v, poles, [ox, oy + (row_h[r] - size(i).1) * scale], scale, ext[i]);
        let oy2 = oy + (row_h[r] - size(i).1) * scale;
        if cfg.cotes_ensemble && *v != View::Iso {
            dim_h(&mut prims, ox, ox + sw, oy2 + sh + 5.0, &mm(size(i).0));
            dim_v(&mut prims, oy2, oy2 + sh, ox - 5.0, &mm(size(i).1));
        }
        if cfg.cotes_pieces && matches!(v, View::Face | View::Dessus) {
            // the diameter of each kind of piece, written beside its first appearance
            let mut seen: Vec<&str> = Vec::new();
            for p in poles.iter().filter(|p| p.role != "ficelle" && p.role != "brêlage" && p.role != "cheville") {
                if seen.contains(&p.role.as_str()) {
                    continue;
                }
                seen.push(&p.role);
                let m = v.project(sub_mid(p));
                let (px, py) = (ox + (m[0] - ext[i].0[0]) * scale, oy2 + (ext[i].1[1] - m[1]) * scale);
                text(&mut prims, [px, py], &format!("Ø{}", mm(p.diameter)), 2.8, Anchor::Middle, Kind::Dim);
            }
        }
    }
    if table_w > 0.0 {
        let ty = margin + 6.0;
        let end = tableau(&mut prims, poles, w - margin - table_w + 4.0, ty, 28);
        let mut yy = end + 5.0;
        text(&mut prims, [w - margin - table_w + 4.0, yy], "Assemblages", 4.5, Anchor::Start, Kind::Title);
        yy += 6.0;
        for (k, c) in joint_summary(poles) {
            text(&mut prims, [w - margin - table_w + 4.0, yy], &format!("{c} × {k}"), 3.0, Anchor::Start, Kind::Text);
            yy += 4.4;
        }
        yy += 3.0;
        text(&mut prims, [w - margin - table_w + 4.0, yy], &format!("Bois à débiter : {:.1} m", wood_metres(poles)), 3.2, Anchor::Start, Kind::Title);
    }
    cartouche(&mut prims, (w, h), &cfg.cartouche, den, cfg.format, feuille);
    Sheet { w, h, prims, den }
}

fn sub_mid(p: &Pole) -> V3 {
    let (a, b) = (p.p_start(), p.end());
    [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, (a[2] + b[2]) / 2.0]
}

/// One furniture of a patrol corner on the overview sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub name: String,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub length: f64,
    pub width: f64,
    pub rotation: f64,
    pub wood_m: f64,
}

/// The footprint (x0, y0, x1, y1) of a placed furniture after its quarter turn about its corner.
pub fn footprint(p: &Placed) -> (f64, f64, f64, f64) {
    let q = (p.rotation / 90.0).round() as i64;
    let (l, w) = (p.length, p.width);
    match q.rem_euclid(4) {
        0 => (p.x, p.y, p.x + l, p.y + w),
        1 => (p.x - w, p.y, p.x, p.y + l),
        2 => (p.x - l, p.y - w, p.x, p.y),
        _ => (p.x, p.y - l, p.x + w, p.y),
    }
}

/// The overview sheet of a patrol corner: the ground plan with each furniture numbered, its overall
/// dimensions and the list. Each furniture has its own sheet.
pub fn sheet_coin(items: &[Placed], cfg: &SheetCfg, feuille: (usize, usize)) -> Sheet {
    let (w, h) = cfg.format.size();
    let mut prims = Vec::new();
    frame(&mut prims, (w, h));
    let boxes: Vec<_> = items.iter().map(footprint).collect();
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for b in &boxes {
        x0 = x0.min(b.0);
        y0 = y0.min(b.1);
        x1 = x1.max(b.2);
        y1 = y1.max(b.3);
    }
    if items.is_empty() {
        (x0, y0, x1, y1) = (0.0, 0.0, 1000.0, 1000.0);
    }
    let (cw, ch) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
    let table_w = if cfg.format == Format::A3 { 112.0 } else { 0.0 };
    let (margin, dim_room) = (12.0, 14.0);
    let (area_w, area_h) = (w - 2.0 * margin - table_w - dim_room, h - 2.0 * margin - 38.0 - 14.0);
    let den = if cfg.echelle > 0.0 { cfg.echelle } else { ECHELLES.iter().copied().find(|d| cw / d <= area_w && ch / d <= area_h).unwrap_or(500.0) };
    let s = 1.0 / den;
    let (ox, oy) = (margin + dim_room, margin + 8.0);
    text(&mut prims, [ox, oy - 3.0], "Vue d'ensemble du coin (de dessus)", 3.8, Anchor::Start, Kind::Title);
    for (i, b) in boxes.iter().enumerate() {
        let (bx, by, bw, bh) = (ox + (b.0 - x0) * s, oy + (y1 - b.3) * s, (b.2 - b.0) * s, (b.3 - b.1) * s);
        rect(&mut prims, bx, by, bw, bh, Kind::Floor);
        text(&mut prims, [bx + bw / 2.0, by + bh / 2.0 + 1.5], &format!("{}", i + 1), 5.0, Anchor::Middle, Kind::Title);
    }
    dim_h(&mut prims, ox, ox + cw * s, oy + ch * s + 5.0, &mm(cw));
    dim_v(&mut prims, oy, oy + ch * s, ox - 5.0, &mm(ch));
    let x = w - margin - table_w + 4.0;
    let mut y = margin + 6.0;
    text(&mut prims, [x, y], "Meubles du coin", 4.5, Anchor::Start, Kind::Title);
    y += 6.0;
    for (i, p) in items.iter().enumerate() {
        let line = format!("{}. {} ({}) : {} × {} mm, {:.1} m de bois", i + 1, p.name, p.kind, mm(p.length), mm(p.width), p.wood_m);
        for (j, l) in wrap(&line, 56).iter().enumerate() {
            text(&mut prims, [x, y], &if j == 0 { l.clone() } else { format!("   {l}") }, 3.0, Anchor::Start, Kind::Text);
            y += 4.2;
        }
    }
    y += 3.0;
    let total: f64 = items.iter().map(|p| p.wood_m).sum();
    text(&mut prims, [x, y], &format!("Total : {} meubles, {:.1} m de bois", items.len(), total), 3.4, Anchor::Start, Kind::Title);
    text(&mut prims, [x, y + 6.0], "Un plan détaillé est établi pour chaque meuble.", 3.0, Anchor::Start, Kind::Text);
    cartouche(&mut prims, (w, h), &cfg.cartouche, den, cfg.format, feuille);
    Sheet { w, h, prims, den }
}

/// An isometric picture of the poles alone (no frame), for the HTML page: returns an SVG.
pub fn iso_only(poles: &[Pole]) -> String {
    let ext = extent2(View::Iso, poles);
    let (lo, hi) = ext;
    let mut prims = Vec::new();
    draw_view(&mut prims, View::Iso, poles, [0.0, 0.0], 1.0, ext);
    let (w, hh) = (hi[0] - lo[0], hi[1] - lo[1]);
    let mut o = String::new();
    let _ = writeln!(o, "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"-20 -20 {:.1} {:.1}\" class=\"iso\">", w + 40.0, hh + 40.0);
    for p in &prims {
        write_prim(&mut o, p, true);
    }
    o.push_str("</svg>\n");
    o
}

fn esc(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            c if c.is_control() => " ".to_string(),
            c => c.to_string(),
        })
        .collect()
}

fn class(k: Kind) -> &'static str {
    match k {
        Kind::Pole => "p",
        Kind::PoleCut => "pc",
        Kind::Dim => "dm",
        Kind::Text => "tx",
        Kind::Title => "ti",
        Kind::Frame => "fr",
        Kind::Rule => "ru",
        Kind::Guide => "gd",
        Kind::Floor => "fl",
    }
}

fn write_prim(o: &mut String, p: &Prim, bare: bool) {
    let _ = bare;
    match p {
        Prim::Line { a, b, w, k } => {
            let _ = writeln!(o, "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" class=\"{}\" stroke-width=\"{:.2}\"/>", a[0], a[1], b[0], b[1], class(*k), w);
        }
        Prim::Poly { pts, k } => {
            let list: Vec<String> = pts.iter().map(|q| format!("{:.2},{:.2}", q[0], q[1])).collect();
            let _ = writeln!(o, "<polygon points=\"{}\" class=\"{}\"/>", list.join(" "), class(*k));
        }
        Prim::Circle { c, r, k } => {
            let _ = writeln!(o, "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" class=\"{}\"/>", c[0], c[1], r, class(*k));
        }
        Prim::Text { at, s, h, anchor, rot, k } => {
            let a = match anchor {
                Anchor::Start => "start",
                Anchor::Middle => "middle",
                Anchor::End => "end",
            };
            let tr = if *rot { format!(" transform=\"rotate(-90 {:.2} {:.2})\"", at[0], at[1]) } else { String::new() };
            let _ = writeln!(o, "<text x=\"{:.2}\" y=\"{:.2}\" font-size=\"{:.2}\" text-anchor=\"{a}\" class=\"{}\"{tr}>{}</text>", at[0], at[1], h, class(*k), esc(s));
        }
    }
}

const STYLE: &str = "<style>\
.p{fill:#e9d3a8;stroke:#4a2f12;stroke-width:.3}.pc{fill:#c9a46a;stroke:#4a2f12;stroke-width:.2}\
.dm{stroke:#c26a00;fill:none}text.dm{fill:#c26a00;stroke:none;font-family:sans-serif}\
text{font-family:sans-serif}text.tx{fill:#1d2330}text.ti{fill:#000761;font-weight:bold}\
.fr{fill:none;stroke:#1d2330;stroke-width:.5}.ru{stroke:#1d2330}.gd{stroke:#6f879c}.fl{fill:#e9d3a8;stroke:#4a2f12;stroke-width:.4}\
</style>";

/// The sheet as SVG text.
pub fn svg(s: &Sheet) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}mm\" height=\"{}mm\">{STYLE}", s.w, s.h, s.w, s.h);
    let _ = writeln!(o, "<rect width=\"{}\" height=\"{}\" fill=\"#fff\"/>", s.w, s.h);
    for p in &s.prims {
        write_prim(&mut o, p, false);
    }
    o.push_str("</svg>\n");
    o
}

pub fn pole_len(p: &Pole) -> f64 {
    len(sub(p.end(), p.p_start()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scout::models::{Params, build, model};

    fn poles_of(id: &str) -> Vec<Pole> {
        model(id).map(|m| build(id, &Params::of(m)).unwrap_or_default()).unwrap_or_default()
    }

    #[test]
    fn a_sheet_has_its_views_dimensions_table_and_title_block() {
        let poles = poles_of("banc");
        let cfg = SheetCfg::standard("Banc <1> & co", "Lou");
        let s = sheet(&poles, &cfg, (1, 1));
        let svg = svg(&s);
        assert!(svg.starts_with("<svg") && svg.trim_end().ends_with("</svg>"));
        for needle in ["Vue de face", "Vue de dessus", "Vue isométrique", "Nomenclature", "Assemblages", "Dessiné par : Lou", "Banc &lt;1&gt; &amp; co", "1500"] {
            assert!(svg.contains(needle), "{needle}");
        }
        assert!(ECHELLES.contains(&s.den));
    }

    #[test]
    fn every_view_draws_every_pole_and_a_chosen_scale_is_kept() {
        let poles = poles_of("table");
        for v in View::ALL {
            let mut cfg = SheetCfg::standard("T", "L");
            cfg.vues = vec![v];
            cfg.echelle = 20.0;
            let s = sheet(&poles, &cfg, (1, 1));
            assert_eq!(s.den, 20.0);
            let n = s.prims.iter().filter(|p| matches!(p, Prim::Poly { k: Kind::Pole, .. } | Prim::Circle { k: Kind::Pole, .. })).count();
            assert!(n >= poles.iter().filter(|p| p.role != "ficelle").count(), "{v:?}: {n}");
        }
    }

    #[test]
    fn an_a4_sheet_has_no_table_and_dimensions_can_be_dropped() {
        let poles = poles_of("etagere");
        let mut cfg = SheetCfg::standard("T", "L");
        cfg.format = Format::A4;
        cfg.cotes_ensemble = false;
        let s = sheet(&poles, &cfg, (1, 2));
        let text = svg(&s);
        assert!(!text.contains("Nomenclature") && !text.contains("class=\"dm\""));
        assert!(text.contains("Feuille 1/2"));
    }

    #[test]
    fn the_overview_numbers_the_furniture_and_quarter_turns_swap_the_footprint() {
        let mut p = Placed { name: "t".into(), kind: "table".into(), x: 0.0, y: 0.0, length: 1500.0, width: 800.0, rotation: 0.0, wood_m: 1.0 };
        assert_eq!(footprint(&p), (0.0, 0.0, 1500.0, 800.0));
        p.rotation = 90.0;
        assert_eq!(footprint(&p), (-800.0, 0.0, 0.0, 1500.0));
        p.rotation = 270.0;
        assert_eq!(footprint(&p), (0.0, -1500.0, 800.0, 0.0));
        let s = sheet_coin(&[p], &SheetCfg::standard("Coin", "L"), (1, 3));
        assert!(svg(&s).contains("Meubles du coin"));
        assert!(svg(&sheet_coin(&[], &SheetCfg::standard("Coin", "L"), (1, 1))).contains("Vue d'ensemble"));
    }

    #[test]
    fn the_isometric_picture_is_a_bare_svg() {
        let s = iso_only(&poles_of("fauteuil"));
        assert!(s.starts_with("<svg") && !s.contains("<script"));
    }
}
