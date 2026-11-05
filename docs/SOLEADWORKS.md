# SoLeadWorks : base, simplification, feuille de route

## Principe

SoLeadWorks = SolidCraft (clean-room, Rust) + une couche « Scout » + un registre de commandes réduit.
Tout est une commande ; l'interface est mince. On ne retire donc pas du code au hasard : on retire
des **commandes**, et ce qui n'est plus appelé se supprime ensuite module par module.

## Retiré (depuis le registre et le ruban)

Le commutateur unique est `crates/engine/src/cmd/retired.rs`. Une commande qui y figure n'est ni
listée, ni trouvée, ni exécutable, ni placée sur le ruban.

- familles entières : `render.*`, `surface.*`, `sheet.*` (tôlerie), `plastic.*`, `motion.*`,
  `pointcloud.*`, `mesh.*`, `canvas.*`, `config.*` (configurations), `contact.*`, `library.*`
- solides : balayage, lissage, tore, tuyau, ressort, filetage, nervure, voile, estampage, remplissage
  de frontière, dépouille
- esquisse : 3D, projections sur surface, courbes d'équation, coniques, ajustement de splines
- inspection : zébrures, courbures, continuité, accessibilité, dépouille
- fichiers : maillages, trancheurs (impression 3D)
- liaisons mécaniques : engrenages, crémaillère, vis, came, mouvement lié, pilotage
- ruban : onglets SURFACE, TÔLERIE, SOUDURES, rendu ; panneaux Règles, Configurations, Contact, Mouvement
- dépôt : packaging, outils d'oracle, scripts de revue de l'amont, workflows de publication,
  captures d'écran de SolidCraft ; la synchronisation hebdomadaire avec l'amont (elle défairait
  l'élagage).

## Ajouté

`crates/engine/src/scout/` (modèles de meubles, plans SVG) et `crates/engine/src/cmd/scout.rs`
(commandes `scout.*`). Le ruban s'ouvre sur l'onglet **Scout** ; l'interface est en français par défaut.

### Meuble ≠ coin de patrouille

- un **meuble** est un composant ; chaque perche est un sous-composant avec son numéro de pièce
  (`TABLE-PIED-80x720`) : la nomenclature `assembly.bom` les regroupe et les compte ;
- un **coin de patrouille** est un composant conteneur (`scout_coin`) qui range des meubles ; chacun
  garde son plan (`scout.plan.meuble`), et le coin a sa vue d'ensemble (`scout.plan.coin`).
- tout est relu depuis les propriétés enregistrées dans le design : un fichier rouvert plus tard se
  planifie de la même façon.

## Refonte sur le modèle de CathIA

- 26 icônes `scout_*` (assets/icons/ui/design/svg/model) : un pictogramme par fonction.
- Onglets et panneaux de CathIA ; la boîte Assembler liste les types par groupe comme CathIA (tenons et
  mortaises, mi-bois, queues d'aronde, chevilles et brêlage), explique ce que chaque assemblage bloque,
  propose d'inverser les pièces et d'ajouter une cheville au mi-bois.
- Ateliers Dessin technique et Nomenclature : fenêtres alimentées par `scout.atelier.*`.

## Reste à faire (par ordre d'utilité)

1. **Boîte de dialogue « Nouveau meuble »** (type, dimensions, diamètres, position) : aujourd'hui le
   ruban crée le meuble avec ses dimensions par défaut ; les dimensions se règlent par
   `scout.meuble.create`.
2. **Tenons, mortaises, mi-bois, chevilles en géométrie** : aujourd'hui les perches se touchent et
   les assemblages sont décrits dans le plan et la liste de débit (tenon = un tiers du diamètre).
3. **Redimensionner un meuble existant** (reconstruction en place) et le lier à des paramètres nommés.
4. Plan de meuble en **PDF**, et plan de coin avec vues de chaque meuble sur la même feuille.
5. Plus de modèles (vaisselier, table à feu, établi, fauteuil) repris de CathIA.
6. Vérifications de bonnes pratiques (règle du tiers, tenon trop court, perches qui se croisent).
7. **Suppression physique** des modules retirés (`sheet`, `plastic`, `surface*`, `motion*`, `render*`,
   tests et crates associés) pour alléger la compilation, après l'adaptation de leurs tests.
8. Fusion de l'historique avec la visionneuse CathIA (`.cathia.json` → projet SoLeadWorks).

## Identité

Logo : celui de CathIA (tuile ScoutHub identique). Mot-symbole : **SO**·**LEAD**·WORKS, graisse forte
puis fine comme SOLIDWORKS, « LEAD » en rouge (le chef, le guide). Le logo CathIA contient un
pictogramme proche de celui de Dassault Systèmes : à remplacer avant toute diffusion hors du groupe.

## Tuile ScoutHub

À ajouter dans `ScoutHub/config.js` (même icône que CathIA, tuile mise en avant) une fois le dépôt
publié sur GitHub Pages :

```js
{ group: "Conception et CAO", featured: true, crop: true, name: "SoLeadWorks", desc: "Meubles et coins de patrouille en 3D, avec plans et historique", url: O + "SoLeadWorks/?lang=fr", icon: "assets/icons/CathIA.jpg" },
```

Le client ScoutHub est déjà chargé par `apps/solidcraft-web/index.html`
(`data-app="SoLeadWorks"`), ce qui donne l'enregistrement dans ScoutCloud.
