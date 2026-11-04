//! Scout furniture and patrol corners (SoLeadWorks). `scout.meuble.*` builds a froissartage
//! furniture from a template: a component named after it that holds one component per pole,
//! each a round pole with its part number (so the bill of materials counts them), its role and
//! its place saved as properties; `scout.coin.create` makes a patrol corner, a component that
//! holds several furniture placed on the ground; `scout.plan.meuble` and `scout.plan.coin` draw
//! the plan sheets (SVG) and list what to cut, join and build.
//!
//! A furniture is built in one undo step, and everything is read back from the saved properties,
//! so a design opened later still plans.

use serde_json::{Value, json};

use super::CommandSpec;
use crate::params::{bad, num, str_, vec2};
use crate::scout::plan::{Placed, coin_svg, meuble_section, meuble_svg, page_html, steps};
use crate::scout::templates::{self, Axis, Dims, Joint, Pole, TEMPLATES, cut_list, extent, wood_metres};
use crate::{EngineError, Result, Session};

const P_POLE: &str = "scout_pole";
const P_KIND: &str = "scout_meuble";
const P_DIMS: &str = "scout_dimensions";
const P_PLACE: &str = "scout_place";
const P_COIN: &str = "scout_coin";

macro_rules! template_command {
    ($id:literal, $label:literal, $kind:literal, $icon:literal) => {
        CommandSpec::new($id, $label, |s, p| meuble_atomic(s, p, $kind))
            .at("SCOUT", "MEUBLE")
            .icon($icon)
            .params("name?, length?, width?, height?: mm; leg?, rail?, claie?: pole diameters in mm; coin?: patrol corner (id or name); at?: [x, y] mm; rotation?: 0|90|180|270 degrees → component, occurrence, poles, wood_m, cut_list")
    };
}

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("scout.meuble.types", "Modèles de meubles", |_, _| Ok(types())).icon("library").noundo().doc_only().params(
        "→ templates: [{id, name, category, summary, defaults: {length, width, height, leg, rail, claie}}] (mm)",
    ),
    template_command!("scout.meuble.banc", "Banc", "banc", "box"),
    template_command!("scout.meuble.banc_dossier", "Banc à dossier", "banc_dossier", "box"),
    template_command!("scout.meuble.table", "Table", "table", "box"),
    template_command!("scout.meuble.table_prepa", "Table de prépa avec étagère", "table_prepa", "box"),
    template_command!("scout.meuble.etagere", "Étagère", "etagere", "box"),
    CommandSpec::new("scout.meuble.create", "Nouveau meuble", |s, p| {
        let kind = str_(p, "type").ok_or_else(|| bad("scout.meuble.create", "`type` is required (scout.meuble.types)"))?.to_string();
        meuble_atomic(s, p, &kind)
    })
    .icon("box")
    .params("type: banc|banc_dossier|table|table_prepa|etagere; then as scout.meuble.banc"),
    CommandSpec::new("scout.coin.create", "Nouveau coin de patrouille", coin_atomic)
        .at("SCOUT", "COIN")
        .icon("component")
        .params("name?: the corner's name; layout?: standard (default: a table, two benches, a shelving and a prep table) | empty → component, meubles: [{component, name, type, at, rotation}]"),
    CommandSpec::new("scout.plan.meuble", "Plan du meuble", plan_meuble)
        .at("SCOUT", "PLANS")
        .icon("export")
        .noundo()
        .doc_only()
        .params("meuble?: id or name (default the only furniture of the design); path?: write the plan as a .svg file; author? → svg, dimensions, cut_list, joints, steps, wood_m"),
    CommandSpec::new("scout.plan.coin", "Plan du coin de patrouille", plan_coin)
        .at("SCOUT", "PLANS")
        .icon("export")
        .noundo()
        .doc_only()
        .params("coin?: id or name (default the only corner); path?: write the overview as a .svg file; author?; meuble_paths?: folder where every furniture's own plan is written too → svg, meubles, wood_m, plans?"),
    CommandSpec::new("scout.export.html", "Exporter en HTML", export_html)
        .at("SCOUT", "PLANS")
        .icon("export")
        .noundo()
        .doc_only()
        .params("path?: write a .html file (self-contained, no scripts, prints on A3); author?; title? → html, meubles (count), coins (count). Every patrol corner (its overview, then each furniture) and every furniture standing alone"),
];

fn types() -> Value {
    let list: Vec<Value> = TEMPLATES
        .iter()
        .map(|t| {
            let d = Dims::of(t);
            json!({"id": t.id, "name": t.name, "category": t.category, "summary": t.summary,
                   "defaults": {"length": d.length, "width": d.width, "height": d.height, "leg": d.leg, "rail": d.rail, "claie": d.claie}})
        })
        .collect();
    json!({ "templates": list })
}

fn run(s: &mut Session, id: &str, p: Value) -> Result<Value> {
    s.execute(id, &p)
}

fn id_of(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// The mm number as the text saved in a property (exact round trip).
fn t(v: f64) -> String {
    format!("{v}")
}

/// Runs `build` on a scratch copy and keeps it as one undo step only when it all worked.
fn atomic(s: &mut Session, f: impl FnOnce(&mut Session) -> Result<Value>) -> Result<Value> {
    let mut computed = s.scratch();
    let result = f(&mut computed)?;
    computed.undo.clear();
    computed.active_component = s.active_component;
    computed.active_occurrence_path = s.active_occurrence_path.clone();
    computed.active_sketch = s.active_sketch.filter(|id| computed.doc.feature(*id).is_some());
    s.accept_computed(computed);
    Ok(result)
}

fn meuble_atomic(s: &mut Session, p: &Value, kind: &str) -> Result<Value> {
    atomic(s, |c| create_meuble(c, p, kind))
}

fn dims_from(cmd: &str, kind: &str, p: &Value) -> Result<Dims> {
    let tpl = templates::template(kind).ok_or_else(|| bad(cmd, format!("unknown furniture `{kind}` (scout.meuble.types)")))?;
    let mut d = Dims::of(tpl);
    for (key, slot) in [("length", &mut d.length), ("width", &mut d.width), ("height", &mut d.height), ("leg", &mut d.leg), ("rail", &mut d.rail), ("claie", &mut d.claie)] {
        match p.get(key) {
            None | Some(Value::Null) => {}
            Some(v) => *slot = v.as_f64().filter(|x| x.is_finite()).ok_or_else(|| bad(cmd, format!("`{key}` must be a number in mm")))?,
        }
    }
    d.check().map_err(|m| bad(cmd, m))?;
    Ok(d)
}

fn rotation_of(cmd: &str, p: &Value) -> Result<f64> {
    let r = num(p, "rotation").unwrap_or(0.0);
    if [0.0, 90.0, 180.0, 270.0].iter().any(|q| (r - q).abs() < 1e-9) { Ok(r) } else { Err(bad(cmd, "`rotation` must be 0, 90, 180 or 270 degrees")) }
}

/// A free component name: `base`, or `base 2`, `base 3`…
fn free_name(s: &Session, base: &str) -> String {
    let taken = |n: &str| s.doc.components.iter().any(|c| c.name == n);
    if !taken(base) {
        return base.to_string();
    }
    (2..10_000).map(|i| format!("{base} {i}")).find(|n| !taken(n)).unwrap_or_else(|| format!("{base} (copie)"))
}

fn component_id(s: &Session, v: &Value, cmd: &str) -> Result<u64> {
    match v {
        Value::Number(n) => n.as_u64().filter(|id| s.doc.components.iter().any(|c| c.id == *id)).ok_or_else(|| bad(cmd, "no such component")),
        Value::String(name) => s.doc.components.iter().find(|c| c.name == *name).map(|c| c.id).ok_or_else(|| bad(cmd, format!("no component named `{name}`"))),
        _ => Err(bad(cmd, "a component is an id or a name")),
    }
}

fn matrix_move(s: &mut Session, occurrence: u64, m: [f64; 12]) -> Result<()> {
    run(s, "occurrence.move", json!({"occurrence": occurrence, "matrix": m, "capture": true, "move_grounded": true}))?;
    Ok(())
}

fn create_meuble(s: &mut Session, p: &Value, kind: &str) -> Result<Value> {
    let cmd = "scout.meuble.create";
    let d = dims_from(cmd, kind, p)?;
    let poles = templates::build(kind, d).map_err(|m| bad(cmd, m))?;
    let rotation = rotation_of(cmd, p)?;
    let at = p.get("at").map(|v| vec2(v).ok_or_else(|| bad(cmd, "`at` is [x, y] in mm"))).transpose()?;
    let coin = p.get("coin").filter(|v| !v.is_null()).map(|v| component_id(s, v, cmd)).transpose()?;
    if let Some(c) = coin
        && !s.doc.components.iter().any(|x| x.id == c && x.properties.contains_key(P_COIN))
    {
        return Err(bad(cmd, "`coin` is not a patrol corner (scout.coin.create)"));
    }
    let tpl_name = templates::template(kind).map_or(kind, |t| t.name);
    let name = free_name(s, str_(p, "name").map(str::trim).filter(|n| !n.is_empty()).unwrap_or(tpl_name));
    let active = s.active_component;
    let top = run(s, "component.create", json!({"name": name, "parent": coin.unwrap_or(0), "grounded": false}))?;
    let (comp, occ) = (id_of(&top, "component"), id_of(&top, "occurrence"));
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for pole in &poles {
        let n = counts.entry(pole.role.to_string()).or_insert(0);
        *n += 1;
        let part = format!("{} {} {}", pole.role, n, name);
        let c = run(s, "component.create", json!({"name": part, "parent": comp, "grounded": false}))?;
        let (pc, po) = (id_of(&c, "component"), id_of(&c, "occurrence"));
        run(s, "solid.cylinder", json!({"diameter": pole.diameter, "height": pole.length, "body_name": part}))?;
        matrix_move(s, po, pole.axis.matrix(pole.origin))?;
        let number = format!("{}-{}-{}x{}", kind.to_uppercase(), pole.role.to_uppercase().replace(' ', "_"), t(pole.diameter), t(pole.length));
        let desc = format!("{} Ø{} × {} mm", pole.role, t(pole.diameter), t(pole.length));
        let rec = [
            pole.role.to_string(),
            t(pole.diameter),
            t(pole.length),
            t(pole.origin[0]),
            t(pole.origin[1]),
            t(pole.origin[2]),
            pole.axis.name().to_string(),
            if pole.joint == Joint::TenonMortaise { "T".into() } else { "P".into() },
            t(pole.receiver),
        ]
        .join("|");
        run(
            s,
            "component.metadata.set",
            json!({"component": pc, "part_number": number, "description": desc, "properties": {P_POLE: rec, "essence": "bois rond", "diametre_mm": t(pole.diameter), "longueur_debit_mm": t(pole.cut_length())}}),
        )?;
        run(s, "component.activate", json!({"component": comp}))?;
    }
    let dims = [d.length, d.width, d.height, d.leg, d.rail, d.claie].map(t).join("|");
    let (x, y) = at.map_or((0.0, 0.0), |v| (v.x, v.y));
    run(
        s,
        "component.metadata.set",
        json!({"component": comp, "description": format!("{tpl_name}"), "properties": {P_KIND: kind, P_DIMS: dims, P_PLACE: format!("{}|{}|{}", t(x), t(y), t(rotation))}}),
    )?;
    if at.is_some() || rotation != 0.0 {
        let (c, sn) = (rotation.to_radians().cos().round(), rotation.to_radians().sin().round());
        matrix_move(s, occ, [c, -sn, 0.0, x, sn, c, 0.0, y, 0.0, 0.0, 1.0, 0.0])?;
    }
    run(s, "component.activate", json!({"component": active}))?;
    let rows = cut_list(&poles);
    Ok(json!({
        "component": comp, "occurrence": occ, "name": name, "type": kind, "poles": poles.len(),
        "wood_m": wood_metres(&poles),
        "cut_list": rows.iter().map(|r| json!({"role": r.role, "count": r.count, "diameter": r.diameter, "length": r.length, "cut_length": r.cut_length})).collect::<Vec<_>>(),
    }))
}

/// The standard corner: a table with a bench on each long side, a shelving and a prep table.
const STANDARD: &[(&str, f64, f64, f64)] = &[
    ("table", 0.0, 0.0, 0.0),
    ("banc", 150.0, -350.0, 0.0),
    ("banc", 150.0, 850.0, 0.0),
    ("etagere", 1700.0, 0.0, 0.0),
    ("table_prepa", 1700.0, 500.0, 0.0),
];

fn coin_atomic(s: &mut Session, p: &Value) -> Result<Value> {
    atomic(s, |c| {
        let cmd = "scout.coin.create";
        let layout = str_(p, "layout").unwrap_or("standard");
        if !matches!(layout, "standard" | "empty") {
            return Err(bad(cmd, "`layout` is standard or empty"));
        }
        let name = free_name(c, str_(p, "name").map(str::trim).filter(|n| !n.is_empty()).unwrap_or("Coin de patrouille"));
        let active = c.active_component;
        let made = run(c, "component.create", json!({"name": name, "parent": 0, "grounded": false}))?;
        let coin = id_of(&made, "component");
        run(c, "component.metadata.set", json!({"component": coin, "description": "Coin de patrouille", "properties": {P_COIN: "1"}}))?;
        run(c, "component.activate", json!({"component": active}))?;
        let mut placed = Vec::new();
        if layout == "standard" {
            for (kind, x, y, rot) in STANDARD {
                let r = create_meuble(c, &json!({"coin": coin, "at": [x, y], "rotation": rot}), kind)?;
                placed.push(json!({"component": r["component"], "name": r["name"], "type": kind, "at": [x, y], "rotation": rot}));
            }
        }
        Ok(json!({"component": coin, "name": name, "meubles": placed}))
    })
}

/// A furniture read back from the design.
struct Meuble {
    id: u64,
    name: String,
    kind: String,
    dims: Dims,
    place: (f64, f64, f64),
    poles: Vec<Pole>,
}

fn parse_axis(s: &str) -> Option<Axis> {
    Axis::parse(s)
}

fn read_pole(rec: &str, role_names: &[&'static str]) -> Option<Pole> {
    let f: Vec<&str> = rec.split('|').collect();
    let n = |i: usize| f.get(i).and_then(|x| x.parse::<f64>().ok()).filter(|x| x.is_finite());
    let role = role_names.iter().copied().find(|r| Some(*r) == f.first().copied())?;
    Some(Pole {
        role,
        diameter: n(1)?,
        length: n(2)?,
        origin: [n(3)?, n(4)?, n(5)?],
        axis: parse_axis(f.get(6)?)?,
        joint: if f.get(7) == Some(&"T") { Joint::TenonMortaise } else { Joint::Posee },
        receiver: n(8).unwrap_or(0.0),
    })
}

const ROLES: &[&str] = &["pied", "montant", "montant de dossier", "traverse longue", "traverse courte", "traverse de dossier", "claie"];

fn read_meuble(s: &Session, id: u64) -> Option<Meuble> {
    let c = s.doc.components.iter().find(|c| c.id == id)?;
    let kind = c.properties.get(P_KIND)?.clone();
    let dv: Vec<f64> = c.properties.get(P_DIMS)?.split('|').filter_map(|x| x.parse().ok()).collect();
    let dims = match dv.as_slice() {
        [length, width, height, leg, rail, claie] => Dims { length: *length, width: *width, height: *height, leg: *leg, rail: *rail, claie: *claie },
        _ => return None,
    };
    let pv: Vec<f64> = c.properties.get(P_PLACE)?.split('|').filter_map(|x| x.parse().ok()).collect();
    let place = match pv.as_slice() {
        [x, y, r] => (*x, *y, *r),
        _ => (0.0, 0.0, 0.0),
    };
    let poles = s.doc.components.iter().filter(|k| k.parent == id).filter_map(|k| k.properties.get(P_POLE)).filter_map(|rec| read_pole(rec, ROLES)).collect();
    Some(Meuble { id, name: c.name.clone(), kind, dims, place, poles })
}

fn pick(s: &Session, p: &Value, key: &str, prop: &str, what: &str, cmd: &str) -> Result<u64> {
    match p.get(key).filter(|v| !v.is_null()) {
        Some(v) => {
            let id = component_id(s, v, cmd)?;
            if s.doc.components.iter().any(|c| c.id == id && c.properties.contains_key(prop)) { Ok(id) } else { Err(bad(cmd, format!("`{key}` is not a {what}"))) }
        }
        None => {
            let all: Vec<u64> = s.doc.components.iter().filter(|c| c.properties.contains_key(prop)).map(|c| c.id).collect();
            match all.as_slice() {
                [one] => Ok(*one),
                [] => Err(bad(cmd, format!("the design has no {what}"))),
                _ => Err(bad(cmd, format!("several: name the `{key}` (the design has {})", all.len()))),
            }
        }
    }
}

fn author(s: &Session, p: &Value) -> String {
    str_(p, "author")
        .map(str::to_string)
        .or_else(|| s.doc.metadata.get("author").and_then(|v| v.as_str().map(str::to_string)))
        .unwrap_or_else(|| "—".into())
}

fn write_text(path: &str, text: &str) -> Result<()> {
    if !path.to_ascii_lowercase().ends_with(".svg") {
        return Err(EngineError::Other(format!("{path}: a plan is written as a .svg file")));
    }
    solidcraft_io::vfs::write(path, text.as_bytes()).map_err(|e| EngineError::Other(format!("{path}: {e}")))
}

fn meuble_result(m: &Meuble, svg: &str) -> Value {
    let rows = cut_list(&m.poles);
    let e = extent(&m.poles);
    json!({
        "meuble": m.id, "name": m.name, "type": m.kind, "svg": svg,
        "dimensions": {"length": e[0], "width": e[1], "height": e[2]},
        "wood_m": wood_metres(&m.poles),
        "cut_list": rows.iter().map(|r| json!({"role": r.role, "count": r.count, "diameter": r.diameter, "length": r.length, "cut_length": r.cut_length})).collect::<Vec<_>>(),
        "joints": rows.iter().filter(|r| r.joint == Joint::TenonMortaise).map(|r| json!({"role": r.role, "count": r.count, "joint": r.joint.label(), "tenon_diameter": (r.diameter / 3.0).ceil()})).collect::<Vec<_>>(),
        "steps": steps(&m.poles),
    })
}

fn plan_meuble(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.plan.meuble";
    let id = pick(s, p, "meuble", P_KIND, "furniture", cmd)?;
    let m = read_meuble(s, id).ok_or_else(|| bad(cmd, "this furniture's saved dimensions are unreadable"))?;
    let tpl = templates::template(&m.kind).map_or(m.kind.as_str(), |t| t.name);
    let svg = meuble_svg(&m.name, tpl, &author(s, p), &m.poles);
    if let Some(path) = str_(p, "path") {
        write_text(path, &svg)?;
    }
    Ok(meuble_result(&m, &svg))
}

fn plan_coin(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.plan.coin";
    let coin = pick(s, p, "coin", P_COIN, "patrol corner", cmd)?;
    let coin_name = s.doc.components.iter().find(|c| c.id == coin).map(|c| c.name.clone()).unwrap_or_default();
    let ids: Vec<u64> = s.doc.components.iter().filter(|c| c.parent == coin && c.properties.contains_key(P_KIND)).map(|c| c.id).collect();
    let meubles: Vec<Meuble> = ids.into_iter().filter_map(|id| read_meuble(s, id)).collect();
    let who = author(s, p);
    let placed: Vec<Placed> = meubles
        .iter()
        .map(|m| {
            let e = extent(&m.poles);
            Placed { name: m.name.clone(), kind: templates::template(&m.kind).map_or(m.kind.clone(), |t| t.name.to_string()), x: m.place.0, y: m.place.1, length: e[0], width: e[1], rotation: m.place.2, wood_m: wood_metres(&m.poles) }
        })
        .collect();
    let svg = coin_svg(&coin_name, &who, &placed);
    if let Some(path) = str_(p, "path") {
        write_text(path, &svg)?;
    }
    let mut plans = Vec::new();
    if let Some(dir) = str_(p, "meuble_paths") {
        for m in &meubles {
            let tpl = templates::template(&m.kind).map_or(m.kind.as_str(), |t| t.name);
            let file = format!("{}/{}.svg", dir.trim_end_matches('/'), m.name.replace(['/', '\\'], "-"));
            write_text(&file, &meuble_svg(&m.name, tpl, &who, &m.poles))?;
            plans.push(json!(file));
        }
    }
    let total: f64 = meubles.iter().map(|m| wood_metres(&m.poles)).sum();
    let mut out = json!({
        "coin": coin, "name": coin_name, "svg": svg, "wood_m": total,
        "meubles": placed.iter().zip(&meubles).map(|(pl, m)| json!({"meuble": m.id, "name": pl.name, "type": m.kind, "at": [pl.x, pl.y], "rotation": pl.rotation, "length": pl.length, "width": pl.width, "wood_m": pl.wood_m, "dimensions": [m.dims.length, m.dims.width, m.dims.height]})).collect::<Vec<_>>(),
    });
    if !plans.is_empty() {
        out["plans"] = Value::Array(plans);
    }
    Ok(out)
}

fn export_html(s: &mut Session, p: &Value) -> Result<Value> {
    let who = author(s, p);
    let mut sections = Vec::new();
    let (mut coins, mut count) = (0usize, 0usize);
    let coin_ids: Vec<u64> = s.doc.components.iter().filter(|c| c.properties.contains_key(P_COIN)).map(|c| c.id).collect();
    for coin in coin_ids {
        let name = s.doc.components.iter().find(|c| c.id == coin).map(|c| c.name.clone()).unwrap_or_default();
        let ids: Vec<u64> = s.doc.components.iter().filter(|c| c.parent == coin && c.properties.contains_key(P_KIND)).map(|c| c.id).collect();
        let ms: Vec<Meuble> = ids.into_iter().filter_map(|id| read_meuble(s, id)).collect();
        let placed: Vec<Placed> = ms
            .iter()
            .map(|m| {
                let e = extent(&m.poles);
                Placed { name: m.name.clone(), kind: templates::template(&m.kind).map_or(m.kind.clone(), |t| t.name.to_string()), x: m.place.0, y: m.place.1, length: e[0], width: e[1], rotation: m.place.2, wood_m: wood_metres(&m.poles) }
            })
            .collect();
        sections.push(format!("<section class=\"meuble\"><h2>{}</h2><p class=\"sub\">Coin de patrouille · {} meubles</p>{}</section>\n", name.replace('<', "&lt;").replace('&', "&amp;"), ms.len(), coin_svg(&name, &who, &placed)));
        for m in &ms {
            let tpl = templates::template(&m.kind).map_or(m.kind.as_str(), |t| t.name);
            sections.push(meuble_section(&m.name, tpl, &who, &m.poles));
            count += 1;
        }
        coins += 1;
    }
    let loose: Vec<u64> = s
        .doc
        .components
        .iter()
        .filter(|c| c.properties.contains_key(P_KIND) && !s.doc.components.iter().any(|k| k.id == c.parent && k.properties.contains_key(P_COIN)))
        .map(|c| c.id)
        .collect();
    for id in loose {
        if let Some(m) = read_meuble(s, id) {
            let tpl = templates::template(&m.kind).map_or(m.kind.as_str(), |t| t.name);
            sections.push(meuble_section(&m.name, tpl, &who, &m.poles));
            count += 1;
        }
    }
    if count == 0 {
        return Err(bad("scout.export.html", "the design has no furniture yet (Scout tab)"));
    }
    let title = str_(p, "title").map(str::trim).filter(|x| !x.is_empty()).unwrap_or("Plans du camp").to_string();
    let html = page_html(&title, &sections);
    if let Some(path) = str_(p, "path") {
        if !path.to_ascii_lowercase().ends_with(".html") {
            return Err(EngineError::Other(format!("{path}: write the export as a .html file")));
        }
        solidcraft_io::vfs::write(path, html.as_bytes()).map_err(|e| EngineError::Other(format!("{path}: {e}")))?;
    }
    Ok(json!({"html": html, "meubles": count, "coins": coins}))
}

#[cfg(test)]
#[path = "scout_tests.rs"]
mod tests;
