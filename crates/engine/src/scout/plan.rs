//! The pages built from a furniture's poles: the building order, the HTML page (a self-contained
//! page, no scripts, printable on A3) and the sheets of `dessin`.

use std::fmt::Write as _;

use super::dessin::{self, SheetCfg};
use super::pole::{Pole, cut_list, joint_summary, wood_metres};

pub use super::dessin::{Placed, footprint};

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

fn mm(v: f64) -> String {
    if (v - v.round()).abs() < 0.05 { format!("{}", v.round() as i64) } else { format!("{v:.1}") }
}

/// The building order of a furniture, from its poles: shape, then join, then lay the top.
pub fn steps(poles: &[Pole]) -> Vec<String> {
    let joints = joint_summary(poles);
    let n = |k: &str| joints.iter().filter(|(j, _)| j.contains(k)).map(|(_, c)| *c).sum::<usize>();
    let mut v = vec![format!("Débiter toutes les pièces d'après la liste de débit ({} pièces, {:.1} m de bois).", poles.len(), wood_metres(poles))];
    if n("fendu") + n("quart") > 0 {
        v.push("Fendre les rondins en deux (ou en quarts) dans leur longueur, face plate du bon côté.".into());
    }
    if n("tenon") + n("aronde") > 0 {
        v.push(format!("Façonner les {} tenons et percer les mortaises correspondantes (tenon = un tiers du diamètre de la pièce qui reçoit).", n("tenon") + n("aronde")));
    }
    if n("mi-bois") > 0 {
        v.push(format!("Tailler les {} mi-bois : chaque pièce est entaillée de la moitié de son épaisseur.", n("mi-bois")));
    }
    v.push("Assembler d'abord les petits côtés, puis les relier ; vérifier l'équerrage aux diagonales.".into());
    if n("cheville") > 0 {
        v.push("Percer et poser les chevilles, qui bloquent les assemblages.".into());
    }
    if poles.iter().any(|p| p.role == "claie") {
        v.push("Poser les claies, les régler à écartement régulier et les brêler.".into());
    }
    if poles.iter().any(|p| p.role == "ficelle") {
        v.push("Tendre le filet de ficelle, maille serrée.".into());
    }
    if n("brêlage") > 0 {
        v.push("Brêler les croisements restants.".into());
    }
    v
}

/// The standard sheet of a furniture, as SVG.
pub fn meuble_svg(title: &str, kind: &str, author: &str, poles: &[Pole]) -> String {
    let mut cfg = SheetCfg::standard(title, author);
    cfg.cartouche.reference = kind.to_string();
    dessin::svg(&dessin::sheet(poles, &cfg, (1, 1)))
}

/// The overview sheet of a patrol corner, as SVG.
pub fn coin_svg(title: &str, author: &str, items: &[Placed]) -> String {
    let cfg = SheetCfg::standard(title, author);
    dessin::svg(&dessin::sheet_coin(items, &cfg, (1, 1)))
}

pub fn iso_svg(poles: &[Pole]) -> String {
    dessin::iso_only(poles)
}

/// One furniture as an HTML section: its isometric picture, the plan sheet, the cut list and the
/// building order.
pub fn meuble_section(title: &str, kind: &str, author: &str, poles: &[Pole]) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "<section class=\"meuble\"><h2>{}</h2><p class=\"sub\">{} · {} m de bois</p>", esc(title), esc(kind), mm(wood_metres(poles)));
    o.push_str(&iso_svg(poles));
    o.push_str("<h3>Liste de débit</h3><table><tr><th>Qté</th><th>Pièce</th><th>Ø mm</th><th>Visible</th><th>À débiter</th></tr>");
    for r in cut_list(poles) {
        let section = if r.width > 0.0 { format!("{}×{}", mm(r.diameter), mm(r.width)) } else { mm(r.diameter) };
        let _ = write!(o, "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>", r.count, esc(&r.role), section, mm(r.length), mm(r.cut_length));
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::scout::models::{Params, build, model};

    fn poles_of(id: &str) -> Vec<Pole> {
        model(id).map(|m| build(id, &Params::of(m)).unwrap_or_default()).unwrap_or_default()
    }

    #[test]
    fn a_furniture_plan_is_well_formed_and_complete() {
        let svg = meuble_svg("Banc <1> & co", "Banc", "Lou", &poles_of("banc"));
        assert!(svg.starts_with("<svg") && svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("Nomenclature") && svg.contains("Banc &lt;1&gt; &amp; co"));
    }

    #[test]
    fn the_html_page_is_self_contained() {
        let html = page_html("Coin <x>", &[meuble_section("Table", "Table", "Lou", &poles_of("table"))]);
        assert!(html.starts_with("<!doctype html>") && html.contains("Coin &lt;x&gt;"));
        assert!(!html.contains("<script"));
        assert!(html.contains("class=\"iso\"") && html.contains("Liste de débit"));
        assert!(!steps(&poles_of("table")).is_empty() && wood_metres(&[]) == 0.0);
    }
}

#[cfg(test)]
mod thumbnails {
    use super::*;
    use crate::scout::models::{MODELS, Params, build};

    /// Writes each model's isometric picture as `<dir>/<id>.svg` (the start page's pictures are made
    /// from these): `SCOUT_THUMBS=/some/dir cargo test -p solidcraft-engine --lib -- --ignored thumbnails`.
    #[test]
    #[ignore = "writes files; run by hand to refresh the start page's pictures"]
    fn write_the_start_page_pictures() {
        let Ok(dir) = std::env::var("SCOUT_THUMBS") else { return };
        for m in MODELS {
            let poles = build(m.id, &Params::of(m)).unwrap_or_default();
            let _ = std::fs::write(format!("{dir}/{}.svg", m.id), iso_svg(&poles));
        }
    }
}
