# Validation de Patrimoine 0.1.2

La version 0.1.2 intègre les changements du rapport de revue. Le suivi associe chacune des 60 références à sa mise en œuvre. [Suivi des corrections](suivi-corrections.md)

Les essais utilisent des coffres synthétiques. Le schéma SQLite 6 et les empreintes des six migrations sont conservés. Le bilan explique les liens de créance anciens incohérents et permet leur régularisation. Les sauvegardes conservent les générations précédentes.

## Contrôles du code source

La commande `bash scripts/check.sh` a abouti avec les dépendances disponibles localement et Cargo en mode hors ligne. Le rapport de suite compte 465 tests Rust, 251 tests d’interface, 62 tests Python et sept contrôles des bornes OSV. Il inclut le typage TypeScript, le build Vite, le formatage et Clippy Rust, la compilation desktop et la vérification des 94 commandes IPC, 143 DTO, 1 158 champs et 25 codes d’erreur. Les tests Rust de financement et de récupération ont aussi été rejoués sur la correction du message de refus.

Les tests de restauration couvrent des erreurs ciblées de renommage, de synchronisation et de nettoyage du journal. Le scénario de compatibilité FIN-01 simule deux liens que 0.1.1 pouvait enregistrer. Il vérifie la réouverture, la restauration ZIP et CSV, les écritures indépendantes, la correction progressive et la lecture à un ancien K. Les essais d’E/S sont réalisés avec des erreurs injectées en logiciel.

## Comparaison des dépendances du 20 novembre 2026

La comparaison JavaScript des bases publiques GitHub du 20 novembre couvre 38 006 avis examinés et 203 couples nom/version, dont 46 de production. Elle trouve zéro correspondance et zéro plage non résolue. Le commit de la base est `e513a44dab27602cb7373052185a2b755b025a03`.

RustSec examine 432 dépendances et 1 555 avis du commit `f291af9b1cc0d21dfcea1709d6b037963cef262b`. Il ne signale aucune vulnérabilité dans sa liste et conserve trois avertissements. `proc-macro-error` et GLib étaient déjà suivis. `chrono` est maintenant signalé comme non maintenu, figure dans le verrou via une dépendance optionnelle de `serde_with` et est absent du graphe résolu pour les cibles et fonctionnalités examinées. La revue hebdomadaire suit ces trois avis. [Politique des avis](../../.github/advisory-review.json)

## Mesures des calculs et de l’historique

Les exemples reproductibles chronomètrent des jeux synthétiques en profil de développement. Les durées dépendent de la machine et du jeu de données.

| Scénario | Résultat observé |
|---|---|
| 4 000 lignes de trésorerie et 8 000 rapprochements | 143 ms. Un dépassement de 0,01 est refusé. |
| Graphe de 384 sociétés et 384 actifs | 178 ms avec vérification du total exact. |
| 4 800 estimations sur 400 actifs | 224 ms à la date finale, 150 ms à une date ancienne. |
| Instantané ancien de 400 estimations | 37 ms avec vérification du total exact. |
| Réouverture après 3 000 changements mixtes | 303 ms, validation de l’historique comprise. |
| Sauvegarde et analyse de cet historique | 1,287 s pour la création, 963 ms pour l’analyse et la vérification de l’empreinte. |

Reproduire avec `cargo run -p patrimoine-infrastructure --example measure-cash-capacity`, `cargo run -p patrimoine-infrastructure --example measure-registry` et `cargo run -p patrimoine-infrastructure --example measure-history -- 3000`. Les mesures vérifient les résultats sur les entrées décrites. Elles donnent des repères pour ces profils.

## État de livraison

L’AppImage complète de 85 055 992 octets a été construite hors réseau sur Ubuntu 22.04.5 à partir des sources du commit `e9d3d14dbc3b867c44b25ce59cf265f5f9858b3d`. Son SHA-256 est `5e14fd85d7fb5129a5e19da7bbbef495db16fd2fe97f69980971b3caa56a3d47`. [Téléchargements et installation](../../distribution/0.1.2-popos22.04/README.md)

- **50 étapes graphiques réussies** sur ce fichier en mode extraction, avec dialogues GTK, sauvegarde, réimport et restauration. Le pilote WebKit est compilé sur Ubuntu 22.04. Le script centre les cibles au bord de la fenêtre avant le clic réel. [Parcours natif](../../distribution/0.1.2-popos22.04/preuves/native-appimage.json)
- **175 ELF vérifiés**, glibc maximale 2.35, GLIBCXX 3.4.30 et CXXABI 1.3.9. Aucune exigence forte ou optionnelle manquante. L’extraction vérifie 734 fichiers de notices et licences, couvrant 468 composants dont 113 paquets Ubuntu. [Contrôles ABI et extraction](../../distribution/0.1.2-popos22.04/validation-popos22.04.json)
- **Mise à jour 0.1.1 vers 0.1.2 et retour arrière réussis** dans le conteneur Ubuntu 22.04 avec Xvfb et sans réseau. Les 62 tables, 758 lignes et deux originaux du coffre source sont vérifiés, ainsi que l’écriture des paramètres dans la nouvelle version et le redémarrage à K=121. Le rapport à K=120 conserve ses champs et ses montants, avec 14 références de provenance ajoutées et vérifiées au même K. [Parcours de mise à jour](../../distribution/0.1.2-popos22.04/preuves/upgrade-appimage.json)

La qualification graphique utilise la session Xvfb d’Ubuntu. Elle couvre le lancement par extraction sur cette plateforme. Le test de retour à 0.1.1 utilise une copie synthétique du coffre et ne teste pas chaque correction de données métier possible après une mise à jour.
