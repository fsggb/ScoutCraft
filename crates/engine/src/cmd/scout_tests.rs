use serde_json::{Value, json};

use crate::Session;

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id} {p}: {e}"),
    }
}

#[test]
fn a_bench_is_a_component_of_counted_poles() {
    let mut s = Session::default();
    let r = run(&mut s, "scout.meuble.banc", json!({}));
    assert_eq!(r["name"], "Banc");
    let poles = r["poles"].as_u64().unwrap_or(0);
    assert!(poles >= 13, "{r}");
    let bom = run(&mut s, "assembly.bom", json!({}));
    assert_eq!(bom["total_occurrences"].as_u64().unwrap_or(0), poles, "{bom}");
    let legs = bom["rows"].as_array().map(|rows| rows.iter().filter(|x| x["part_number"].as_str().is_some_and(|n| n.contains("PIED"))).map(|x| x["count"].as_u64().unwrap_or(0)).sum::<u64>());
    assert_eq!(legs, Some(4), "{bom}");
    // one undo step takes the whole furniture back
    run(&mut s, "edit.undo", json!({}));
    assert!(s.doc.components.is_empty() || s.doc.components.iter().all(|c| c.id == 0), "{:?}", s.doc.components.len());
}

#[test]
fn its_plan_lists_what_to_cut_and_how_to_build() {
    let mut s = Session::default();
    run(&mut s, "scout.meuble.table", json!({"length": 1800, "name": "Table du coin"}));
    let plan = run(&mut s, "scout.plan.meuble", json!({"author": "Lou"}));
    assert_eq!(plan["name"], "Table du coin");
    assert_eq!(plan["dimensions"]["length"].as_f64().map(f64::round), Some(1800.0), "{plan}");
    assert!(plan["svg"].as_str().is_some_and(|x| x.contains("Table du coin") && x.contains("Lou")));
    assert!(plan["steps"].as_array().is_some_and(|x| x.len() >= 5));
    assert!(plan["cut_list"].as_array().is_some_and(|x| x.iter().any(|r| r["role"] == "claie")));
}

#[test]
fn a_patrol_corner_holds_several_furniture_each_with_its_own_plan() {
    let mut s = Session::default();
    let r = run(&mut s, "scout.coin.create", json!({"name": "Coin des Lions"}));
    assert_eq!(r["meubles"].as_array().map(Vec::len), Some(5), "{r}");
    let plan = run(&mut s, "scout.plan.coin", json!({}));
    assert_eq!(plan["meubles"].as_array().map(Vec::len), Some(5));
    assert!(plan["svg"].as_str().is_some_and(|x| x.contains("Coin des Lions")));
    // each furniture still plans on its own
    let one = run(&mut s, "scout.plan.meuble", json!({"meuble": "Étagère"}));
    assert_eq!(one["type"], "etagere");
    // two furniture: naming one is needed
    let e = s.execute("scout.plan.meuble", &json!({}));
    assert!(e.is_err());
}

#[test]
fn bad_input_is_refused_not_panicked() {
    let mut s = Session::default();
    for p in [
        json!({"length": "long"}),
        json!({"length": 1e30}),
        json!({"rotation": 45}),
        json!({"at": [1]}),
        json!({"coin": "nope"}),
        json!({"leg": 10}),
    ] {
        assert!(s.execute("scout.meuble.banc", &p).is_err(), "{p}");
    }
    assert!(s.execute("scout.meuble.create", &json!({"type": "fusée"})).is_err());
    assert!(s.execute("scout.meuble.create", &json!({})).is_err());
    assert!(s.execute("scout.coin.create", &json!({"layout": "x"})).is_err());
    assert!(s.execute("scout.plan.coin", &json!({})).is_err());
    assert!(s.execute("scout.plan.meuble", &json!({"meuble": 99})).is_err());
}

#[test]
fn the_html_export_holds_the_corner_and_every_furniture() {
    let mut s = Session::default();
    assert!(s.execute("scout.export.html", &json!({})).is_err());
    run(&mut s, "scout.coin.create", json!({"name": "Coin des Lions"}));
    run(&mut s, "scout.meuble.banc", json!({"name": "Banc seul"}));
    let r = run(&mut s, "scout.export.html", json!({"title": "Camp d'été"}));
    assert_eq!((r["coins"].as_u64(), r["meubles"].as_u64()), (Some(1), Some(6)), "{}", r["coins"]);
    let html = r["html"].as_str().unwrap_or_default();
    assert!(html.contains("Coin des Lions") && html.contains("Banc seul") && html.contains("Camp d&#39;été") || html.contains("Camp d'été"));
    assert!(s.execute("scout.export.html", &json!({"path": "x.txt"})).is_err());
}

#[test]
fn the_registry_has_no_sheet_metal_left() {
    assert!(crate::cmd::find_command("sheet.flange").is_none());
    assert!(crate::cmd::find_command("surface.loft").is_some());
    assert!(crate::cmd::find_command("scout.meuble.banc").is_some());
    assert!(crate::cmd::find_command("solid.extrude").is_some());
}
