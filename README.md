# ScoutCraft

Suite complète d'applications créatives FSGGB (anciennement ArtCraft), servie par GitHub Pages : https://fsggb.github.io/ScoutCraft/

Ce dépôt n'a plus de portail propre : `index.html` renvoie vers ScoutHub, qui liste toutes les applications. Il assemble les versions web des applications amont (`storytold/*`, listées dans `apps.yml`) ; ScoutDraft et ScoutCAD ont chacun leur propre dépôt et leur propre déploiement.

## Mises à jour amont
Le workflow `.github/workflows/pages.yml` s'exécute **chaque lundi** : il relève la dernière version de chaque application amont, la compare à `upstream-versions.txt` et ne redéploie le site que si l'une d'elles a changé. Il peut aussi être lancé à la main (Actions > Deploy ScoutCraft > Run workflow).

## Indépendance
Ce dépôt est indépendant de ScoutOffice, ScoutHub, ScoutMap et CathIA : une maintenance ici n'affecte pas les autres sites.
