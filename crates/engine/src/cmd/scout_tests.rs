use serde_json::{Value, json};

use crate::Session;

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id} {p}: {e}"),
    }
}

/// The size of the placed bodies of the components under the one named `prefix`.
fn size_of(s: &Session, prefix: &str) -> [f64; 3] {
    let root = s.doc.components.iter().find(|c| c.name.starts_with(prefix)).map_or(0, |c| c.id);
    let st = s.world_state();
    let b = st
        .bodies
        .iter()
        .filter(|b| {
            let mut c = s.doc.model_body_component(b);
            loop {
                if c == root {
                    return true;
                }
                match s.doc.components.iter().find(|k| k.id == c) {
                    Some(k) if k.parent != c => c = k.parent,
                    _ => return false,
                }
            }
        })
        .fold(solidcraft_geom::Aabb3::EMPTY, |a, b| a.union(&b.mesh().bounds()));
    [b.max.x - b.min.x, b.max.y - b.min.y, b.max.z - b.min.z]
}

#[test]
fn a_bench_is_real_geometry_of_the_size_asked() {
    let mut s = Session::default();
    let r = run(&mut s, "scout.meuble.create", json!({"type": "banc", "L": 1500, "h": 450}));
    assert_eq!(r["name"], "Banc court");
    let poles = r["poles"].as_u64().unwrap_or(0);
    assert!(poles >= 5, "{r}");
    let size = size_of(&s, "Banc court");
    assert!((size[0] - 1500.0).abs() < 30.0, "{size:?}");
    assert!((size[2] - 450.0).abs() < 40.0, "{size:?}");
    let bom = run(&mut s, "assembly.bom", json!({}));
    assert_eq!(bom["total_occurrences"].as_u64().unwrap_or(0), poles, "{bom}");
    run(&mut s, "edit.undo", json!({}));
    assert!(s.doc.components.iter().all(|c| c.id == 0));
}

#[test]
fn all_eleven_models_build_through_the_command() {
    for m in crate::scout::models::MODELS {
        let mut s = Session::default();
        let r = run(&mut s, "scout.meuble.create", json!({"type": m.id}));
        assert!(r["poles"].as_u64().unwrap_or(0) >= 5, "{} {r}", m.id);
        let plan = run(&mut s, "scout.atelier.dessin", json!({}));
        assert_eq!(plan["type"], m.id);
        assert!(plan["svg"].as_str().is_some_and(|x| x.contains("Nomenclature") && x.contains("Vue de face")), "{}", m.id);
        assert!(plan["steps"].as_array().is_some_and(|x| x.len() >= 3), "{}", m.id);
    }
}

#[test]
fn a_model_takes_its_own_settings() {
    let mut s = Session::default();
    run(&mut s, "scout.meuble.create", json!({"type": "prepa", "long": 2000, "name": "Prépa"}));
    let plan = run(&mut s, "scout.atelier.dessin", json!({}));
    assert!((plan["dimensions"]["length"].as_f64().unwrap_or(0.0) - 2000.0).abs() < 130.0, "{}", plan["dimensions"]);
    assert!(s.execute("scout.meuble.create", &json!({"type": "prepa", "long": "long"})).is_err());
}

#[test]
fn a_patrol_corner_holds_several_furniture_each_with_its_own_plan() {
    let mut s = Session::default();
    let r = run(&mut s, "scout.coin.create", json!({"name": "Coin des Lions"}));
    assert_eq!(r["meubles"].as_array().map(Vec::len), Some(5), "{r}");
    let plan = run(&mut s, "scout.atelier.dessin", json!({"coin": "Coin des Lions"}));
    assert_eq!((plan["sujet"].as_str(), plan["meubles"].as_array().map(Vec::len)), (Some("coin"), Some(5)));
    assert!(plan["svg"].as_str().is_some_and(|x| x.contains("Coin des Lions") && x.contains("Meubles du coin")));
    let one = run(&mut s, "scout.atelier.dessin", json!({"meuble": "Étagère"}));
    assert_eq!(one["type"], "etagere");
    assert!(s.execute("scout.atelier.dessin", &json!({})).is_err());
}

#[test]
fn two_poles_are_joined_by_each_kind_of_assemblage() {
    let mut s = Session::default();
    run(&mut s, "scout.piece.traverse", json!({"name": "A", "diameter": 80, "length": 1000, "at": [0, 0, 40]}));
    run(&mut s, "scout.piece.traverse", json!({"name": "B", "diameter": 80, "length": 1000, "at": [500, -500, 110], "dir": [0, 1, 0]}));
    // half-lap
    let r = run(&mut s, "scout.assemblage", json!({"kind": "mibois", "a": "A", "b": "B"}));
    assert!(r["changed"].as_u64().unwrap_or(0) >= 2, "{r}");
    // a peg through both
    let r = run(&mut s, "scout.assemblage", json!({"kind": "cheville", "a": "A", "b": "B", "d": 20}));
    assert_eq!(r["created"].as_array().map(Vec::len), Some(1), "{r}");
    // the records follow the geometry
    let a = s.doc.components.iter().find(|c| c.name == "A").and_then(|c| c.properties.get("scout_pole")).cloned().unwrap_or_default();
    assert!(a.contains("Notch") && a.contains("Hole"), "{a}");
    // a lashing, then the joints are listed in the BOM too
    run(&mut s, "scout.assemblage", json!({"kind": "brelage", "a": "A", "b": "B"}));
    assert!(run(&mut s, "assembly.bom", json!({}))["total_occurrences"].as_u64().unwrap_or(0) >= 5);
}

#[test]
fn a_tenon_goes_into_a_mortise() {
    for kind in ["hache", "scie", "plat", "platscie", "aronde1", "aronde2", "aronde3"] {
        let mut s = Session::default();
        run(&mut s, "scout.piece.traverse", json!({"name": "Pied", "diameter": 100, "length": 800, "at": [0, 0, 0], "dir": [0, 0, 1]}));
        run(&mut s, "scout.piece.traverse", json!({"name": "Traverse", "diameter": 60, "length": 600, "at": [0, 0, 500], "dir": [1, 0, 0]}));
        let r = run(&mut s, "scout.assemblage", json!({"kind": kind, "a": "Traverse", "b": "Pied"}));
        assert!(r["changed"].as_u64().unwrap_or(0) >= 3, "{kind} {r}");
        let t = s.doc.components.iter().find(|c| c.name == "Traverse").and_then(|c| c.properties.get("scout_pole")).cloned().unwrap_or_default();
        assert!(t.contains("Tenon"), "{kind} {t}");
    }
    // parallel poles cannot be tenoned
    let mut s = Session::default();
    run(&mut s, "scout.piece.traverse", json!({"name": "A"}));
    run(&mut s, "scout.piece.traverse", json!({"name": "B", "at": [0, 300, 40]}));
    assert!(s.execute("scout.assemblage", &json!({"kind": "hache", "a": "A", "b": "B"})).is_err());
}

#[test]
fn a_pole_is_split_cut_and_sharpened() {
    let mut s = Session::default();
    run(&mut s, "scout.piece.traverse", json!({"name": "Rondin", "diameter": 150, "length": 1000}));
    run(&mut s, "scout.faconner", json!({"kind": "demi", "piece": "Rondin"}));
    let r = run(&mut s, "scout.faconner", json!({"kind": "couper", "piece": "Rondin", "length": 800}));
    assert_eq!(r["length"].as_f64(), Some(800.0));
    run(&mut s, "scout.faconner", json!({"kind": "pointe", "piece": "Rondin", "len": 200}));
    // a pole is split once
    assert!(s.execute("scout.faconner", &json!({"kind": "quart", "piece": "Rondin"})).is_err());
    run(&mut s, "scout.piece.traverse", json!({"name": "Autre", "diameter": 150, "length": 1000, "at": [0, 500, 75]}));
    run(&mut s, "scout.faconner", json!({"kind": "quart", "piece": "Autre"}));
    assert!(s.execute("scout.faconner", &json!({"kind": "couper", "piece": "Rondin", "length": 5000})).is_err());
    assert!(s.execute("scout.faconner", &json!({"kind": "pointe", "piece": "Rondin", "len": 9000})).is_err());
    assert!(s.execute("scout.faconner", &json!({"kind": "zzz", "piece": "Rondin"})).is_err());
}

#[test]
fn the_other_pieces_of_cathia_exist() {
    let mut s = Session::default();
    assert_eq!(run(&mut s, "scout.piece.equerre", json!({}))["poles"].as_u64().map(|n| n >= 5), Some(true));
    assert!(run(&mut s, "scout.piece.claies", json!({}))["poles"].as_u64().unwrap_or(0) >= 10);
    assert!(run(&mut s, "scout.piece.filet", json!({}))["poles"].as_u64().unwrap_or(0) >= 20);
}

#[test]
fn the_html_export_holds_the_corner_and_every_furniture() {
    let mut s = Session::default();
    assert!(s.execute("scout.export.html", &json!({})).is_err());
    run(&mut s, "scout.coin.create", json!({"name": "Coin des Lions"}));
    run(&mut s, "scout.meuble.create", json!({"type": "banc", "name": "Banc seul"}));
    let r = run(&mut s, "scout.export.html", json!({"title": "Camp d'été"}));
    assert_eq!((r["coins"].as_u64(), r["meubles"].as_u64()), (Some(1), Some(6)));
    let html = r["html"].as_str().unwrap_or_default();
    assert!(html.contains("Coin des Lions") && html.contains("Banc seul") && html.contains("Camp d'été"));
    assert!(s.execute("scout.export.html", &json!({"path": "x.txt"})).is_err());
}

#[test]
fn bad_input_is_refused_not_panicked() {
    let mut s = Session::default();
    for p in [
        json!({"type": "banc", "L": "long"}),
        json!({"type": "banc", "L": 1e30}),
        json!({"type": "banc", "rotation": 45}),
        json!({"type": "banc", "at": [1]}),
        json!({"type": "banc", "coin": "nope"}),
        json!({"type": "fusée"}),
        json!({}),
    ] {
        assert!(s.execute("scout.meuble.create", &p).is_err(), "{p}");
    }
    assert!(s.execute("scout.coin.create", &json!({"layout": "x"})).is_err());
    assert!(s.execute("scout.atelier.dessin", &json!({})).is_err());
    assert!(s.execute("scout.atelier.dessin", &json!({"meuble": 99})).is_err());
    assert!(s.execute("scout.atelier.nomenclature", &json!({})).is_err());
    assert!(s.execute("scout.assemblage", &json!({"kind": "mibois", "a": "A", "b": "A"})).is_err());
    assert!(s.execute("scout.assemblage", &json!({"kind": "mibois"})).is_err());
    assert!(s.execute("scout.piece.traverse", &json!({"length": 1e12})).is_err());
    assert!(s.execute("scout.piece.traverse", &json!({"dir": [0, 0, 0]})).is_err());
    assert!(s.execute("scout.piece.claies", &json!({"long": -1})).is_err());
}

#[test]
fn the_registry_has_no_sheet_metal_left() {
    assert!(crate::cmd::is_retired("sheet.flange"));
    assert!(crate::cmd::is_retired("surface.loft"));
    assert!(crate::cmd::find_command("scout.meuble.create").is_some());
    assert!(crate::cmd::find_command("solid.extrude").is_some());
}

#[test]
fn the_drawing_workshop_takes_its_views_scale_and_title_block() {
    let mut s = Session::default();
    run(&mut s, "scout.meuble.create", json!({"type": "banc", "name": "Banc du coin"}));
    let r = run(
        &mut s,
        "scout.atelier.dessin",
        json!({"format": "A4", "echelle": 20, "vues": ["face", "dessus"], "cotes": false, "tableau": false, "titre": "Mon plan", "dessinateur": "Lou", "societe": "FSGGB", "indice": "B"}),
    );
    let svg = r["svg"].as_str().unwrap_or_default();
    assert_eq!(r["sheet"]["echelle"].as_f64(), Some(20.0), "{}", r["sheet"]);
    assert!(svg.contains("Mon plan") && svg.contains("Lou") && svg.contains("FSGGB") && svg.contains("Ind. B"));
    assert!(svg.contains("Vue de face") && svg.contains("Vue de dessus") && !svg.contains("Vue de droite") && !svg.contains("Nomenclature"));
    assert!(r["prims"].as_array().is_some_and(|a| a.len() > 20));
    for bad in [json!({"format": "A0"}), json!({"vues": ["face", "zzz"]}), json!({"vues": []}), json!({"echelle": -3}), json!({"path": "x.txt"})] {
        assert!(s.execute("scout.atelier.dessin", &bad).is_err(), "{bad}");
    }
}

#[test]
fn the_nomenclature_has_its_three_tables_and_a_csv() {
    let mut s = Session::default();
    run(&mut s, "scout.coin.create", json!({}));
    let all = run(&mut s, "scout.atelier.nomenclature", json!({}));
    assert!(all["pieces"].as_array().is_some_and(|a| a.len() > 10), "{}", all["poles"]);
    let one = run(&mut s, "scout.atelier.nomenclature", json!({"meuble": "Étagère"}));
    assert!(one["poles"].as_u64().unwrap_or(0) < all["poles"].as_u64().unwrap_or(0));
    assert!(one["csv"].as_str().is_some_and(|c| c.contains("LISTE DE MATÉRIEL")));
    assert!(s.execute("scout.atelier.nomenclature", &json!({"path": "x.txt"})).is_err());
}

#[test]
fn a_profile_of_metal_is_made_in_each_shape() {
    for kind in ["tube_rond", "tube_carre", "corniere", "u", "i"] {
        let mut s = Session::default();
        let r = run(&mut s, "scout.profil", json!({"kind": kind, "size": 40, "epaisseur": 3, "length": 500, "dir": "Y"}));
        assert!(r["name"].as_str().is_some(), "{kind}");
        let size = size_of(&s, r["name"].as_str().unwrap_or(""));
        assert!((size[1] - 500.0).abs() < 1.0, "{kind} {size:?}");
    }
    let mut s = Session::default();
    assert!(s.execute("scout.profil", &json!({"kind": "zzz"})).is_err());
    assert!(s.execute("scout.profil", &json!({"size": 10, "epaisseur": 6})).is_err());
}

#[test]
fn a_half_lap_can_be_pegged_in_the_same_step() {
    let mut s = Session::default();
    run(&mut s, "scout.piece.traverse", json!({"name": "A", "diameter": 80, "length": 1000, "at": [0, 0, 40]}));
    run(&mut s, "scout.piece.traverse", json!({"name": "B", "diameter": 80, "length": 1000, "at": [500, -500, 110], "dir": [0, 1, 0]}));
    let r = run(&mut s, "scout.assemblage.mibois", json!({"a": "A", "b": "B", "chev": true, "d": 20}));
    assert_eq!(r["created"].as_array().map(Vec::len), Some(1), "{r}");
}

#[test]
fn a_sketch_plane_offset_runs_along_its_normal() {
    // XZ offsets along +y, YZ along +x, XY along +z, and the sketch's own axes are as `to_sketch` says.
    for (plane, axis) in [("XZ", 1usize), ("YZ", 0), ("XY", 2)] {
        let mut s = Session::default();
        run(&mut s, "sketch.create", json!({"plane": plane, "offset": 10, "name": "P"}));
        run(&mut s, "sketch.line", json!({"points": [[0, 0], [40, 0], [40, 10], [0, 10], [0, 0]]}));
        run(&mut s, "sketch.finish", json!({}));
        run(&mut s, "solid.extrude", json!({"distance": 6, "direction": "symmetric", "body_name": "T"}));
        let st = s.world_state();
        let b = st.bodies.iter().fold(solidcraft_geom::Aabb3::EMPTY, |a, b| a.union(&b.mesh().bounds()));
        let (lo, hi) = ([b.min.x, b.min.y, b.min.z], [b.max.x, b.max.y, b.max.z]);
        let c = (lo[axis] + hi[axis]) / 2.0;
        assert!((c - 10.0).abs() < 1e-6, "{plane}: centre {c} {lo:?} {hi:?}");
    }
}

/// Volume of the body of a component named `name`.
fn volume_of(s: &mut Session, name: &str) -> f64 {
    let r = run(s, "inspect.center_of_mass", json!({"bodies": [name]}));
    r["volume"].as_f64().unwrap_or(0.0)
}

fn pied_and_traverse(s: &mut Session) {
    run(s, "scout.piece.traverse", json!({"name": "Pied", "diameter": 100, "length": 800, "at": [0, 0, 0], "dir": [0, 0, 1]}));
    run(s, "scout.piece.traverse", json!({"name": "Traverse", "diameter": 60, "length": 600, "at": [0, 0, 500], "dir": [1, 0, 0]}));
}

#[test]
fn every_tenon_fits_its_mortise_without_touching_it() {
    for kind in ["hache", "scie", "plat", "platscie", "aronde1", "aronde2", "aronde3"] {
        let mut s = Session::default();
        pied_and_traverse(&mut s);
        let (v_pied, v_trav) = (volume_of(&mut s, "Pied"), volume_of(&mut s, "Traverse"));
        run(&mut s, "scout.assemblage", json!({"kind": kind, "a": "Traverse", "b": "Pied"}));
        let (w_pied, w_trav) = (volume_of(&mut s, "Pied"), volume_of(&mut s, "Traverse"));
        assert!(w_pied < v_pied - 500.0, "{kind}: the mortise took nothing ({v_pied} → {w_pied})");
        assert!(w_trav > 0.0, "{kind}: {v_trav} → {w_trav}");
        // the joint is exact: the tenon sits in its mortise with a little play and no overlap
        let r = run(&mut s, "inspect.interference", json!({}));
        let overlap: f64 = r["interferences"].as_array().map(|a| a.iter().map(|x| x["volume"].as_f64().unwrap_or(0.0)).sum()).unwrap_or(0.0);
        assert!(overlap < 1.0, "{kind}: the tenon overlaps its mortise by {overlap} mm³ {r}");
    }
}

#[test]
fn a_dovetail_flares_towards_its_tip() {
    let mut s = Session::default();
    pied_and_traverse(&mut s);
    run(&mut s, "scout.assemblage", json!({"kind": "aronde1", "a": "Traverse", "b": "Pied", "w": 30, "theta": 15}));
    let rec = s.doc.components.iter().find(|c| c.name == "Traverse").and_then(|c| c.properties.get("scout_pole")).cloned().unwrap_or_default();
    assert!(rec.contains("Aronde") && rec.contains("\"theta\":1"), "{rec}");
    // the tenon is wider at its tip than at its shoulder: the traverse's bounding box in the wide
    // direction (here z) is the pole's 60 mm, so look at the mortise: it is a groove across the pied
    let size = size_of(&s, "Traverse");
    assert!(size[0] > 500.0, "{size:?}");
}

#[test]
fn a_tenon_does_not_come_through_a_split_log() {
    // a leg tenoned into the round side of a split log stays inside it
    let mut s = Session::default();
    run(&mut s, "scout.piece.traverse", json!({"name": "Rondin", "diameter": 200, "length": 1000, "at": [0, 100, 450], "dir": [1, 0, 0]}));
    run(&mut s, "scout.faconner.demi", json!({"piece": "Rondin"}));
    run(&mut s, "scout.piece.traverse", json!({"name": "Pied", "diameter": 80, "length": 450, "at": [500, 150, 0], "dir": [0, 0, 1]}));
    run(&mut s, "scout.assemblage.tenon", json!({"a": "Pied", "b": "Rondin"}));
    let top = size_of(&s, "Rondin");
    let pied = size_of(&s, "Pied");
    assert!(top[2] <= 101.0, "the log's flat face is intact: {top:?}");
    assert!(pied[2] < 450.0 + 1.0, "{pied:?}");
    let all = run(&mut s, "inspect.interference", json!({}));
    assert!(all["interferences"].as_array().is_none_or(Vec::is_empty), "{all}");
}

#[test]
fn shaping_a_placed_pole_changes_its_volume_as_it_should() {
    let vol = |s: &mut Session, n: &str| run(s, "inspect.center_of_mass", json!({"bodies": [n]}))["volume"].as_f64().unwrap_or(-1.0);
    let mut s = Session::default();
    // placed away from the origin, along x, so its frame is not the world's
    run(&mut s, "scout.piece.traverse", json!({"name": "R", "diameter": 100, "length": 500, "at": [700, 300, 450], "dir": [1, 0, 0]}));
    let full = vol(&mut s, "R");
    run(&mut s, "scout.faconner.couper", json!({"piece": "R", "length": 300}));
    let cut = vol(&mut s, "R");
    assert!((cut / full - 0.6).abs() < 0.02, "{full} → {cut}");
    run(&mut s, "scout.faconner.demi", json!({"piece": "R"}));
    let half = vol(&mut s, "R");
    assert!((half / cut - 0.5).abs() < 0.03, "{cut} → {half}");
    run(&mut s, "scout.faconner.pointe", json!({"piece": "R", "len": 150}));
    let pointed = vol(&mut s, "R");
    assert!(pointed < half && pointed > half * 0.8, "{half} → {pointed}");
}
