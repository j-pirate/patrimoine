# Validation de Patrimoine 0.1.2

La version 0.1.2 intègre les changements du rapport de revue. Le suivi associe chacune des 60 références à sa mise en œuvre. [Suivi des corrections](suivi-corrections.md)

Les essais utilisent des coffres synthétiques. Le schéma SQLite 6 et les empreintes des six migrations sont conservés. Le bilan explique les liens de créance anciens incohérents et permet leur régularisation. Les sauvegardes conservent les générations précédentes.

## Comparaison des dépendances du 20 novembre 2026

La comparaison JavaScript des bases publiques GitHub du 20 novembre couvre 38 006 avis examinés et 203 couples nom/version, dont 46 de production. Elle trouve zéro correspondance et zéro plage non résolue. Le commit de la base est `e513a44dab27602cb7373052185a2b755b025a03`.

RustSec examine 432 dépendances et 1 555 avis du commit `f291af9b1cc0d21dfcea1709d6b037963cef262b`. Il ne signale aucune vulnérabilité dans sa liste et conserve trois avertissements. `proc-macro-error` et GLib étaient déjà suivis. `chrono` est maintenant signalé comme non maintenu, figure dans le verrou via une dépendance optionnelle de `serde_with` et est absent du graphe résolu pour les cibles et fonctionnalités examinées. La revue hebdomadaire suit ces trois avis. [Politique des avis](../../.github/advisory-review.json)

## État de livraison

La construction et les essais natifs du candidat Pop!_OS 22.04 sont en cours. Le paquet à télécharger et ses empreintes seront publiés après validation.
