//! The plans: one sheet per furniture (three views, overall dimensions, cut list, assemblies
//! and building order) and one overview sheet for a whole patrol corner (top view of every
//! furniture with its footprint). Both are SVG text, A3 landscape (420 × 297 mm), drawn from the
//! poles alone so a plan never depends on the viewport.

use std::fmt::Write as _;

use super::templates::{Axis, CutRow, Pole, cut_list, extent, wood_metres};

const SHEET_W: f64 = 420.0;
const SHEET_H: f64 = 297.0;

/// Text safe inside SVG.
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

/// A millimetre length as the plans write it.
fn mm(v: f64) -> String {
    if (v - v.round()).abs() < 0.05 { format!("{}", v.round() as i64) } else { format!("{v:.1}") }
}

/// A usual drawing scale denominator at or above `d`.
fn nice_scale(d: f64) -> f64 {
    [1.0, 2.0, 5.0, 10.0, 20.0, 25.0, 50.0, 100.0, 200.0, 500.0].into_iter().find(|n| *n >= d).unwrap_or(1000.0)
}

fn idx(a: Axis) -> usize {
    match a {
        Axis::X => 0,
        Axis::Y => 1,
        Axis::Z => 2,
    }
}

/// One orthographic view: the poles projected on the axes `(iu, iv)`, v up, top-left of its box at
/// `(ox, oy)` in sheet millimetres, `scale` sheet mm per mm.
fn view(out: &mut String, poles: &[Pole], (iu, iv): (usize, usize), (ox, oy): (f64, f64), scale: f64, size: (f64, f64), lo: [f64; 3], title: &str) {
    let _ = writeln!(out, "<g class=\"view\"><text x=\"{:.2}\" y=\"{:.2}\" class=\"t\">{}</text>", ox, oy - 3.0, esc(title));
    let px = |u: f64| ox + (u - lo[iu]) * scale;
    let py = |v: f64| oy + (size.1 - (v - lo[iv])) * scale;
    for p in poles {
        let (s, e, r) = (p.origin, p.end(), p.diameter / 2.0);
        let a = idx(p.axis);
        if a == iu {
            let (x0, x1) = (s[iu].min(e[iu]), s[iu].max(e[iu]));
            let (y0, y1) = (s[iv] - r, s[iv] + r);
            let _ = writeln!(out, "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" class=\"p\"/>", px(x0), py(y1), (x1 - x0) * scale, (y1 - y0) * scale);
        } else if a == iv {
            let (y0, y1) = (s[iv].min(e[iv]), s[iv].max(e[iv]));
            let (x0, x1) = (s[iu] - r, s[iu] + r);
            let _ = writeln!(out, "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" class=\"p\"/>", px(x0), py(y1), (x1 - x0) * scale, (y1 - y0) * scale);
        } else {
            let _ = writeln!(out, "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" class=\"p\"/>", px(s[iu]), py(s[iv]), r * scale);
        }
    }
    out.push_str("</g>\n");
}

/// A horizontal dimension line under a box and a vertical one at its left, with the value.
fn dim_h(out: &mut String, x0: f64, x1: f64, y: f64, text: &str) {
    let _ = writeln!(
        out,
        "<g class=\"dim\"><line x1=\"{x0:.2}\" y1=\"{y:.2}\" x2=\"{x1:.2}\" y2=\"{y:.2}\"/><line x1=\"{x0:.2}\" y1=\"{:.2}\" x2=\"{x0:.2}\" y2=\"{:.2}\"/><line x1=\"{x1:.2}\" y1=\"{:.2}\" x2=\"{x1:.2}\" y2=\"{:.2}\"/><text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\">{}</text></g>",
        y - 2.0,
        y + 2.0,
        y - 2.0,
        y + 2.0,
        (x0 + x1) / 2.0,
        y + 5.0,
        esc(text)
    );
}

fn dim_v(out: &mut String, y0: f64, y1: f64, x: f64, text: &str) {
    let _ = writeln!(
        out,
        "<g class=\"dim\"><line x1=\"{x:.2}\" y1=\"{y0:.2}\" x2=\"{x:.2}\" y2=\"{y1:.2}\"/><line x1=\"{:.2}\" y1=\"{y0:.2}\" x2=\"{:.2}\" y2=\"{y0:.2}\"/><line x1=\"{:.2}\" y1=\"{y1:.2}\" x2=\"{:.2}\" y2=\"{y1:.2}\"/><text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" transform=\"rotate(-90 {:.2} {:.2})\">{}</text></g>",
        x - 2.0,
        x + 2.0,
        x - 2.0,
        x + 2.0,
        x - 3.0,
        (y0 + y1) / 2.0,
        x - 3.0,
        (y0 + y1) / 2.0,
        esc(text)
    );
}

const STYLE: &str = "<style>\
.p{fill:#e9d3a8;stroke:#4a2f12;stroke-width:.35}\
.dim line{stroke:#c26a00;stroke-width:.25}.dim text{fill:#c26a00;font:3px sans-serif}\
.t{font:bold 4px sans-serif;fill:#000761}.b{font:3.2px sans-serif;fill:#1d2330}.h{font:bold 3.2px sans-serif;fill:#000761}\
.frame{fill:none;stroke:#1d2330;stroke-width:.5}.rule{stroke:#1d2330;stroke-width:.2}\
</style>";

/// The building order of a furniture, from its poles: legs first, then the rails, then the top.
pub fn steps(poles: &[Pole]) -> Vec<String> {
    let rows = cut_list(poles);
    let n = |role: &str| rows.iter().filter(|r| r.role == role).map(|r| r.count).sum::<usize>();
    let leg_role = rows.iter().map(|r| r.role).find(|r| *r == "pied" || *r == "montant" || *r == "montant de dossier").unwrap_or("pied");
    let legs = n("pied") + n("montant") + n("montant de dossier");
    let mut v = vec![format!("Débiter toutes les pièces d'après la liste de débit ({} pièces, {} m de bois).", poles.len(), format!("{:.1}", wood_metres(poles)))];
    v.push(format!("Tracer et percer les mortaises dans les {legs} {leg_role}s, à la règle du tiers (tenon = un tiers du diamètre)."));
    v.push("Façonner les tenons aux deux bouts des traverses.".into());
    if n("traverse courte") > 0 {
        v.push("Assembler les deux petits côtés : un pied, une traverse courte, un pied, et cheviller.".into());
    }
    if n("traverse longue") > 0 {
        v.push("Relier les deux petits côtés par les traverses longues, cheviller, vérifier l'équerrage aux diagonales.".into());
    }
    if n("traverse de dossier") > 0 {
        v.push("Poser les traverses de dossier dans les montants arrière.".into());
    }
    if n("claie") > 0 {
        v.push("Poser les claies sur les traverses, les régler à écartement régulier et les brêler.".into());
    }
    v
}

fn table(out: &mut String, rows: &[CutRow], x: f64, y: f64) -> f64 {
    let _ = writeln!(out, "<text x=\"{x:.2}\" y=\"{y:.2}\" class=\"t\">Liste de débit</text>");
    let mut yy = y + 6.0;
    for (i, h) in ["Qté", "Pièce", "Ø mm", "Visible", "À débiter"].iter().enumerate() {
        let cx = x + [0.0, 9.0, 50.0, 64.0, 80.0][i];
        let _ = writeln!(out, "<text x=\"{cx:.2}\" y=\"{yy:.2}\" class=\"h\">{h}</text>");
    }
    let _ = writeln!(out, "<line x1=\"{x:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" class=\"rule\"/>", yy + 1.5, x + 100.0, yy + 1.5);
    yy += 6.0;
    for r in rows {
        for (i, cell) in [r.count.to_string(), r.role.to_string(), mm(r.diameter), mm(r.length), mm(r.cut_length)].iter().enumerate() {
            let cx = x + [0.0, 9.0, 50.0, 64.0, 80.0][i];
            let _ = writeln!(out, "<text x=\"{cx:.2}\" y=\"{yy:.2}\" class=\"b\">{}</text>", esc(cell));
        }
        yy += 5.0;
    }
    yy
}

/// The plan sheet of one furniture.
pub fn meuble_svg(title: &str, kind: &str, author: &str, poles: &[Pole]) -> String {
    let e = extent(poles);
    let lo = {
        let mut lo = [f64::MAX; 3];
        for p in poles {
            let (s, en) = (p.origin, p.end());
            for i in 0..3 {
                let r = if idx(p.axis) == i { 0.0 } else { p.diameter / 2.0 };
                lo[i] = lo[i].min(s[i].min(en[i]) - r);
            }
        }
        if poles.is_empty() { [0.0; 3] } else { lo }
    };
    let (l, w, h) = (e[0].max(1.0), e[1].max(1.0), e[2].max(1.0));
    // Face (x,z) over top (x,y) on the left, side (y,z) at their right.
    let gap = 22.0;
    let scale_den = nice_scale(((l + w) / (250.0 - gap)).max((h + w) / (210.0 - gap)));
    let scale = 1.0 / scale_den;
    let (fx, fy) = (22.0, 28.0);
    let (sx, sy) = (fx + l * scale + gap, fy);
    let (tx, ty) = (fx, fy + h * scale + gap);
    let mut o = String::new();
    let _ = writeln!(o, "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {SHEET_W} {SHEET_H}\" width=\"{SHEET_W}mm\" height=\"{SHEET_H}mm\">{STYLE}");
    let _ = writeln!(o, "<rect width=\"{SHEET_W}\" height=\"{SHEET_H}\" fill=\"#fff\"/><rect x=\"5\" y=\"5\" width=\"{}\" height=\"{}\" class=\"frame\"/>", SHEET_W - 10.0, SHEET_H - 10.0);
    view(&mut o, poles, (0, 2), (fx, fy), scale, (l, h), lo, "Vue de face");
    view(&mut o, poles, (1, 2), (sx, sy), scale, (w, h), lo, "Vue de côté");
    view(&mut o, poles, (0, 1), (tx, ty), scale, (l, w), lo, "Vue de dessus");
    dim_h(&mut o, fx, fx + l * scale, fy + h * scale + 4.0, &mm(l));
    dim_v(&mut o, fy, fy + h * scale, fx - 4.0, &mm(h));
    dim_h(&mut o, sx, sx + w * scale, sy + h * scale + 4.0, &mm(w));
    let x = 285.0;
    let mut y = table(&mut o, &cut_list(poles), x, 25.0) + 6.0;
    let _ = writeln!(o, "<text x=\"{x}\" y=\"{y:.2}\" class=\"t\">Assemblages</text>");
    y += 6.0;
    let rows = cut_list(poles);
    for r in rows.iter().filter(|r| r.joint == super::templates::Joint::TenonMortaise) {
        let tenon = (r.diameter / 3.0).ceil();
        let _ = writeln!(o, "<text x=\"{x}\" y=\"{y:.2}\" class=\"b\">{}</text>", esc(&format!("{} × {} : tenon Ø {} mm", r.count, r.role, mm(tenon))));
        y += 5.0;
    }
    if poles.iter().any(|p| p.joint == super::templates::Joint::Posee) {
        let _ = writeln!(o, "<text x=\"{x}\" y=\"{y:.2}\" class=\"b\">claies : posées sur les traverses et brêlées</text>");
        y += 5.0;
    }
    y += 4.0;
    let _ = writeln!(o, "<text x=\"{x}\" y=\"{y:.2}\" class=\"t\">Montage</text>");
    y += 6.0;
    for (i, s) in steps(poles).iter().enumerate() {
        for (j, line) in wrap(s, 62).iter().enumerate() {
            let _ = writeln!(o, "<text x=\"{x}\" y=\"{y:.2}\" class=\"b\">{}</text>", esc(&if j == 0 { format!("{}. {line}", i + 1) } else { format!("   {line}") }));
            y += 4.2;
        }
    }
    // cartouche
    let _ = writeln!(o, "<rect x=\"230\" y=\"262\" width=\"185\" height=\"30\" class=\"frame\"/>");
    let _ = writeln!(o, "<text x=\"234\" y=\"270\" class=\"t\">{}</text>", esc(title));
    let _ = writeln!(o, "<text x=\"234\" y=\"276\" class=\"b\">{}</text>", esc(&format!("Plan de meuble : {kind}   Échelle 1:{}   Format A3", mm(scale_den))));
    let _ = writeln!(o, "<text x=\"234\" y=\"282\" class=\"b\">{}</text>", esc(&format!("Encombrement {} × {} × {} mm   {} m de bois à débiter", mm(l), mm(w), mm(h), mm(wood_metres(poles)))));
    let _ = writeln!(o, "<text x=\"234\" y=\"288\" class=\"b\">{}</text>", esc(&format!("Dessiné par : {author}   SoLeadWorks")));
    o.push_str("</svg>\n");
    o
}

/// An isometric picture of the poles (no scripts: poles are thick lines, far ones first).
pub fn iso_svg(poles: &[Pole]) -> String {
    let proj = |p: [f64; 3]| ((p[0] - p[1]) * 0.866_025, (p[0] + p[1]) * 0.5 - p[2]);
    let mut order: Vec<&Pole> = poles.iter().collect();
    let depth = |p: &Pole| {
        let (s, e) = (p.origin, p.end());
        (s[0] + e[0] + s[1] + e[1]) / 2.0 + (s[2] + e[2]) * 0.5
    };
    order.sort_by(|a, b| depth(a).partial_cmp(&depth(b)).unwrap_or(std::cmp::Ordering::Equal));
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in poles {
        for q in [p.origin, p.end()] {
            let (x, y) = proj(q);
            let r = p.diameter / 2.0;
            (x0, y0, x1, y1) = (x0.min(x - r), y0.min(y - r), x1.max(x + r), y1.max(y + r));
        }
    }
    if poles.is_empty() {
        (x0, y0, x1, y1) = (0.0, 0.0, 1.0, 1.0);
    }
    let mut o = String::new();
    let _ = writeln!(o, "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{:.1} {:.1} {:.1} {:.1}\" class=\"iso\">", x0 - 20.0, y0 - 20.0, x1 - x0 + 40.0, y1 - y0 + 40.0);
    for p in order {
        let ((ax, ay), (bx, by)) = (proj(p.origin), proj(p.end()));
        let _ = writeln!(o, "<line x1=\"{ax:.1}\" y1=\"{ay:.1}\" x2=\"{bx:.1}\" y2=\"{by:.1}\" stroke=\"#4a2f12\" stroke-width=\"{:.1}\"/>", p.diameter + 3.0);
        let _ = writeln!(o, "<line x1=\"{ax:.1}\" y1=\"{ay:.1}\" x2=\"{bx:.1}\" y2=\"{by:.1}\" stroke=\"#d9b77a\" stroke-width=\"{:.1}\"/>", p.diameter);
    }
    o.push_str("</svg>\n");
    o
}

/// One furniture as an HTML section: its isometric picture, the plan sheet, the cut list and the
/// building order.
pub fn meuble_section(title: &str, kind: &str, author: &str, poles: &[Pole]) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "<section class=\"meuble\"><h2>{}</h2><p class=\"sub\">{} · {} m de bois</p>", esc(title), esc(kind), mm(wood_metres(poles)));
    o.push_str(&iso_svg(poles));
    o.push_str("<table><tr><th>Qté</th><th>Pièce</th><th>Ø mm</th><th>Visible</th><th>À débiter</th></tr>");
    for r in cut_list(poles) {
        let _ = write!(o, "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>", r.count, esc(r.role), mm(r.diameter), mm(r.length), mm(r.cut_length));
    }
    o.push_str("</table><ol>");
    for s in steps(poles) {
        let _ = write!(o, "<li>{}</li>", esc(&s));
    }
    o.push_str("</ol>");
    o.push_str(&meuble_svg(title, kind, author, poles));
    o.push_str("</section>\n");
    o
}

/// A whole self-contained page (no scripts, no external files), printable on A3 landscape.
pub fn page_html(title: &str, sections: &[String]) -> String {
    let mut o = String::new();
    let _ = writeln!(
        o,
        "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{t}</title><style>\
@page{{size:A3 landscape;margin:8mm}}body{{font:15px system-ui,sans-serif;color:#1d2330;margin:0;background:#f1f2f4}}\
header{{background:#000761;color:#fff;padding:14px 24px}}header b{{color:#ff5a62}}h1{{margin:0;font-size:22px}}\
.meuble{{background:#fff;margin:16px auto;max-width:1100px;padding:16px 24px;border-radius:8px;break-after:page}}\
.sub{{color:#6b7280;margin-top:-8px}}.iso{{width:100%;max-height:360px}}\
table{{border-collapse:collapse;margin:12px 0}}td,th{{border:1px solid #d5d8de;padding:4px 10px;text-align:left}}th{{background:#eef0f4}}\
svg.iso+table{{width:100%}}section>svg:last-child{{width:100%;height:auto;border:1px solid #d5d8de}}\
</style></head><body><header><h1><b>SO</b><b style=\"color:#ff5a62\">LEAD</b>WORKS · {t}</h1></header>",
        t = esc(title)
    );
    for s in sections {
        o.push_str(s);
    }
    o.push_str("</body></html>\n");
    o
}

/// Greedy word wrap at `width` characters.
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

/// One furniture in a patrol corner: its name, the footprint's corner and size in the corner's
/// plane, millimetres.
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

/// The footprint of a placed furniture after its quarter turn about its own corner (x,y).
pub fn footprint(p: &Placed) -> (f64, f64, f64, f64) {
    let q = (p.rotation / 90.0).round() as i64 % 4;
    let (l, w) = (p.length, p.width);
    match q.rem_euclid(4) {
        0 => (p.x, p.y, p.x + l, p.y + w),
        1 => (p.x - w, p.y, p.x, p.y + l),
        2 => (p.x - l, p.y - w, p.x, p.y),
        _ => (p.x, p.y - l, p.x + w, p.y),
    }
}

/// The overview sheet of a patrol corner.
pub fn coin_svg(title: &str, author: &str, items: &[Placed]) -> String {
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
    let (w, h) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
    let den = nice_scale((w / 250.0).max(h / 190.0));
    let s = 1.0 / den;
    let (ox, oy) = (22.0, 30.0);
    let mut o = String::new();
    let _ = writeln!(o, "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {SHEET_W} {SHEET_H}\" width=\"{SHEET_W}mm\" height=\"{SHEET_H}mm\">{STYLE}");
    let _ = writeln!(o, "<rect width=\"{SHEET_W}\" height=\"{SHEET_H}\" fill=\"#fff\"/><rect x=\"5\" y=\"5\" width=\"{}\" height=\"{}\" class=\"frame\"/>", SHEET_W - 10.0, SHEET_H - 10.0);
    let _ = writeln!(o, "<text x=\"{ox}\" y=\"{}\" class=\"t\">Vue d'ensemble du coin (de dessus)</text>", oy - 6.0);
    for (i, b) in boxes.iter().enumerate() {
        let (bx, by, bw, bh) = (ox + (b.0 - x0) * s, oy + (y1 - b.3) * s, (b.2 - b.0) * s, (b.3 - b.1) * s);
        let _ = writeln!(o, "<rect x=\"{bx:.2}\" y=\"{by:.2}\" width=\"{bw:.2}\" height=\"{bh:.2}\" class=\"p\"/>");
        let _ = writeln!(o, "<text x=\"{:.2}\" y=\"{:.2}\" class=\"t\" text-anchor=\"middle\">{}</text>", bx + bw / 2.0, by + bh / 2.0 + 1.5, i + 1);
            }
    dim_h(&mut o, ox, ox + w * s, oy + h * s + 4.0, &mm(w));
    dim_v(&mut o, oy, oy + h * s, ox - 4.0, &mm(h));
    let x = 285.0;
    let mut y = 25.0;
    let _ = writeln!(o, "<text x=\"{x}\" y=\"{y}\" class=\"t\">Meubles du coin</text>");
    y += 6.0;
    for (i, p) in items.iter().enumerate() {
        let line = format!("{}. {} ({}) : {} × {} mm, {} m de bois", i + 1, p.name, p.kind, mm(p.length), mm(p.width), mm(p.wood_m));
        for (j, l) in wrap(&line, 62).iter().enumerate() {
            let _ = writeln!(o, "<text x=\"{x}\" y=\"{y:.2}\" class=\"b\">{}</text>", esc(&if j == 0 { l.clone() } else { format!("   {l}") }));
            y += 4.2;
        }
    }
    y += 3.0;
    let total: f64 = items.iter().map(|p| p.wood_m).sum();
    let _ = writeln!(o, "<text x=\"{x}\" y=\"{y:.2}\" class=\"h\">{}</text>", esc(&format!("Total : {} meubles, {} m de bois", items.len(), mm(total))));
    let _ = writeln!(o, "<text x=\"{x}\" y=\"{:.2}\" class=\"b\">Un plan détaillé est établi pour chaque meuble.</text>", y + 6.0);
    let _ = writeln!(o, "<rect x=\"230\" y=\"262\" width=\"185\" height=\"30\" class=\"frame\"/>");
    let _ = writeln!(o, "<text x=\"234\" y=\"270\" class=\"t\">{}</text>", esc(title));
    let _ = writeln!(o, "<text x=\"234\" y=\"276\" class=\"b\">{}</text>", esc(&format!("Plan de coin de patrouille   Échelle 1:{}   Format A3", mm(den))));
    let _ = writeln!(o, "<text x=\"234\" y=\"288\" class=\"b\">{}</text>", esc(&format!("Dessiné par : {author}   SoLeadWorks")));
    o.push_str("</svg>\n");
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scout::templates::{Dims, TEMPLATES, build};

    #[test]
    fn a_furniture_plan_is_well_formed_and_complete() {
        let poles = build("banc", Dims::of(&TEMPLATES[0])).unwrap_or_default();
        let svg = meuble_svg("Banc <1> & co", "Banc", "Lou", &poles);
        assert!(svg.starts_with("<svg") && svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("Liste de débit") && svg.contains("Montage") && svg.contains("Assemblages"));
        assert!(svg.contains("Banc &lt;1&gt; &amp; co"));
        assert!(svg.contains("1200"));
    }

    #[test]
    fn quarter_turns_swap_the_footprint() {
        let mut p = Placed { name: "t".into(), kind: "table".into(), x: 0.0, y: 0.0, length: 1500.0, width: 800.0, rotation: 0.0, wood_m: 1.0 };
        assert_eq!(footprint(&p), (0.0, 0.0, 1500.0, 800.0));
        p.rotation = 90.0;
        assert_eq!(footprint(&p), (-800.0, 0.0, 0.0, 1500.0));
        p.rotation = 270.0;
        assert_eq!(footprint(&p), (0.0, -1500.0, 800.0, 0.0));
    }

    #[test]
    fn the_html_page_is_self_contained() {
        let poles = build("table", Dims::of(&TEMPLATES[2])).unwrap_or_default();
        let html = page_html("Coin <x>", &[meuble_section("Table", "Table", "Lou", &poles)]);
        assert!(html.starts_with("<!doctype html>") && html.contains("Coin &lt;x&gt;"));
        assert!(!html.contains("<script"));
        assert!(html.contains("class=\"iso\"") && html.contains("Liste de débit"));
    }

    #[test]
    fn an_empty_corner_still_draws() {
        let svg = coin_svg("Coin", "Lou", &[]);
        assert!(svg.contains("Vue d'ensemble"));
    }
}
