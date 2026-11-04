# SoLeadWorks : instructions pour les agents

SoLeadWorks est SolidCraft allégé et orienté meubles de camp scout (voir `README.md` et
`docs/SOLEADWORKS.md`). Il garde les règles de fond de SolidCraft :

- **Clean-room** : jamais de code, d'icônes, de textes ou de modèles copiés d'un produit commercial ;
  aucun code GPL/LGPL/AGPL (OpenCASCADE, FreeCAD, SolveSpace, CadQuery…), pas même en lecture.
- **Jamais de plantage** : pas de `unwrap`/`expect`/`panic`/`unsafe` hors tests ; les erreurs sont des
  `Result` ; toute entrée est hostile (tailles bornées, `NaN` refusés).
- **Tout est une commande** (`crates/engine/src/cmd/*`, `Session::execute`) ; un appel programmatique
  n'ouvre jamais de boîte de dialogue. L'interface est mince.
- **Couches** : geom, sketch/kernel/render, doc, io, engine, ui-egui, apps ; seul `crates/kernel`
  utilise truck. Rust uniquement, et il doit compiler pour wasm
  (`cargo check --target wasm32-unknown-unknown -p solidcraft-web`).
- Chaque ressource (image, icône, police) a une ligne dans `ATTRIBUTION.md`.
- Un test de régression va dans le fichier de test existant de la zone.

## Spécifique SoLeadWorks

- Une commande retirée se déclare dans `crates/engine/src/cmd/retired.rs` ; n'en ajoutez pas au ruban.
- La couche scout : `crates/engine/src/scout/` (données et plans, pures) et `cmd/scout.rs`.
  Un meuble est un composant dont chaque perche est un sous-composant ; tout se relit des propriétés
  `scout_*`.
- Un **plan par meuble**, une **vue d'ensemble par coin** : ne pas fondre les deux.
- L'interface est en français par défaut ; libellés français directement dans le registre.
- Avant un commit : `cargo test -p solidcraft-engine --lib scout` et le contrôle wasm.
