//! Scout furniture, joints and plans (SoLeadWorks), the functions of CathIA on SolidCraft's kernel.
//!
//! - `scout.meuble.create` builds one of the eleven models: a component named after it that holds
//!   one component per pole (or board, or rope), each with its part number (so the bill of
//!   materials counts them), its record and its joints. Joints are real geometry: a tenon is a stub
//!   joined to the pole, a mortise or a half-lap notch a cut, a peg a pole of its own.
//! - `scout.assemblage` joins two poles already in the design (half-lap, the tenons, the dovetails,
//!   pegs, lashing); `scout.faconner` splits, cuts and sharpens one; `scout.piece.*` add a pole, a
//!   set square, a deck of claies or a net.
//! - `scout.coin.create` makes a patrol corner, a component holding several furniture on the ground;
//!   `scout.atelier.dessin` (the technical drawing), `scout.atelier.nomenclature` and
//!   `scout.export.html` draw the plans and list what to
//!   cut, join and build.
//!
//! Every change is one undo step and every pole's record is saved in its component's properties
//! (`scout_pole`), so a design opened later still plans and still joins.

use serde_json::{Value, json};

use super::CommandSpec;
use crate::params::{bad, num, str_, vec2, vec3};
use crate::scout::models::{self, MODELS, Params};
use crate::scout::dessin::{self, Cartouche, Format, Placed, SheetCfg, View};
use crate::scout::nomenclature;
use crate::scout::plan::{meuble_section, page_html, steps};
use crate::scout::pole::{self, Build, End, MAX_POLES, Op, Pole, SplitMode, Style, TenonOpts, V3, add, cut_list, extent, joint_summary, len, mul, wood_metres};
use crate::{EngineError, Result, Session};

const P_POLE: &str = "scout_pole";
const P_KIND: &str = "scout_meuble";
const P_PARAMS: &str = "scout_reglages";
const P_PLACE: &str = "scout_place";
const P_COIN: &str = "scout_coin";
/// The pole as it was before the first joint or shaping made on it (the point a rebuild restarts from).
const P_BASE: &str = "scout_base";
/// The design's own key: the joints and shapings made, in order, so a changed pole can have them redone.
const K_RECIPES: &str = "fr.soleadworks.recettes";
/// On a pole built without its hidden joints (the quick way): how many of its first operations are
/// still to be cut (see `detail`).
const P_FAST: &str = "scout_rapide";

/// Joints that cannot be seen from outside (the tenon in its mortise, a peg's hole): the quick way
/// leaves them out of the geometry and `scout.meuble.detail` cuts them when asked.
fn hidden(op: &Op) -> bool {
    matches!(op, Op::Tenon { .. } | Op::Mortise { .. } | Op::Hole { .. } | Op::Saddle { .. })
}

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("scout.meuble.types", "Modèles de meubles", |_, _| Ok(types())).icon("library").noundo().doc_only().params(
        "→ models: [{id, name, category, summary, params: [{key, label, default, options?, check?}]}] (CathIA's eleven)",
    ),
    CommandSpec::new("scout.meuble.create", "Nouveau meuble", |s, p| atomic(s, |c| create_meuble(c, p)))
        .at("SCOUT", "MEUBLE")
        .icon("scout_meuble")
        .params("type: etabli|banc|fauteuil|bancdos|bancdiag|table|prepa|cuisine|etagere|vaisselier|feu; name?; the model's own settings by key (scout.meuble.types), at top level or in params: {…}; coin?: patrol corner (id or name); at?: [x, y] mm; rotation?: 0|90|180|270 → component, occurrence, name, type, poles, wood_m, cut_list, joints"),
    CommandSpec::new("scout.coin.create", "Nouveau coin de patrouille", |s, p| atomic(s, |c| create_coin(c, p)))
        .at("SCOUT", "COIN")
        .icon("scout_coin")
        .params("name?; layout?: standard (a table, two benches, a shelving and a prep table) | empty → component, meubles"),
    CommandSpec::new("scout.assemblage", "Assemblage", |s, p| atomic(s, |c| assemblage(c, p)))
        .icon("joint")
        .params("kind: mibois|hache|scie|plat|platscie|aronde1|aronde2|aronde3|cheville|brelage; a, b: the two poles (component name or id; for a tenon, a carries the tenon and b the mortise); through?: bool (mortise through); pc?: % of the half-lap taken from a (default 50); d?: tenon Ø / thickness or peg Ø mm; w?: flat tenon width mm; theta?: dovetail angle °; turns?, rope?: lashing → joint, changed, created"),
    CommandSpec::new("scout.faconner", "Façonner une pièce", |s, p| atomic(s, |c| faconner(c, p)))
        .icon("scale")
        .params("kind: demi|quart|couper|pointe; piece: component name or id; length?: new length mm (couper, from the start); len?: length of the point mm; end?: start|end (pointe, default end)"),
    // The ribbon's buttons: each opens its dialog on the right kind and runs the same command.
    CommandSpec::new("scout.assemblage.mibois", "Mi-bois", |s, p| atomic(s, |c| assemblage(c, &with_kind(p, "mibois")))).icon("scout_mibois").params("a, b; pc? (see scout.assemblage)"),
    CommandSpec::new("scout.assemblage.tenon", "Tenon et mortaise", |s, p| atomic(s, |c| assemblage(c, &with_kind(p, "hache")))).icon("scout_tenon").params("a, b; kind? hache|scie|plat|platscie; through?, d?, w? (see scout.assemblage)"),
    CommandSpec::new("scout.assemblage.aronde", "Queue d’aronde", |s, p| atomic(s, |c| assemblage(c, &with_kind(p, "aronde1")))).icon("scout_aronde").params("a, b; kind? aronde1|aronde2|aronde3; theta? (see scout.assemblage)"),
    CommandSpec::new("scout.assemblage.cheville", "Chevilles", |s, p| atomic(s, |c| assemblage(c, &with_kind(p, "cheville")))).icon("scout_cheville").params("a, b; d? (see scout.assemblage)"),
    CommandSpec::new("scout.assemblage.brelage", "Brêlage", |s, p| atomic(s, |c| assemblage(c, &with_kind(p, "brelage")))).icon("scout_brelage").params("a, b; turns?, rope? (see scout.assemblage)"),
    CommandSpec::new("scout.faconner.demi", "Fendre", |s, p| atomic(s, |c| faconner(c, &with_kind(p, "demi")))).icon("scout_fendre").params("piece (see scout.faconner)"),
    CommandSpec::new("scout.faconner.quart", "Quart de bois", |s, p| atomic(s, |c| faconner(c, &with_kind(p, "quart")))).icon("scout_quart").params("piece (see scout.faconner)"),
    CommandSpec::new("scout.faconner.couper", "Couper", |s, p| atomic(s, |c| faconner(c, &with_kind(p, "couper")))).icon("scout_couper").params("piece, length (see scout.faconner)"),
    CommandSpec::new("scout.faconner.pointe", "Pointe", |s, p| atomic(s, |c| faconner(c, &with_kind(p, "pointe")))).icon("scout_pointe").params("piece, len?, end? (see scout.faconner)"),
    CommandSpec::new("scout.piece.traverse", "Traverse", |s, p| atomic(s, |c| piece_traverse(c, p)))
        .at("SCOUT", "PIÈCES")
        .icon("scout_traverse")
        .params("diameter?: mm (default 80); length?: mm (default 1500); at?: [x, y, z] start point mm; dir?: [x, y, z] direction (default along x); name? → component, name"),
    CommandSpec::new("scout.piece.equerre", "Équerre", |s, p| atomic(s, |c| piece_equerre(c, p)))
        .at("SCOUT", "PIÈCES")
        .icon("scout_equerre")
        .params("unit?: mm for the 3-4-5 (default 300: poles of 0.9, 1.2 and 1.5 m); d?: pole Ø mm (default 50); at?: [x, y]"),
    CommandSpec::new("scout.meuble.detail", "Détailler les assemblages", |s, p| atomic(s, |c| detail_cmd(c, p)))
        .at("SCOUT", "MEUBLES")
        .icon("scout_tenon")
        .params("meuble?: name or id (default: all); cuts the tenons, mortises and peg holes left out of the furniture built the quick way (slow: seconds per joint)"),
    CommandSpec::new("scout.meuble.modifier", "Modifier le meuble", |s, p| atomic(s, |c| meuble_modifier(c, p)))
        .at("SCOUT", "MEUBLES")
        .icon("scout_equerre")
        .params("meuble?: name or id; any setting of its model (see scout.meuble.types), alone or under params → builds it again in place and redoes the joints made on its pieces since (one undo step)"),
    CommandSpec::new("scout.piece.modifier", "Modifier une pièce", |s, p| atomic(s, |c| modifier(c, p)))
        .at("SCOUT", "PIÈCES")
        .icon("scout_traverse")
        .params("piece: name or id; diameter?, length?: mm; dx?, dy?, dz?: move in mm → rebuilds the piece and every joint that depends on it (one undo step)"),
    CommandSpec::new("scout.plateau", "Plateau", |s, p| atomic(s, |c| piece_plateau(c, p)))
        .at("SCOUT", "PIÈCES")
        .icon("scout_claies")
        .params("mode?: tresse|droit|filet|quarts (default tresse); long?, larg?: mm (default 1000 × 600); dc?: claie Ø mm (20); dr?: rail Ø mm (40); maille?: mm (30); c0?: start of the claying mm; cl?: claied length mm (0 = to the end); z?: height of the bottom mm; at?: [x, y]"),
    CommandSpec::new("scout.profil.tube_rond", "Tube rond", |s, p| atomic(s, |c| piece_profil(c, &with_kind(p, "tube_rond")))).icon("cylinder").params("see scout.profil"),
    CommandSpec::new("scout.profil.tube_carre", "Tube carré", |s, p| atomic(s, |c| piece_profil(c, &with_kind(p, "tube_carre")))).icon("box").params("see scout.profil"),
    CommandSpec::new("scout.profil.corniere", "Cornière", |s, p| atomic(s, |c| piece_profil(c, &with_kind(p, "corniere")))).icon("box").params("see scout.profil"),
    CommandSpec::new("scout.profil.u", "Profilé en U", |s, p| atomic(s, |c| piece_profil(c, &with_kind(p, "u")))).icon("box").params("see scout.profil"),
    CommandSpec::new("scout.profil.i", "Poutre en I", |s, p| atomic(s, |c| piece_profil(c, &with_kind(p, "i")))).icon("box").params("see scout.profil"),
    CommandSpec::new("scout.profil", "Profilé", |s, p| atomic(s, |c| piece_profil(c, p)))
        .icon("box")
        .params("kind?: tube_rond|tube_carre|corniere|u|i (default tube_carre); size?: mm (default 40); epaisseur?: wall mm (default 3); length?: mm (default 1000); at?: [x, y, z]; dir?: X|Y|Z or [x, y, z]; name? → component, name"),
    CommandSpec::new("scout.atelier.dessin", "Dessin technique", dessin_cmd)
        .at("PLANS", "FEUILLE")
        .icon("scout_plan")
        .noundo()
        .doc_only()
        .params("meuble?|coin?: id or name (default the only furniture, else the only corner); format?: A3|A4; echelle?: n for 1:n (0 = automatic); vues?: [face, arriere, droite, gauche, dessus, dessous, iso]; cotes?: bool (overall dimensions); cotes_pieces?: bool; tableau?: bool; titre?, societe?, dessinateur?, date?, materiau?, reference?, indice?: the title block; path?: write a .svg → name, sujet (meuble|coin), svg, sheet: {w, h, echelle}, prims"),
    CommandSpec::new("scout.atelier.nomenclature", "Nomenclature", nomenclature_cmd)
        .at("NOMENCLATURE", "TABLEAUX")
        .icon("scout_nomenclature")
        .noundo()
        .doc_only()
        .params("meuble?|coin?: id or name (default the whole design); path?: write the three tables as a .csv → pieces, assemblages, materiel, csv"),
    CommandSpec::new("scout.export.html", "Exporter en HTML", export_html)
        .at("PLANS", "EXPORTER")
        .icon("scout_html")
        .noundo()
        .doc_only()
        .params("path?: write a .html file (self-contained, no scripts, prints on A3); author?; title? → html, meubles (count), coins (count)"),
];

/// `p` with `kind` set when it does not name one.
fn with_kind(p: &Value, kind: &str) -> Value {
    let mut q = p.clone();
    if let Value::Object(m) = &mut q {
        m.entry("kind").or_insert_with(|| json!(kind));
    }
    q
}

fn types() -> Value {
    let list: Vec<Value> = MODELS
        .iter()
        .map(|m| {
            let params: Vec<Value> = m
                .params
                .iter()
                .map(|d| json!({"key": d.key, "label": d.label, "default": d.default, "options": d.options, "check": d.check}))
                .collect();
            json!({"id": m.id, "name": m.name, "category": m.category, "summary": m.summary, "params": params})
        })
        .collect();
    json!({ "models": list })
}

fn run(s: &mut Session, id: &str, p: Value) -> Result<Value> {
    #[cfg(test)]
    {
        let t = std::time::Instant::now();
        let r = s.execute(id, &p);
        PROFILE.with(|m| {
            let mut m = m.borrow_mut();
            let e = m.entry(id.to_string()).or_insert((0, 0));
            e.0 += 1;
            e.1 += t.elapsed().as_millis();
        });
        r
    }
    #[cfg(not(test))]
    s.execute(id, &p)
}

#[cfg(test)]
thread_local! {
    pub static PROFILE: std::cell::RefCell<std::collections::BTreeMap<String, (u32, u128)>> = const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}

fn id_of(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// Runs `f` on a scratch copy and keeps it as one undo step only when it all worked.
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

fn e(m: String) -> EngineError {
    EngineError::Other(m)
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

fn pole_of(s: &Session, id: u64) -> Option<Pole> {
    s.doc.components.iter().find(|c| c.id == id).and_then(|c| c.properties.get(P_POLE)).and_then(|t| serde_json::from_str(t).ok())
}

fn name_of(s: &Session, id: u64) -> String {
    s.doc.components.iter().find(|c| c.id == id).map(|c| c.name.clone()).unwrap_or_default()
}

fn piece(s: &Session, p: &Value, key: &str, cmd: &str) -> Result<(u64, Pole)> {
    let v = p.get(key).filter(|v| !v.is_null()).ok_or_else(|| bad(cmd, format!("`{key}` is required: a pole (component name or id)")))?;
    let id = component_id(s, v, cmd)?;
    let pole = pole_of(s, id).ok_or_else(|| bad(cmd, format!("`{key}` is not a pole made by SoLeadWorks (a furniture's pole, a Traverse…)")))?;
    Ok((id, pole))
}

fn part_number(kind: &str, pole: &Pole) -> String {
    let size = if pole.width > 0.0 { format!("{}x{}x{}", pole.diameter, pole.width, pole.length.round()) } else { format!("{}x{}", pole.diameter, pole.length.round()) };
    format!("{}-{}-{}", kind.to_uppercase(), pole.role.to_uppercase().replace([' ', '\''], "_"), size)
}

thread_local! {
    static TOOLS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// A fresh name for a cutting or joining tool body.
fn tool_name(name: &str) -> String {
    let n = TOOLS.with(|c| {
        c.set(c.get() + 1);
        c.get()
    });
    format!("{name} outil {n}")
}

/// A new body for a tool, then `solid.combine` of it into the pole's body: the interface's own
/// Boolean, so every tenon, mortise, notch and split is a Combine in the history.
fn combine(s: &mut Session, name: &str, tool: &str, operation: &str) -> Result<()> {
    run(s, "solid.combine", json!({"target": name, "tools": [tool], "operation": operation}))?;
    Ok(())
}

/// The same, but the tool is another pole's body and stays.
fn combine_keep(s: &mut Session, name: &str, tool: &str, operation: &str) -> Result<()> {
    run(s, "solid.combine", json!({"target": name, "tools": [tool], "operation": operation, "keep_tools": true}))?;
    Ok(())
}

/// A point or a direction of the pole's own frame, as the commands take it: in the world, with the
/// placement of the component being edited (a component placed since it was built reads the numbers
/// of the features it is given in the world, and keeps the boxes aligned with its own axes).
fn world_point(s: &Session, p: V3) -> V3 {
    let q = solidcraft_doc::apply_point(&s.active_component_frame(), solidcraft_geom::Vec3::new(p[0], p[1], p[2]));
    [q.x, q.y, q.z]
}

fn world_dir(s: &Session, v: V3) -> V3 {
    let q = solidcraft_doc::apply_vector(&s.active_component_frame(), solidcraft_geom::Vec3::new(v[0], v[1], v[2]));
    [q.x, q.y, q.z]
}

fn tool_cylinder(s: &mut Session, tool: &str, base: V3, axis: V3, d: f64, h: f64) -> Result<()> {
    let (base, axis) = (world_point(s, base), world_dir(s, axis));
    run(s, "solid.cylinder", json!({"base": base, "axis": axis, "diameter": d, "height": h, "body_name": tool}))?;
    Ok(())
}

fn tool_box(s: &mut Session, tool: &str, centre: V3, half: V3) -> Result<()> {
    // the box's corner (its smallest x, y, z in the pole's frame) taken into the world
    let corner = world_point(s, [centre[0] - half[0], centre[1] - half[1], centre[2] - half[2]]);
    run(s, "solid.box", json!({"corner": corner, "length": 2.0 * half[0], "width": 2.0 * half[1], "height": 2.0 * half[2], "body_name": tool}))?;
    Ok(())
}

/// The sketch plane whose normal is the local axis `s`, and the sketch coordinates of a point on it
/// (the plane XZ draws z downwards; the offset runs along the normal, +y, +x, +z).
fn sketch_plane(s: usize) -> &'static str {
    match s {
        0 => "YZ",
        1 => "XZ",
        _ => "XY",
    }
}

fn to_sketch(plane: &str, p: V3) -> [f64; 2] {
    match plane {
        "XZ" => [p[0], -p[2]],
        "YZ" => [p[1], p[2]],
        _ => [p[0], p[1]],
    }
}

/// A prism: a polygon (model points on the plane with normal axis `s`) extruded `thickness`, centred
/// on the plane, as a new body named `tool`.
fn tool_prism(s: &mut Session, tool: &str, axis: usize, offset: f64, pts: &[V3], thickness: f64) -> Result<()> {
    let plane = sketch_plane(axis);
    let mut list: Vec<[f64; 2]> = pts.iter().map(|q| to_sketch(plane, *q)).collect();
    if let Some(first) = list.first().copied() {
        list.push(first);
    }
    run(s, "sketch.create", json!({"plane": plane, "offset": offset}))?;
    run(s, "sketch.line", json!({"points": list}))?;
    run(s, "sketch.finish", json!({}))?;
    run(s, "solid.extrude", json!({"distance": thickness, "direction": "symmetric", "body_name": tool}))?;
    Ok(())
}

/// Applies one operation to a pole's body (named `name`), in the pole's own frame, with tool bodies
/// and Combine.
fn apply_op(s: &mut Session, name: &str, pole: &Pole, op: &Op) -> Result<()> {
    let big = pole.radius() * 2.0 + 4.0;
    match op {
        Op::Tenon { end, style, d, w, len, wide, theta, typ } => {
            let (z, sgn) = if *end == End::Start { (pole.start, -1.0) } else { (pole.start + pole.length, 1.0) };
            let tool = tool_name(name);
            if style.round() {
                // a little into the pole, so the stub is joined and not merely touching
                tool_cylinder(s, &tool, [0.0, 0.0, z - sgn], [0.0, 0.0, sgn], *d, len + 1.0)?;
            } else if *style == Style::Aronde {
                // a dovetail: narrow at the shoulder, flaring towards the tip, `d` thick
                let (hn, ht) = (w / 2.0, w / 2.0 + len * theta.to_radians().tan());
                let (z0, z1) = (z - sgn, z + sgn * len);
                let pts: Vec<V3> = if *wide == 0 {
                    vec![[-hn, 0.0, z0], [hn, 0.0, z0], [ht, 0.0, z1], [-ht, 0.0, z1]]
                } else {
                    vec![[0.0, -hn, z0], [0.0, hn, z0], [0.0, ht, z1], [0.0, -ht, z1]]
                };
                tool_prism(s, &tool, if *wide == 0 { 1 } else { 0 }, 0.0, &pts, *d)?;
                let _ = typ;
            } else {
                let (sx, sy) = if *wide == 0 { (*w, *d) } else { (*d, *w) };
                let (z0, h) = if sgn > 0.0 { (z - 1.0, len + 1.0) } else { (z - len, len + 1.0) };
                tool_box(s, &tool, [0.0, 0.0, z0 + h / 2.0], [sx / 2.0, sy / 2.0, h / 2.0])?;
            }
            combine(s, name, &tool, "join")?;
        }
        Op::Mortise { c, u, style, d, w, depth, wide, theta, typ, open } => {
            let tool = tool_name(name);
            if style.round() {
                tool_cylinder(s, &tool, *c, *u, *d, *depth)?;
            } else if *style == Style::Aronde {
                // the groove across (or along) the receiving pole that the dovetail slides into
                let m = dominant(*u);
                let sign = if u[m] >= 0.0 { 1.0 } else { -1.0 };
                let (wdir, slide) = if m == 2 {
                    (0, 1)
                } else if *typ == 3 {
                    (1 - m, 2)
                } else {
                    (2, 1 - m)
                };
                let _ = wide;
                let len = (depth - 3.0 - 1.0).max(1.0);
                let (hn, ht) = (w / 2.0, w / 2.0 + len * theta.to_radians().tan());
                let at = |dist: f64, hw: f64, side: f64| -> V3 {
                    let mut p = *c;
                    p[m] += sign * dist;
                    p[wdir] += side * hw;
                    p
                };
                let far = 3.0 + len + 1.0;
                let pts = vec![at(0.0, hn, -1.0), at(0.0, hn, 1.0), at(3.0, hn, 1.0), at(far, ht, 1.0), at(far, ht, -1.0), at(3.0, hn, -1.0)];
                tool_prism(s, &tool, slide, c[slide], &pts, open.max(*d))?;
            } else {
                let m = dominant(*u);
                let sign = if u[m] >= 0.0 { 1.0 } else { -1.0 };
                let mut centre = *c;
                centre[m] += sign * depth / 2.0;
                let mut half = [0.0; 3];
                half[m] = depth / 2.0;
                if m == 2 {
                    half[0] = w / 2.0;
                    half[1] = d / 2.0;
                } else {
                    half[2] = w / 2.0;
                    half[1 - m] = d / 2.0;
                }
                tool_box(s, &tool, centre, half)?;
            }
            combine(s, name, &tool, "cut")?;
        }
        Op::Notch { c, h } => {
            let tool = tool_name(name);
            tool_box(s, &tool, *c, *h)?;
            combine(s, name, &tool, "cut")?;
        }
        Op::Hole { c, u, d, depth } => {
            let tool = tool_name(name);
            tool_cylinder(s, &tool, *c, *u, *d, *depth)?;
            combine(s, name, &tool, "cut")?;
        }
        Op::Split { mode } => {
            let (zc, hz) = (pole.start + pole.length / 2.0, pole.length / 2.0 + 2.0);
            // everything on the +x side goes
            let tool = tool_name(name);
            tool_box(s, &tool, [big / 2.0, 0.0, zc], [big / 2.0, big, hz])?;
            combine(s, name, &tool, "cut")?;
            if *mode == SplitMode::Quart {
                // and, of the remaining half, the +y side
                let tool = tool_name(name);
                tool_box(s, &tool, [-big / 2.0, big / 2.0, zc], [big / 2.0, big / 2.0, hz])?;
                combine(s, name, &tool, "cut")?;
            }
        }
        Op::CutOff { end, at } => {
            let (zc, hz) = if *end == End::End { (at + pole.length + 10.0, pole.length + 10.0) } else { (at - pole.length - 10.0, pole.length + 10.0) };
            let tool = tool_name(name);
            tool_box(s, &tool, [0.0, 0.0, zc], [big, big, hz])?;
            combine(s, name, &tool, "cut")?;
        }
        Op::Saddle { with, .. } => {
            // the end of this pole is cut to the other pole's body (a wolf's mouth): the joint has
            // neither a gap nor an overlap
            if !with.starts_with('#') {
                combine_keep(s, name, with, "cut")?;
            }
        }
        Op::Point { end, len } => {
            let r = pole.radius();
            let (at, sgn) = if *end == End::Start { (pole.start + len, -1.0) } else { (pole.start + pole.length - len, 1.0) };
            let (zc, hz) = if *end == End::End { (at + len / 2.0 + 5.0, len / 2.0 + 5.0) } else { (at - len / 2.0 - 5.0, len / 2.0 + 5.0) };
            let tool = tool_name(name);
            tool_box(s, &tool, [0.0, 0.0, zc], [big, big, hz])?;
            combine(s, name, &tool, "cut")?;
            let cone = tool_name(name);
            let (cb, ca) = (world_point(s, [0.0, 0.0, at - sgn]), world_dir(s, [0.0, 0.0, sgn]));
            run(s, "solid.cone", json!({"base": cb, "axis": ca, "radius": r * 0.995, "height": *len + 1.0, "body_name": cone}))?;
            combine(s, name, &cone, "join")?;
        }
    }
    Ok(())
}

fn dominant(v: V3) -> usize {
    let a = v.map(f64::abs);
    if a[0] >= a[1] && a[0] >= a[2] { 0 } else if a[1] >= a[2] { 1 } else { 2 }
}

/// Makes the component of one pole under `parent`, builds and places it, saves its record.
/// A bar whose axis is a spline (a woven claie): the body is swept along it, then copied by a
/// pattern when the pole is an array.
fn curved_body(s: &mut Session, name: &str, pole: &Pole) -> Result<()> {
    let pts: Vec<[f64; 2]> = pole.curve.iter().map(|[x, z]| [*x, -*z]).collect();
    run(s, "sketch.create", json!({"plane": "XZ"}))?;
    let sketch = s.active_sketch.ok_or_else(|| e("no sketch".into()))?;
    let made = run(s, "sketch.spline.fit_point", json!({"points": pts}))?;
    let curve = made["curves"][0].as_str().ok_or_else(|| e("the spline has no curve".into()))?.to_string();
    run(s, "sketch.finish", json!({}))?;
    run(s, "solid.pipe", json!({"path_sketch": sketch, "path": [curve], "diameter": pole.diameter, "body_name": name}))?;
    if let Some(a) = &pole.array {
        let step = pole.local_dir(a.step);
        let spacing = len(a.step);
        if a.count > 1 && spacing > 0.0 {
            run(s, "solid.pattern.rectangular", json!({"bodies": [name], "dir1": step, "count1": a.count, "spacing1": spacing}))?;
        }
    }
    Ok(())
}

fn make_pole(s: &mut Session, parent: u64, kind: &str, name: &str, pole: &Pole, detail: bool) -> Result<(u64, u64)> {
    let c = run(s, "component.create", json!({"name": name, "parent": parent, "grounded": false}))?;
    let (comp, occ) = (id_of(&c, "component"), id_of(&c, "occurrence"));
    let (z0, z1) = pole.built_range();
    if !pole.curve.is_empty() {
        curved_body(s, name, pole)?;
    } else if pole.width > 0.0 {
        run(s, "solid.box", json!({"corner": [-pole.diameter / 2.0, -pole.width / 2.0, z0], "length": pole.diameter, "width": pole.width, "height": z1 - z0, "body_name": name}))?;
    } else {
        run(s, "solid.cylinder", json!({"base": [0.0, 0.0, z0], "axis": [0.0, 0.0, 1.0], "diameter": pole.diameter, "height": z1 - z0, "body_name": name}))?;
    }
    for op in pole.ops.iter().filter(|o| !matches!(o, Op::Saddle { .. }) && (detail || !hidden(o))) {
        apply_op(s, name, pole, op)?;
    }
    run(s, "occurrence.move", json!({"occurrence": occ, "matrix": pole.matrix(), "capture": true, "move_grounded": true}))?;
    save_pole(s, comp, kind, pole)?;
    if !detail && pole.ops.iter().any(hidden) {
        run(s, "component.metadata.set", json!({"component": comp, "properties": {P_FAST: pole.ops.len().to_string()}}))?;
    }
    run(s, "component.activate", json!({"component": parent}))?;
    Ok((comp, occ))
}

/// Once every pole of a part exists: each end cut to fit another pole is cut to it (`#n` names the
/// pole of index n), and the records are saved with the real names.
fn fit_saddles(s: &mut Session, kind: &str, comps: &[(u64, String)], poles: &mut [Pole]) -> Result<()> {
    for i in 0..poles.len() {
        let mut changed = false;
        for k in 0..poles[i].ops.len() {
            let Op::Saddle { with, end, extra } = poles[i].ops[k].clone() else { continue };
            let Some(j) = with.strip_prefix('#').and_then(|n| n.parse::<usize>().ok()) else { continue };
            let Some((_, target)) = comps.get(j) else { continue };
            let Some((comp, name)) = comps.get(i) else { continue };
            run(s, "component.activate", json!({"component": *comp}))?;
            let recv = poles[j].clone();
            if recv.width == 0.0 {
                // A plain cylinder of the receiving pole is a far lighter cutter than its body (with
                // all its mortises): cut with it, then put the tenons back, which it also took.
                let (z0, z1) = recv.built_range();
                let base = poles[i].local(add(recv.origin, mul(recv.dir, z0)));
                let axis = poles[i].local_dir(recv.dir);
                let tool = tool_name(name);
                tool_cylinder(s, &tool, base, axis, recv.diameter, z1 - z0)?;
                combine(s, name, &tool, "cut")?;
                for op in poles[i].ops.clone().iter().filter(|o| matches!(o, Op::Tenon { .. })) {
                    apply_op(s, name, &poles[i], op)?;
                }
            } else {
                combine_keep(s, name, target, "cut")?;
            }
            poles[i].ops[k] = Op::Saddle { with: target.clone(), end, extra };
            changed = true;
        }
        if changed && let Some((comp, _)) = comps.get(i) {
            let parent = s.doc.components.iter().find(|c| c.id == *comp).map_or(0, |c| c.parent);
            run(s, "component.activate", json!({"component": parent}))?;
            save_pole(s, *comp, kind, &poles[i])?;
        }
    }
    Ok(())
}

fn save_pole(s: &mut Session, comp: u64, kind: &str, pole: &Pole) -> Result<()> {
    let record = serde_json::to_string(pole).map_err(|x| e(x.to_string()))?;
    let desc = if pole.width > 0.0 {
        format!("{} {} × {} × {} mm", pole.role, pole.diameter, pole.width, pole.length.round())
    } else {
        format!("{} Ø{} × {} mm", pole.role, pole.diameter, pole.length.round())
    };
    run(
        s,
        "component.metadata.set",
        json!({"component": comp, "part_number": part_number(kind, pole), "description": desc,
               "properties": {P_POLE: record, "essence": "bois rond", "longueur_debit_mm": format!("{}", pole.cut_length().round())}}),
    )?;
    Ok(())
}

fn model_params(cmd: &str, m: &models::Model, p: &Value) -> Result<Params> {
    let mut v = Params::of(m);
    let mut take = |src: &Value| -> Result<()> {
        if let Some(o) = src.as_object() {
            for d in m.params {
                if let Some(x) = o.get(d.key).filter(|x| !x.is_null()) {
                    let n = match x {
                        Value::Bool(b) => f64::from(u8::from(*b)),
                        other => other.as_f64().ok_or_else(|| bad(cmd, format!("`{}` must be a number", d.key)))?,
                    };
                    v.set(d.key, n);
                }
            }
        }
        Ok(())
    };
    take(p)?;
    if let Some(inner) = p.get("params") {
        take(inner)?;
    }
    Ok(v)
}

fn rotation_of(cmd: &str, p: &Value) -> Result<f64> {
    let r = num(p, "rotation").unwrap_or(0.0);
    if [0.0, 90.0, 180.0, 270.0].iter().any(|q| (r - q).abs() < 1e-9) { Ok(r) } else { Err(bad(cmd, "`rotation` must be 0, 90, 180 or 270 degrees")) }
}

fn coin_param(s: &Session, p: &Value, cmd: &str) -> Result<Option<u64>> {
    let Some(v) = p.get("coin").filter(|v| !v.is_null()) else { return Ok(None) };
    let c = component_id(s, v, cmd)?;
    if s.doc.components.iter().any(|x| x.id == c && x.properties.contains_key(P_COIN)) { Ok(Some(c)) } else { Err(bad(cmd, "`coin` is not a patrol corner (scout.coin.create)")) }
}

fn create_meuble(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.meuble.create";
    let kind = str_(p, "type").ok_or_else(|| bad(cmd, "`type` is required (scout.meuble.types)"))?.to_string();
    let m = models::model(&kind).ok_or_else(|| bad(cmd, format!("unknown furniture `{kind}` (scout.meuble.types)")))?;
    let params = model_params(cmd, m, p)?;
    let poles = models::build(&kind, &params).map_err(|x| bad(cmd, x))?;
    let rotation = rotation_of(cmd, p)?;
    let at = p.get("at").map(|v| vec2(v).ok_or_else(|| bad(cmd, "`at` is [x, y] in mm"))).transpose()?;
    let coin = coin_param(s, p, cmd)?;
    let detail = bool_p(p, "detail", false);
    let name = free_name(s, str_(p, "name").map(str::trim).filter(|n| !n.is_empty()).unwrap_or(m.name));
    let active = s.active_component;
    let top = run(s, "component.create", json!({"name": name, "parent": coin.unwrap_or(0), "grounded": false}))?;
    let (comp, occ) = (id_of(&top, "component"), id_of(&top, "occurrence"));
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    let mut poles = poles;
    let mut comps: Vec<(u64, String)> = Vec::new();
    for pole in &poles {
        let n = counts.entry(pole.role.clone()).or_insert(0);
        *n += 1;
        let part = format!("{} {} {}", pole.role, n, name);
        let (pc, _) = make_pole(s, comp, &kind, &part, pole, detail)?;
        comps.push((pc, part));
    }
    if detail {
        fit_saddles(s, &kind, &comps, &mut poles)?;
    }
    let (x, y) = at.map_or((0.0, 0.0), |v| (v.x, v.y));
    run(
        s,
        "component.metadata.set",
        json!({"component": comp, "description": m.name, "properties": {P_KIND: kind, P_PARAMS: params.to_json(), P_PLACE: format!("{}|{}|{}", x, y, rotation)}}),
    )?;
    if at.is_some() || rotation != 0.0 {
        let (c, sn) = (rotation.to_radians().cos().round(), rotation.to_radians().sin().round());
        run(s, "occurrence.move", json!({"occurrence": occ, "matrix": [c, -sn, 0.0, x, sn, c, 0.0, y, 0.0, 0.0, 1.0, 0.0], "capture": true, "move_grounded": true}))?;
    }
    run(s, "component.activate", json!({"component": active}))?;
    Ok(json!({
        "component": comp, "occurrence": occ, "name": name, "type": kind, "poles": pole::expand_all(&poles).len(), "wood_m": wood_metres(&poles),
        "cut_list": rows_json(&poles), "joints": joints_json(&poles),
    }))
}

fn rows_json(poles: &[Pole]) -> Vec<Value> {
    let poles = &pole::expand_all(poles);
    cut_list(poles).iter().map(|r| json!({"role": r.role, "count": r.count, "diameter": r.diameter, "width": r.width, "length": r.length, "cut_length": r.cut_length})).collect()
}

fn joints_json(poles: &[Pole]) -> Vec<Value> {
    let poles = &pole::expand_all(poles);
    joint_summary(poles).iter().map(|(k, c)| json!({"joint": k, "count": c})).collect()
}

/// The standard corner: a table with a bench on each long side, a shelving and a prep table.
const STANDARD: &[(&str, f64, f64, f64)] = &[("table", 0.0, 0.0, 0.0), ("banc", 150.0, -400.0, 0.0), ("banc", 150.0, 950.0, 0.0), ("etagere", 2000.0, 0.0, 0.0), ("prepa", 2000.0, 700.0, 0.0)];

fn create_coin(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.coin.create";
    let layout = str_(p, "layout").unwrap_or("standard");
    if !matches!(layout, "standard" | "empty") {
        return Err(bad(cmd, "`layout` is standard or empty"));
    }
    let name = free_name(s, str_(p, "name").map(str::trim).filter(|n| !n.is_empty()).unwrap_or("Coin de patrouille"));
    let active = s.active_component;
    let made = run(s, "component.create", json!({"name": name, "parent": 0, "grounded": false}))?;
    let coin = id_of(&made, "component");
    run(s, "component.metadata.set", json!({"component": coin, "description": "Coin de patrouille", "properties": {P_COIN: "1"}}))?;
    run(s, "component.activate", json!({"component": active}))?;
    let mut placed = Vec::new();
    if layout == "standard" {
        for (kind, x, y, rot) in STANDARD {
            let r = create_meuble(s, &json!({"type": kind, "coin": coin, "at": [x, y], "rotation": rot}))?;
            placed.push(json!({"component": r["component"], "name": r["name"], "type": kind, "at": [x, y], "rotation": rot}));
        }
    }
    Ok(json!({"component": coin, "name": name, "meubles": placed}))
}

/// Cuts the hidden joints (tenons, mortises, peg holes) of the furniture built the quick way.
fn detail_cmd(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.meuble.detail";
    let root = if p.get("meuble").is_some_and(|v| !v.is_null()) { Some(component_id(s, &p["meuble"], cmd)?) } else { None };
    let under = |s: &Session, mut c: u64, root: u64| loop {
        if c == root {
            return true;
        }
        match s.doc.components.iter().find(|k| k.id == c) {
            Some(k) if k.parent != c => c = k.parent,
            _ => return false,
        }
    };
    let todo: Vec<(u64, usize, Pole)> = s
        .doc
        .components
        .iter()
        .filter(|c| root.is_none_or(|r| under(s, c.id, r)))
        .filter_map(|c| {
            let n: usize = c.properties.get(P_FAST)?.parse().ok()?;
            let pole: Pole = serde_json::from_str(c.properties.get(P_POLE)?).ok()?;
            Some((c.id, n, pole))
        })
        .collect();
    let active = s.active_component;
    let mut cuts = 0;
    for (comp, n, pole) in &todo {
        let name = name_of(s, *comp);
        run(s, "component.activate", json!({"component": *comp}))?;
        for op in pole.ops.iter().take(*n).filter(|o| hidden(o) && !matches!(o, Op::Saddle { .. })) {
            apply_op(s, &name, pole, op)?;
            cuts += 1;
        }
        run(s, "component.metadata.set", json!({"component": *comp, "properties": {P_FAST: ""}}))?;
    }
    run(s, "component.activate", json!({"component": active}))?;
    Ok(json!({"pieces": todo.len(), "operations": cuts}))
}

// ---- joints and shaping on poles already in the design ----------------------------------------

/// What changed on a pole between `old` and `new`: the cut that takes off what was trimmed, then the
/// operations added since.
fn new_ops(old: &Pole, new: &Pole) -> Vec<Op> {
    let mut v = Vec::new();
    let saddled = |e: End| new.ops.iter().skip(old.ops.len()).any(|o| matches!(o, Op::Saddle { end, .. } if *end == e));
    if new.start > old.start + 1e-6 && !saddled(End::Start) {
        v.push(Op::CutOff { end: End::Start, at: new.start });
    }
    if new.start + new.length < old.start + old.length - 1e-6 && !saddled(End::End) {
        v.push(Op::CutOff { end: End::End, at: new.start + new.length });
    }
    v.extend(new.ops.iter().skip(old.ops.len()).cloned());
    v
}

/// Brings a pole's body and record up to date with `new`.
fn update_pole(s: &mut Session, comp: u64, old: &Pole, new: &Pole) -> Result<usize> {
    let name = name_of(s, comp);
    let ops = new_ops(old, new);
    run(s, "component.activate", json!({"component": comp}))?;
    for op in &ops {
        apply_op(s, &name, new, op)?;
    }
    let parent = s.doc.components.iter().find(|c| c.id == comp).map_or(0, |c| c.parent);
    run(s, "component.activate", json!({"component": parent}))?;
    let kind = s.doc.components.iter().find(|c| c.id == comp).and_then(|c| c.metadata.get("part_number").and_then(|v| v.as_str().map(str::to_string))).unwrap_or_default();
    let kind = kind.split('-').next().unwrap_or("PIECE").to_string();
    save_pole(s, comp, &kind, new)?;
    Ok(ops.len())
}

/// A new pole (a peg, a lashing ring) as a component beside `beside`.
fn add_pole(s: &mut Session, beside: u64, pole: &Pole) -> Result<String> {
    let parent = s.doc.components.iter().find(|c| c.id == beside).map_or(0, |c| c.parent);
    let name = free_name(s, &format!("{} {}", pole.role, name_of(s, beside)));
    make_pole(s, parent, "AJOUT", &name, pole, true)?;
    Ok(name)
}

/// The recipes of the design, in the order they were made.
fn recipes(s: &Session) -> Vec<Value> {
    s.doc.metadata.get(K_RECIPES).and_then(|v| v.get("recettes")).and_then(Value::as_array).cloned().unwrap_or_default()
}

fn set_recipes(s: &mut Session, list: &[Value]) -> Result<()> {
    run(s, "document.metadata.set", json!({"key": K_RECIPES, "value": {"recettes": list}}))?;
    Ok(())
}

/// Remembers a joint or a shaping just made (`cmd`, its parameters with the pieces by name, the
/// poles it created) and the base of each pole it touched, unless it is itself a replay.
fn record_recipe(s: &mut Session, cmd: &str, p: &Value, touched: &[(&str, u64, &Pole)], created: &[String]) -> Result<()> {
    let mut q = p.clone();
    let mut pieces = Vec::new();
    for (key, id, old) in touched {
        let name = name_of(s, *id);
        q[*key] = json!(name);
        pieces.push(name);
        let has_base = s.doc.components.iter().any(|c| c.id == *id && c.properties.contains_key(P_BASE));
        if !has_base {
            let base = serde_json::to_string(old).map_err(|x| e(x.to_string()))?;
            run(s, "component.metadata.set", json!({"component": *id, "properties": {P_BASE: base}}))?;
        }
    }
    if p.get("_replay").is_some() {
        return Ok(());
    }
    let mut list = recipes(s);
    if list.len() >= 500 {
        return Err(e("plus de 500 assemblages : fusionnez ou simplifiez avant d'en ajouter".into()));
    }
    list.push(json!({"c": cmd, "p": q, "pieces": pieces, "created": created}));
    set_recipes(s, &list)
}

/// Every piece joined to `start` by a recipe, directly or not (and `start`).
fn cluster(list: &[Value], start: &str) -> Vec<String> {
    let mut seen = vec![start.to_string()];
    loop {
        let before = seen.len();
        for r in list {
            let names: Vec<String> = r["pieces"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
            if names.iter().any(|n| seen.contains(n)) {
                for n in names {
                    if !seen.contains(&n) {
                        seen.push(n);
                    }
                }
            }
        }
        if seen.len() == before {
            return seen;
        }
    }
}

/// Rebuilds the pieces `names` from their bases (the first of them changed by `edit`) and does their
/// joints and shapings again: what makes a change to one piece reach every joint that depends on it.
fn rebuild(s: &mut Session, names: &[String], edit: impl Fn(&str, &mut Pole)) -> Result<(usize, Vec<String>)> {
    let list = recipes(s);
    struct Member {
        name: String,
        parent: u64,
        kind: String,
        base: Pole,
        quick: bool,
    }
    let mut members = Vec::new();
    for n in names {
        let c = s.doc.components.iter().find(|c| c.name == *n).ok_or_else(|| e(format!("« {n} » n'existe plus")))?;
        let rec = c.properties.get(P_BASE).or_else(|| c.properties.get(P_POLE)).ok_or_else(|| e(format!("« {n} » n'est pas une perche de SoLeadWorks")))?;
        let mut base: Pole = serde_json::from_str(rec).map_err(|x| e(x.to_string()))?;
        edit(n, &mut base);
        let kind = c.metadata.get("part_number").and_then(|v| v.as_str()).and_then(|t| t.split('-').next()).unwrap_or("PIECE").to_string();
        members.push(Member { name: n.clone(), parent: c.parent, kind, base, quick: c.properties.contains_key(P_FAST) });
    }
    // the poles those joints made (pegs, lashings) go too: the replay makes them again
    let touching: Vec<&Value> = list.iter().filter(|r| r["pieces"].as_array().is_some_and(|a| a.iter().any(|v| v.as_str().is_some_and(|x| names.iter().any(|n| n == x))))).collect();
    let mut gone: Vec<String> = touching.iter().flat_map(|r| r["created"].as_array().cloned().unwrap_or_default()).filter_map(|v| v.as_str().map(str::to_string)).collect();
    gone.extend(names.iter().cloned());
    for n in &gone {
        if let Some(id) = s.doc.components.iter().find(|c| c.name == *n).map(|c| c.id) {
            run(s, "component.delete", json!({"component": id}))?;
        }
    }
    for m in &members {
        let (comp, _) = make_pole(s, m.parent, &m.kind, &m.name, &m.base, !m.quick)?;
        // the base stays what it was, with the change
        let rec = serde_json::to_string(&m.base).map_err(|x| e(x.to_string()))?;
        run(s, "component.metadata.set", json!({"component": comp, "properties": {P_BASE: rec}}))?;
    }
    let mut failed = Vec::new();
    let mut fresh = Vec::new();
    for r in &touching {
        let mut q = r["p"].clone();
        q["_replay"] = json!(true);
        let result = match r["c"].as_str() {
            Some("assemblage") => assemblage(s, &q),
            Some("faconner") => faconner(s, &q),
            _ => continue,
        };
        match result {
            Ok(v) => fresh.push(((*r).clone(), v)),
            Err(x) => failed.push(format!("{} : {x}", r["p"]["kind"].as_str().unwrap_or("assemblage"))),
        }
    }
    // the recipes keep the new names of what they created
    let mut all = list.clone();
    for (old, v) in &fresh {
        if let Some(slot) = all.iter_mut().find(|r| *r == old) {
            slot["created"] = v["created"].clone();
        }
    }
    set_recipes(s, &all)?;
    Ok((members.len(), failed))
}

/// Changes one pole (its diameter, its length, its place) and redoes every joint that depends on it.
fn modifier(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.piece.modifier";
    let (id, pole) = piece(s, p, "piece", cmd)?;
    let name = name_of(s, id);
    let d = bounded(cmd, p, "diameter", pole.diameter, 10.0, 600.0)?;
    let len = bounded(cmd, p, "length", pole.length, 20.0, 6000.0)?;
    let dx = bounded(cmd, p, "dx", 0.0, -10_000.0, 10_000.0)?;
    let dy = bounded(cmd, p, "dy", 0.0, -10_000.0, 10_000.0)?;
    let dz = bounded(cmd, p, "dz", 0.0, -10_000.0, 10_000.0)?;
    let names = cluster(&recipes(s), &name);
    let (count, failed) = rebuild(s, &names, |n, b| {
        if n == name {
            b.diameter = d;
            if p.get("length").is_some_and(|v| !v.is_null()) {
                b.length = len;
            }
            b.origin = add(b.origin, [dx, dy, dz]);
        }
    })?;
    Ok(json!({"piece": name, "rebuilt": count, "pieces": names, "failed": failed}))
}

/// Changes the settings of a furniture (its length, its leg diameter…) and builds it again in the
/// same place, then redoes the joints and shapings made on its pieces since.
fn meuble_modifier(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.meuble.modifier";
    let id = pick(s, p, "meuble", P_KIND, "furniture", cmd)?;
    let c = s.doc.components.iter().find(|c| c.id == id).ok_or_else(|| bad(cmd, "no such furniture"))?;
    let (kind, name) = (c.properties.get(P_KIND).cloned().unwrap_or_default(), c.name.clone());
    let parent = c.parent;
    let m = models::model(&kind).ok_or_else(|| bad(cmd, format!("unknown furniture `{kind}`")))?;
    let mut merged: serde_json::Map<String, Value> = c.properties.get(P_PARAMS).and_then(|t| serde_json::from_str(t).ok()).unwrap_or_default();
    for src in [Some(p), p.get("params")].into_iter().flatten() {
        if let Some(o) = src.as_object() {
            for d in m.params {
                if let Some(x) = o.get(d.key).filter(|x| !x.is_null()) {
                    merged.insert(d.key.to_string(), x.clone());
                }
            }
        }
    }
    let place: Vec<f64> = c.properties.get(P_PLACE).map(|t| t.split('|').filter_map(|x| x.parse().ok()).collect()).unwrap_or_default();
    let (x, y, rot) = match place.as_slice() {
        [x, y, r] => (*x, *y, *r),
        _ => (0.0, 0.0, 0.0),
    };
    let coin = s.doc.components.iter().any(|k| k.id == parent && k.properties.contains_key(P_COIN)).then_some(parent);
    let detail = !s.doc.components.iter().any(|k| k.parent == id && k.properties.contains_key(P_FAST));
    let old_names: Vec<String> = s.doc.components.iter().filter(|k| k.parent == id).map(|k| k.name.clone()).collect();
    run(s, "component.delete", json!({"component": id}))?;
    let mut q = json!({"type": kind, "name": name, "at": [x, y], "rotation": rot, "params": merged, "detail": detail});
    if let Some(cn) = coin {
        q["coin"] = json!(cn);
    }
    let made = create_meuble(s, &q)?;
    // what the user joined on its pieces since: done again wherever the pieces still exist
    let mut failed = Vec::new();
    let mut redone = 0;
    for r in recipes(s) {
        let pieces: Vec<String> = r["pieces"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
        if !pieces.iter().any(|n| old_names.contains(n)) {
            continue;
        }
        if !pieces.iter().all(|n| s.doc.components.iter().any(|c| c.name == *n)) {
            failed.push(format!("{} : une pièce n'existe plus dans le nouveau meuble", r["p"]["kind"].as_str().unwrap_or("assemblage")));
            continue;
        }
        let mut qq = r["p"].clone();
        qq["_replay"] = json!(true);
        let result = match r["c"].as_str() {
            Some("assemblage") => assemblage(s, &qq),
            Some("faconner") => faconner(s, &qq),
            _ => continue,
        };
        match result {
            Ok(_) => redone += 1,
            Err(x) => failed.push(format!("{} : {x}", r["p"]["kind"].as_str().unwrap_or("assemblage"))),
        }
    }
    Ok(json!({"meuble": made["component"], "name": name, "poles": made["poles"], "redone": redone, "failed": failed}))
}

fn style_of(kind: &str) -> Option<(Style, bool)> {
    Some(match kind {
        "hache" => (Style::Hache, false),
        "scie" => (Style::Scie, false),
        "plat" | "platscie" => (Style::Plat, false),
        "aronde1" | "aronde2" | "aronde3" => (Style::Aronde, false),
        _ => return None,
    })
}

fn assemblage(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.assemblage";
    let kind = str_(p, "kind").ok_or_else(|| bad(cmd, "`kind` is required (mibois, hache, scie, plat, platscie, aronde1, aronde2, aronde3, cheville, brelage)"))?;
    let (ia, a) = piece(s, p, "a", cmd)?;
    let (ib, b) = piece(s, p, "b", cmd)?;
    if ia == ib {
        return Err(bad(cmd, "choisissez deux pièces différentes"));
    }
    let (oa, ob) = (a.clone(), b.clone());
    let mut build = Build { poles: vec![a, b] };
    let d = num(p, "d").unwrap_or(0.0);
    let result = match kind {
        "mibois" => build.mibois(0, 1, num(p, "pc").unwrap_or(50.0)).and_then(|()| {
            if p.get("chev").and_then(Value::as_bool).unwrap_or(false) { build.peg(0, 1, if d > 0.0 { d } else { 20.0 }).map(|_| ()) } else { Ok(()) }
        }),
        "cheville" => build.peg(0, 1, if d > 0.0 { d } else { 20.0 }).map(|_| ()),
        "brelage" => build.lash(0, 1, num(p, "turns").unwrap_or(6.0), num(p, "rope").unwrap_or(6.0)),
        other => match style_of(other) {
            Some((style, through)) => {
                let mut o = TenonOpts::new(style);
                o.d = d;
                o.w = num(p, "w").unwrap_or(0.0);
                o.theta = num(p, "theta").unwrap_or(if style == Style::Aronde { 20.0 } else { 0.0 });
                o.through = through || p.get("through").and_then(Value::as_bool).unwrap_or(false);
                o.saddle = p.get("gueule").and_then(Value::as_bool).unwrap_or(false);
                o.typ = match other {
                    "aronde2" => 2,
                    "aronde3" => 3,
                    _ => 1,
                };
                build.tenon(0, 1, o)
            }
            None => return Err(bad(cmd, format!("unknown kind `{other}`"))),
        },
    };
    result.map_err(|m| bad(cmd, m))?;
    // `#1` and `#0` in a saddle name the two poles of the joint
    let names = [name_of(s, ia), name_of(s, ib)];
    for pole in &mut build.poles[..2] {
        for op in &mut pole.ops {
            if let Op::Saddle { with, .. } = op
                && let Some(k) = with.strip_prefix('#').and_then(|n| n.parse::<usize>().ok())
                && let Some(n) = names.get(k)
            {
                *with = n.clone();
            }
        }
    }
    // the receiving pole first: its mortise must exist before the other pole is cut to fit it
    let mut changed = update_pole(s, ib, &ob, &build.poles[1])?;
    changed += update_pole(s, ia, &oa, &build.poles[0])?;
    let mut created = Vec::new();
    for extra in build.poles.iter().skip(2) {
        created.push(add_pole(s, ia, extra)?);
    }
    record_recipe(s, "assemblage", p, &[("a", ia, &oa), ("b", ib, &ob)], &created)?;
    Ok(json!({"kind": kind, "a": name_of(s, ia), "b": name_of(s, ib), "changed": changed, "created": created}))
}

fn faconner(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.faconner";
    let kind = str_(p, "kind").ok_or_else(|| bad(cmd, "`kind` is required (demi, quart, couper, pointe)"))?;
    let (id, old) = piece(s, p, "piece", cmd)?;
    let mut new = old.clone();
    if matches!(kind, "demi" | "quart") && old.ops.iter().any(|o| matches!(o, Op::Split { .. })) {
        return Err(bad(cmd, format!("« {} » est déjà fendue", name_of(s, id))));
    }
    match kind {
        "demi" => new.push(Op::Split { mode: SplitMode::Demi }),
        "quart" => new.push(Op::Split { mode: SplitMode::Quart }),
        "couper" => {
            let l = num(p, "length").ok_or_else(|| bad(cmd, "`length` is required: the new length in mm"))?;
            if !(10.0..old.length).contains(&l) {
                return Err(bad(cmd, format!("`length` must be between 10 and {} mm (shorter than the pole)", old.length.round())));
            }
            new.length = l;
            Ok(())
        }
        "pointe" => {
            let l = num(p, "len").unwrap_or(150.0);
            if !(5.0..old.length).contains(&l) {
                return Err(bad(cmd, "`len`: the point must be shorter than the pole"));
            }
            let end = match str_(p, "end") {
                Some("start") => End::Start,
                _ => End::End,
            };
            new.push(Op::Point { end, len: l })
        }
        other => return Err(bad(cmd, format!("unknown kind `{other}`"))),
    }
    .map_err(|m| bad(cmd, m))?;
    let changed = update_pole(s, id, &old, &new)?;
    record_recipe(s, "faconner", p, &[("piece", id, &old)], &[])?;
    Ok(json!({"kind": kind, "piece": name_of(s, id), "changed": changed, "length": new.length}))
}

fn loose(s: &mut Session, name: &str, kind: &str, poles: &[Pole], at: Option<V3>) -> Result<Value> {
    if poles.len() > MAX_POLES {
        return Err(e(format!("{} pièces : plus que les {MAX_POLES} permises", poles.len())));
    }
    let name = free_name(s, name);
    let active = s.active_component;
    let top = run(s, "component.create", json!({"name": name, "parent": 0, "grounded": false}))?;
    let (comp, occ) = (id_of(&top, "component"), id_of(&top, "occurrence"));
    for (i, pole) in poles.iter().enumerate() {
        make_pole(s, comp, kind, &format!("{} {} {}", pole.role, i + 1, name), pole, true)?;
    }
    if let Some(a) = at {
        run(s, "occurrence.move", json!({"occurrence": occ, "matrix": [1.0, 0.0, 0.0, a[0], 0.0, 1.0, 0.0, a[1], 0.0, 0.0, 1.0, a[2]], "capture": true, "move_grounded": true}))?;
    }
    run(s, "component.activate", json!({"component": active}))?;
    Ok(json!({"component": comp, "name": name, "poles": pole::expand_all(poles).len()}))
}

fn bounded(cmd: &str, p: &Value, key: &str, default: f64, lo: f64, hi: f64) -> Result<f64> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => v.as_f64().filter(|x| x.is_finite() && (lo..=hi).contains(x)).ok_or_else(|| bad(cmd, format!("`{key}` must be a number between {lo} and {hi}"))),
    }
}

fn at_of(cmd: &str, p: &Value) -> Result<Option<V3>> {
    match p.get("at") {
        None | Some(Value::Null) => Ok(None),
        Some(v) => match (vec3(v), vec2(v)) {
            (Some(q), _) => Ok(Some([q.x, q.y, q.z])),
            (None, Some(q)) => Ok(Some([q.x, q.y, 0.0])),
            _ => Err(bad(cmd, "`at` is [x, y] or [x, y, z] in mm")),
        },
    }
}

fn piece_traverse(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.piece.traverse";
    let d = bounded(cmd, p, "diameter", 80.0, 15.0, 400.0)?;
    let l = bounded(cmd, p, "length", 1500.0, 50.0, 6000.0)?;
    let dir = match p.get("dir") {
        None | Some(Value::Null) => [1.0, 0.0, 0.0],
        Some(v) => vec3(v).map(|q| [q.x, q.y, q.z]).filter(|q| pole::len(*q) > 1e-6).ok_or_else(|| bad(cmd, "`dir` is [x, y, z], not zero"))?,
    };
    let at = at_of(cmd, p)?.unwrap_or([0.0, 0.0, d / 2.0]);
    let q = pole::add(at, pole::mul(pole::norm(dir), l));
    let pole = Pole::bar("traverse", d, at, q);
    let name = str_(p, "name").map(str::trim).filter(|n| !n.is_empty()).unwrap_or("Traverse");
    let name = free_name(s, name);
    let active = s.active_component;
    let (comp, _) = make_pole(s, 0, "TRAVERSE", &name, &pole, true)?;
    run(s, "component.activate", json!({"component": active}))?;
    Ok(json!({"component": comp, "name": name}))
}

fn piece_equerre(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.piece.equerre";
    let k = bounded(cmd, p, "unit", 300.0, 50.0, 1000.0)?;
    let d = bounded(cmd, p, "d", 50.0, 15.0, 150.0)?;
    let at = at_of(cmd, p)?.unwrap_or([0.0; 3]);
    let r = d / 2.0;
    let (a_len, b_len) = (3.0 * k, 4.0 * k);
    let o = [at[0], at[1], at[2] + r];
    let mut b = Build::default();
    // two poles half-lapped at the right angle, the hypotenuse lying on both ends, lashed
    let a = b.bar("côté 3", d, pole::add(o, [-r, 0.0, 0.0]), pole::add(o, [a_len, 0.0, 0.0]));
    let c = b.bar("côté 4", d, pole::add(o, [0.0, -r, r]), pole::add(o, [0.0, b_len, r]));
    let h = b.bar("hypoténuse", d, pole::add(o, [a_len, 0.0, 2.0 * r]), pole::add(o, [0.0, b_len, 2.0 * r]));
    b.mibois(a, c, 50.0).map_err(|m| bad(cmd, m))?;
    b.lash(h, a, 6.0, 6.0).map_err(|m| bad(cmd, m))?;
    b.lash(h, c, 6.0, 6.0).map_err(|m| bad(cmd, m))?;
    loose(s, "Équerre", "EQUERRE", &b.poles, None)
}

/// A plateau: two rails along x and the fill between them, in one of CathIA's forms.
fn piece_plateau(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.plateau";
    let mode = match str_(p, "mode") {
        None => models::Fill::Tresse,
        Some(k) => models::Fill::from_key(k).ok_or_else(|| bad(cmd, "`mode` is tresse, droit, filet or quarts"))?,
    };
    let (l, w) = (bounded(cmd, p, "long", 1000.0, 100.0, 5000.0)?, bounded(cmd, p, "larg", 600.0, 100.0, 5000.0)?);
    let o = models::FillOpts {
        dc: bounded(cmd, p, "dc", 20.0, 8.0, 80.0)?,
        maille: bounded(cmd, p, "maille", 30.0, 10.0, 300.0)?.max((l + w) / 160.0),
        rail: bounded(cmd, p, "dr", 40.0, 20.0, 150.0)?,
        c0: bounded(cmd, p, "c0", 0.0, 0.0, 4900.0)?,
        cl: bounded(cmd, p, "cl", 0.0, 0.0, 5000.0)?,
    };
    let z = bounded(cmd, p, "z", 0.0, 0.0, 3000.0)?;
    let at = at_of(cmd, p)?.unwrap_or([0.0; 3]);
    let mut b = Build::default();
    let zr = z + o.rail / 2.0;
    for y in [at[1], at[1] + w] {
        b.bar("lisse", o.rail, [at[0], y, zr], [at[0] + l, y, zr]);
    }
    models::fill(&mut b, mode, (at[0], at[0] + l), (at[1], at[1] + w), zr + o.rail / 2.0, o).map_err(|x| bad(cmd, x))?;
    loose(s, "Plateau", "PLATEAU", &b.poles, None)
}

// ---- reading back, plans and the HTML page -----------------------------------------------------

/// A furniture read back from the design.
struct Meuble {
    id: u64,
    name: String,
    kind: String,
    place: (f64, f64, f64),
    poles: Vec<Pole>,
}

fn read_meuble(s: &Session, id: u64) -> Option<Meuble> {
    let c = s.doc.components.iter().find(|c| c.id == id)?;
    let kind = c.properties.get(P_KIND)?.clone();
    let pv: Vec<f64> = c.properties.get(P_PLACE).map(|t| t.split('|').filter_map(|x| x.parse().ok()).collect()).unwrap_or_default();
    let place = match pv.as_slice() {
        [x, y, r] => (*x, *y, *r),
        _ => (0.0, 0.0, 0.0),
    };
    let poles = s.doc.components.iter().filter(|k| k.parent == id).filter_map(|k| k.properties.get(P_POLE)).filter_map(|rec| serde_json::from_str::<Pole>(rec).ok()).flat_map(|p| p.expand()).collect();
    Some(Meuble { id, name: c.name.clone(), kind, place, poles })
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
    str_(p, "author").map(str::to_string).or_else(|| s.doc.metadata.get("author").and_then(|v| v.as_str().map(str::to_string))).unwrap_or_else(|| "—".into())
}

fn write_text(path: &str, ext: &str, text: &str) -> Result<()> {
    if !path.to_ascii_lowercase().ends_with(ext) {
        return Err(e(format!("{path}: write it as a {ext} file")));
    }
    solidcraft_io::vfs::write(path, text.as_bytes()).map_err(|x| e(format!("{path}: {x}")))
}

fn model_name(kind: &str) -> String {
    models::model(kind).map_or(kind.to_string(), |m| m.name.to_string())
}

fn placed_of(m: &Meuble) -> Placed {
    let x = extent(&m.poles);
    Placed { name: m.name.clone(), kind: model_name(&m.kind), x: m.place.0, y: m.place.1, length: x[0], width: x[1], rotation: m.place.2, wood_m: wood_metres(&m.poles) }
}

fn coin_meubles(s: &Session, coin: u64) -> Vec<Meuble> {
    s.doc.components.iter().filter(|c| c.parent == coin && c.properties.contains_key(P_KIND)).map(|c| c.id).collect::<Vec<_>>().into_iter().filter_map(|id| read_meuble(s, id)).collect()
}

fn bool_p(p: &Value, k: &str, default: bool) -> bool {
    p.get(k).and_then(Value::as_bool).unwrap_or(default)
}

fn text_p(p: &Value, k: &str) -> String {
    str_(p, k).map(|x| x.trim().chars().take(120).collect()).unwrap_or_default()
}

fn sheet_cfg(s: &Session, p: &Value, titre: &str) -> Result<SheetCfg> {
    let cmd = "scout.atelier.dessin";
    let mut cfg = SheetCfg::standard(titre, &author(s, p));
    cfg.format = match str_(p, "format") {
        None | Some("A3") => Format::A3,
        Some("A4") => Format::A4,
        Some(_) => return Err(bad(cmd, "`format` is A3 or A4")),
    };
    cfg.echelle = bounded(cmd, p, "echelle", 0.0, 0.0, 1000.0)?;
    if let Some(list) = p.get("vues").filter(|v| !v.is_null()) {
        let arr = list.as_array().ok_or_else(|| bad(cmd, "`vues` is a list: face, arriere, droite, gauche, dessus, dessous, iso"))?;
        let views: Vec<View> = arr.iter().filter_map(|v| v.as_str().and_then(View::parse)).collect();
        if views.len() != arr.len() || views.is_empty() || views.len() > 7 {
            return Err(bad(cmd, "`vues`: one to seven of face, arriere, droite, gauche, dessus, dessous, iso"));
        }
        cfg.vues = views;
    }
    cfg.cotes_ensemble = bool_p(p, "cotes", true);
    cfg.cotes_pieces = bool_p(p, "cotes_pieces", false);
    cfg.tableau = bool_p(p, "tableau", true);
    let c = &mut cfg.cartouche;
    *c = Cartouche {
        titre: if text_p(p, "titre").is_empty() { titre.to_string() } else { text_p(p, "titre") },
        societe: text_p(p, "societe"),
        dessinateur: if text_p(p, "dessinateur").is_empty() { c.dessinateur.clone() } else { text_p(p, "dessinateur") },
        date: text_p(p, "date"),
        materiau: if str_(p, "materiau").is_some() { text_p(p, "materiau") } else { c.materiau.clone() },
        reference: text_p(p, "reference"),
        indice: text_p(p, "indice"),
    };
    Ok(cfg)
}

fn dessin_cmd(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.atelier.dessin";
    let has = |prop: &str| s.doc.components.iter().filter(|c| c.properties.contains_key(prop)).count();
    let coin_wanted = p.get("coin").is_some_and(|v| !v.is_null()) || (p.get("meuble").is_none_or(Value::is_null) && has(P_KIND) == 0 && has(P_COIN) == 1);
    if coin_wanted {
        let coin = pick(s, p, "coin", P_COIN, "patrol corner", cmd)?;
        let name = name_of(s, coin);
        let placed: Vec<Placed> = coin_meubles(s, coin).iter().map(placed_of).collect();
        let cfg = sheet_cfg(s, p, &name)?;
        let sheet = dessin::sheet_coin(&placed, &cfg, (1, 1 + placed.len()));
        let svg = dessin::svg(&sheet);
        if let Some(path) = str_(p, "path") {
            write_text(path, ".svg", &svg)?;
        }
        return Ok(json!({"name": name, "sujet": "coin", "svg": svg, "sheet": {"w": sheet.w, "h": sheet.h, "echelle": sheet.den}, "prims": sheet.prims, "meubles": placed.iter().map(|x| x.name.clone()).collect::<Vec<_>>()}));
    }
    let id = pick(s, p, "meuble", P_KIND, "furniture", cmd)?;
    let m = read_meuble(s, id).ok_or_else(|| bad(cmd, "this furniture's record is unreadable"))?;
    let mut cfg = sheet_cfg(s, p, &m.name)?;
    if cfg.cartouche.reference.is_empty() {
        cfg.cartouche.reference = model_name(&m.kind);
    }
    let sheet = dessin::sheet(&m.poles, &cfg, (1, 1));
    let svg = dessin::svg(&sheet);
    if let Some(path) = str_(p, "path") {
        write_text(path, ".svg", &svg)?;
    }
    let x = extent(&m.poles);
    Ok(json!({
        "meuble": m.id, "name": m.name, "type": m.kind, "sujet": "meuble", "svg": svg,
        "sheet": {"w": sheet.w, "h": sheet.h, "echelle": sheet.den}, "prims": sheet.prims,
        "dimensions": {"length": x[0], "width": x[1], "height": x[2]},
        "wood_m": wood_metres(&m.poles), "cut_list": rows_json(&m.poles), "joints": joints_json(&m.poles), "steps": steps(&m.poles),
    }))
}

fn all_poles(s: &Session, root: Option<u64>) -> Vec<Pole> {
    let under = |mut c: u64, root: u64| loop {
        if c == root {
            return true;
        }
        match s.doc.components.iter().find(|k| k.id == c) {
            Some(k) if k.parent != c => c = k.parent,
            _ => return false,
        }
    };
    s.doc.components.iter().filter(|c| root.is_none_or(|r| under(c.id, r))).filter_map(|c| c.properties.get(P_POLE)).filter_map(|t| serde_json::from_str::<Pole>(t).ok()).flat_map(|p| p.expand()).collect()
}

fn nomenclature_cmd(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.atelier.nomenclature";
    let root = if p.get("coin").is_some_and(|v| !v.is_null()) {
        Some(pick(s, p, "coin", P_COIN, "patrol corner", cmd)?)
    } else if p.get("meuble").is_some_and(|v| !v.is_null()) {
        Some(pick(s, p, "meuble", P_KIND, "furniture", cmd)?)
    } else {
        None
    };
    let poles = all_poles(s, root);
    if poles.is_empty() {
        return Err(bad(cmd, "the design has no pieces yet (Créer tab)"));
    }
    let n = nomenclature::of(&poles);
    let csv = nomenclature::csv(&n);
    if let Some(path) = str_(p, "path") {
        write_text(path, ".csv", &csv)?;
    }
    Ok(json!({"pieces": n.pieces, "assemblages": n.assemblages, "materiel": n.materiel, "csv": csv, "poles": poles.len()}))
}

fn piece_profil(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "scout.profil";
    let kind = str_(p, "kind").unwrap_or("tube_carre");
    let size = bounded(cmd, p, "size", 40.0, 10.0, 300.0)?;
    let t = bounded(cmd, p, "epaisseur", 3.0, 0.5, 40.0)?;
    if 2.0 * t >= size {
        return Err(bad(cmd, "`epaisseur` is too thick for `size`"));
    }
    let l = bounded(cmd, p, "length", 1000.0, 20.0, 6000.0)?;
    let dir = match p.get("dir") {
        None | Some(Value::Null) => [1.0, 0.0, 0.0],
        Some(Value::String(a)) => match a.as_str() {
            "X" => [1.0, 0.0, 0.0],
            "Y" => [0.0, 1.0, 0.0],
            "Z" => [0.0, 0.0, 1.0],
            _ => return Err(bad(cmd, "`dir` is X, Y, Z or [x, y, z]")),
        },
        Some(v) => vec3(v).map(|q| [q.x, q.y, q.z]).filter(|q| pole::len(*q) > 1e-6).ok_or_else(|| bad(cmd, "`dir` is X, Y, Z or [x, y, z], not zero"))?,
    };
    let at = at_of(cmd, p)?.unwrap_or([0.0, 0.0, size / 2.0]);
    let label = match kind {
        "tube_rond" => "Tube rond",
        "tube_carre" => "Tube carré",
        "corniere" => "Cornière",
        "u" => "Profilé en U",
        "i" => "Poutre en I",
        _ => return Err(bad(cmd, "`kind` is tube_rond, tube_carre, corniere, u or i")),
    };
    let name = free_name(s, str_(p, "name").map(str::trim).filter(|n| !n.is_empty()).unwrap_or(label));
    let active = s.active_component;
    let c = run(s, "component.create", json!({"name": name, "parent": 0, "grounded": false}))?;
    let (comp, occ) = (id_of(&c, "component"), id_of(&c, "occurrence"));
    let h = size / 2.0;
    let target = json!([name.clone()]);
    let bx = |s: &mut Session, corner: V3, sz: V3, op: &str| -> Result<()> {
        let mut q = json!({"corner": corner, "length": sz[0], "width": sz[1], "height": sz[2]});
        if op == "new" {
            q["body_name"] = json!(name);
        } else {
            q["operation"] = json!(op);
            q["targets"] = target.clone();
        }
        run(s, "solid.box", q)?;
        Ok(())
    };
    match kind {
        "tube_rond" => {
            run(s, "solid.cylinder", json!({"base": [0.0, 0.0, 0.0], "axis": [0.0, 0.0, 1.0], "diameter": size, "height": l, "body_name": name}))?;
            run(s, "solid.cylinder", json!({"base": [0.0, 0.0, -1.0], "axis": [0.0, 0.0, 1.0], "diameter": size - 2.0 * t, "height": l + 2.0, "operation": "cut", "targets": target}))?;
        }
        "tube_carre" => {
            bx(s, [-h, -h, 0.0], [size, size, l], "new")?;
            bx(s, [-h + t, -h + t, -1.0], [size - 2.0 * t, size - 2.0 * t, l + 2.0], "cut")?;
        }
        "corniere" => {
            bx(s, [-h, -h, 0.0], [size, t, l], "new")?;
            bx(s, [-h, -h, 0.0], [t, size, l], "join")?;
        }
        "u" => {
            bx(s, [-h, -h, 0.0], [size, t, l], "new")?;
            bx(s, [-h, -h, 0.0], [t, size, l], "join")?;
            bx(s, [h - t, -h, 0.0], [t, size, l], "join")?;
        }
        _ => {
            bx(s, [-h, -h, 0.0], [size, t, l], "new")?;
            bx(s, [-h, h - t, 0.0], [size, t, l], "join")?;
            bx(s, [-t / 2.0, -h, 0.0], [t, size, l], "join")?;
        }
    }
    let pole = Pole::bar("profilé", size, at, pole::add(at, pole::mul(pole::norm(dir), l)));
    run(s, "occurrence.move", json!({"occurrence": occ, "matrix": pole.matrix(), "capture": true, "move_grounded": true}))?;
    run(s, "component.metadata.set", json!({"component": comp, "part_number": format!("PROFIL-{}-{}x{}", kind.to_uppercase(), size, l), "description": format!("{label} {size} × {l} mm, paroi {t}"), "properties": {"essence": "métal"}}))?;
    run(s, "component.activate", json!({"component": active}))?;
    Ok(json!({"component": comp, "name": name}))
}

fn export_html(s: &mut Session, p: &Value) -> Result<Value> {
    let who = author(s, p);
    let mut sections = Vec::new();
    let (mut coins, mut count) = (0usize, 0usize);
    let coin_ids: Vec<u64> = s.doc.components.iter().filter(|c| c.properties.contains_key(P_COIN)).map(|c| c.id).collect();
    for coin in coin_ids {
        let name = name_of(s, coin);
        let ms = coin_meubles(s, coin);
        let placed: Vec<Placed> = ms.iter().map(placed_of).collect();
        let esc = name.replace('&', "&amp;").replace('<', "&lt;");
        sections.push(format!("<section class=\"meuble\"><h2>{esc}</h2><p class=\"sub\">Coin de patrouille · {} meubles</p>{}</section>\n", ms.len(), dessin::svg(&dessin::sheet_coin(&placed, &SheetCfg::standard(&name, &who), (1, 1)))));
        for m in &ms {
            sections.push(meuble_section(&m.name, &model_name(&m.kind), &who, &m.poles));
            count += 1;
        }
        coins += 1;
    }
    let loose_ids: Vec<u64> = s
        .doc
        .components
        .iter()
        .filter(|c| c.properties.contains_key(P_KIND) && !s.doc.components.iter().any(|k| k.id == c.parent && k.properties.contains_key(P_COIN)))
        .map(|c| c.id)
        .collect();
    for id in loose_ids {
        if let Some(m) = read_meuble(s, id) {
            sections.push(meuble_section(&m.name, &model_name(&m.kind), &who, &m.poles));
            count += 1;
        }
    }
    if count == 0 {
        return Err(bad("scout.export.html", "the design has no furniture yet (Scout tab)"));
    }
    let title = str_(p, "title").map(str::trim).filter(|x| !x.is_empty()).unwrap_or("Plans du camp").to_string();
    let html = page_html(&title, &sections);
    if let Some(path) = str_(p, "path") {
        write_text(path, ".html", &html)?;
    }
    Ok(json!({"html": html, "meubles": count, "coins": coins}))
}

#[cfg(test)]
#[path = "scout_tests.rs"]
mod tests;
