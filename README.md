<p align="center">
  <img src="assets/branding/soleadworks/logo-cathia.jpg" width="220" alt="SoLeadWorks" />
</p>

<h1 align="center"><b style="color:#000761">SO</b><b style="color:#d9232d">LEAD</b><span style="color:#000761">WORKS</span></h1>

<p align="center">
  <strong>Conception 3D de meubles et de coins de patrouille pour le camp scout.</strong><br />
  Un meuble = un plan (vues cotées, liste de débit, assemblages, ordre de montage).<br />
  Un coin de patrouille = une vue d'ensemble, et un plan pour chaque meuble.
</p>

SoLeadWorks est l'alternative « de fond » à **CathIA** : même objectif (le froissartage : bois rond,
tenons, mi-bois, claies, brêlages), mais construite sur le noyau complet de
[SolidCraft](https://github.com/tangency-labs/solidcraft) (Rust, esquisses contraintes, noyau B-rep,
historique paramétrique, assemblages, nomenclature) **allégé à l'extrême** : tout ce qui sert à la
tôlerie, au plastique, aux surfaces, aux maillages, à l'animation mécanique, au rendu ou à l'usinage
a été retiré.

## Ce qu'on garde de SolidCraft

- **Historique et arborescence** : frise chronologique des opérations (marqueur de retour en arrière,
  suppression, réordonnancement), points de contrôle (`edit.checkpoint` / `edit.restore_checkpoint`),
  annuler / rétablir.
- Esquisses contraintes et cotées, extrusion, révolution, trous, congés, chanfreins, symétrie, répétitions.
- **Assemblages** : composants, liaisons, vue éclatée (le montage), contrôle d'interférences.
- **Nomenclature** (`assembly.bom`), propriétés des pièces, paramètres nommés, unités.
- Application web (WebGPU/WebGL2) enregistrant dans ScoutCloud via ScoutHub.

## Ce qu'on ajoute : l'onglet « Scout »

| Commande | Rôle |
|---|---|
| `scout.meuble.banc`, `.banc_dossier`, `.table`, `.table_prepa`, `.etagere` | un meuble paramétré en bois rond : un composant par perche (pied, traverse, claie…), numéro de pièce et section pour la nomenclature |
| `scout.meuble.create` | idem, avec `type`, dimensions (`length`, `width`, `height`), diamètres (`leg`, `rail`, `claie`), position `at` et `rotation` |
| `scout.coin.create` | un **coin de patrouille** : un conteneur qui range plusieurs meubles au sol (disposition « standard » : table, deux bancs, étagère, table de prépa) |
| `scout.plan.meuble` | le **plan d'un meuble** (SVG A3 : face, côté, dessus, cotes d'ensemble, liste de débit, assemblages à la règle du tiers, montage) |
| `scout.plan.coin` | la **vue d'ensemble du coin** (dessus, emprise de chaque meuble, total de bois) ; avec `meuble_paths`, écrit aussi le plan de chaque meuble |
| `scout.meuble.types` | les modèles disponibles et leurs dimensions par défaut |

Toutes passent par `Session::execute` comme le reste de SolidCraft : ruban, palette de commandes,
scripts, CLI, canal de contrôle et MCP.

## Construire

```
cargo test -p solidcraft-engine --lib scout      # moteur + plans
cd apps/solidcraft-web && trunk build --release  # application web (dist/web)
```

Voir `docs/SOLEADWORKS.md` pour la feuille de route, ce qui a été retiré et ce qui reste à retirer.

## Licence

MIT OU Apache-2.0 (voir `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`). Les dépendances sont listées dans
`THIRD-PARTY-LICENSES.txt`.
