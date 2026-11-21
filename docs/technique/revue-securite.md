# Revue de sécurité locale — phase 10

Revue du **13 novembre 2026**, sur les sources et dépendances verrouillées de Patrimoine **0.1.1**, pour la cible Linux x86_64 Debian 13. Elle décrit les contrôles effectués et les risques conservés. Elle ne constitue ni un audit indépendant complet, ni une certification de sécurité. Les essais utilisent exclusivement des coffres synthétiques.

## Résultat et risques conservés

La comparaison locale des dépendances JavaScript ne trouve aucune correspondance dans le snapshot d’avis publics examiné. L’audit Rust ne comporte aucune entrée dans sa liste de vulnérabilités, mais signale **deux avertissements informatifs non corrigés**, dont un défaut de sûreté mémoire dans GLib. Présenter ce résultat comme un audit sans alerte serait incorrect.

| Composant verrouillé | Avis et portée | Décision pour cette qualification |
|---|---|---|
| `proc-macro-error 1.0.4` | [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html) : bibliothèque non maintenue, apportée notamment par `glib-macros`, exécutée pendant la compilation | Avertissement conservé. Les sources de compilation restent une surface de confiance ; suivre les mises à jour de la chaîne GTK/Tauri et ses macros. |
| `glib 0.18.5` | [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html), [GHSA-wrw7-89jp-8q8g](https://github.com/advisories/GHSA-wrw7-89jp-8q8g) : comportement indéfini dans `VariantStrIter::impl_get`, appelé par les itérateurs de chaînes de `Variant` ; corrigé à partir de `0.20.0` | Version affectée conservée via GTK 3/WebKitGTK/Tauri. La montée de GLib seule ne respecte pas les contraintes de cette chaîne. Aucun fork local ni correctif de source tiers n’est appliqué. |

La recherche dans les sources installées trouve `VariantStrIter` et `array_iter_str` dans les définitions, exemples et tests de GLib, sans appel identifié dans l’application ni les dépendants Tauri/GTK examinés. Une inspection complémentaire par `nm -C target/debug/patrimoine` réussit avec **125 548 lignes de symboles** et aucun symbole correspondant à ces deux noms. Le fichier est un exécutable de développement avec informations de débogage, non dépouillé ; son SHA-256 est `d394e0c91ef3439d55a07e7b5f0579551e5e17dfd8bb033e3b19a104b3722d5e`. La [preuve du contrôle final](../historique/preuves.md#preuve-057) conserve aussi le type non dépouillé du fichier et la sortie réussie de l’outil. Ce constat porte sur cet exécutable, sans déduire une absence de symboles à partir d’un paquet de production dépouillé. Il réduit la portée d’appel observée ; il ne corrige pas la dépendance et ne démontre pas l’absence de tout chemin indirect.

Toute introduction d’un appel à `array_iter_str`, toute modification de la chaîne GTK/Tauri ou toute nouvelle construction impose de réexaminer cet avis. Le défaut reste inscrit dans les risques de cette version, même en l’absence d’appel observé.

## Dépendances : méthode et preuves

Les résultats sont datés : un avis publié après le snapshot ou absent de sa couverture peut modifier les conclusions.

| Contrôle | Base et périmètre | Résultat enregistré |
|---|---|---|
| Rust, `cargo-audit 0.23.0` | `Cargo.lock`, **432 dépendances** ; **1 490 avis** RustSec ; checkout public `af7b3ec33460b8e2de8d422e16d628518891a519`, commit du 12 novembre 2026 | `vulnerabilities.count = 0`, plus les **deux avertissements** détaillés ci-dessus. [Résultat JSON](../historique/preuves.md#preuve-070). |
| JavaScript, comparaison locale | **203 couples nom/version distincts**, dont **46 de production** ; GitHub Advisory Database, commit `6129b60c1e199460d2925a7f5e6195fe7e58813c` du **12 novembre 2026 à 21:08:39 UTC** ; avis `github-reviewed` au format OSV | **37 729 fichiers d’avis**, **10 698 déclarations npm** ; **0 correspondance**, **0 plage non résolue**. [Résultat JSON](../historique/preuves.md#preuve-068). |

La tentative d’audit npm nécessitant l’envoi d’un inventaire de dépendances a été refusée par la revue automatique d’autorisation. Elle n’a pas été relancée ni contournée. La méthode retenue télécharge la base publique indépendamment du projet, puis compare les noms et versions localement avec [audit-js-offline.mjs](../../scripts/audit-js-offline.mjs). Aucun inventaire du projet n’est envoyé par ce script.

Le comparateur utilise `semver 7.8.5`, les versions explicites et les événements OSV `SEMVER` et `ECOSYSTEM` npm. Ses **sept contrôles de bornes** couvrent introduction, correctif exclusif, dernière version affectée inclusive, préversions et intervalles disjoints ; ils sont inclus dans [check.sh](../../scripts/check.sh). Les avis retirés sont ignorés. Les avis non examinés par GitHub, listes de paquets malveillants et autres écosystèmes ne sont pas couverts par ce contrôle JavaScript.

Reproduction avec le même checkout public disponible localement :

```bash
node scripts/audit-js-offline.mjs --self-test
node scripts/audit-js-offline.mjs \
  --database /chemin/advisory-database \
  --lockfile package-lock.json \
  --database-commit-time 2026-11-12T21:08:39Z \
  --output resultat-audit-js.json
```

Le résultat contient l’empreinte du lockfile effectivement lu. Les inventaires et notices natifs sont décrits dans [licences-tierces.md](licences-tierces.md). Les bibliothèques Debian, le runtime AppImage et le noyau ne sont pas couverts par la comparaison npm ni par RustSec ; aucun scan exhaustif de leurs CVE n’est revendiqué ici. Les versions exactes permettent une revue séparée et doivent être suivies lors d’une nouvelle construction.

## Interface locale, IPC et fichiers

La [configuration Tauri](../../src-tauri/tauri.conf.json) embarque l’interface et applique une CSP de production : scripts et polices locaux, objets et frames interdits, formulaires désactivés, connexions limitées à l’IPC Tauri. Les styles en ligne restent autorisés pour l’interface et les graphiques. La politique de développement distincte autorise le WebSocket Vite local ; elle ne remplace pas la politique du paquet.

La [capacité de la fenêtre principale](../../src-tauri/capabilities/main.json) accorde les fonctions de base et les dialogues ouvrir/enregistrer. Elle n’expose pas au JavaScript d’API générique de fichiers, de shell ou de SQL. Les commandes métier Rust constituent néanmoins une surface IPC et doivent rester validées côté Rust. Pour les imports et exports, le [backend](../../src-tauri/src/main.rs) vérifie le chemin absolu et la portée accordée par le dialogue natif avant de transmettre le chemin au coffre.

Les [primitives de fichiers](../../crates/infrastructure/src/files.rs) refusent les chemins relatifs, traversées et liens symboliques, utilisent `O_NOFOLLOW`, vérifient les fichiers ordinaires et créent les nouveaux fichiers en `0600`. Le coffre est privé (`0700`) et un verrou exclusif empêche son ouverture simultanée en écriture. Les destinations d’export doivent être neuves et extérieures au coffre ; la publication par lien atomique refuse un fichier apparu entre-temps. Ces protections ne remplacent pas l’isolation du compte Linux et ne constituent pas une preuve de résistance à toutes les courses de fichiers provoquées par un autre programme du même compte.

Les données, documents et sauvegardes ne sont **pas chiffrés** par Patrimoine. Un programme exécuté avec les droits du même utilisateur peut lire ses données ; l’application ne fournit pas de séparation de droits entre membres d’une famille. Les empreintes SHA-256 détectent une altération par rapport à une valeur connue, sans authentifier l’auteur d’un paquet ou d’une archive. Le [guide Linux](../utilisateur/installation-linux.md) détaille emplacement des données, sauvegarde et récupération.

## Archives et restauration

Le lecteur ne fait pas confiance au manifeste seul. Il vérifie les noms d’entrées, les doublons, les types de fichiers, les tailles, les empreintes, le schéma SQLite attendu, les références et les invariants applicatifs avant activation. Les contrôles SQLite utilisent notamment `trusted_schema=OFF` ; les lectures de validation sont en lecture seule. Une empreinte recalculée ne rend pas acceptable un schéma supplémentaire, un déclencheur ou une version future.

Les budgets de lecture et d’écriture de cette version sont les suivants :

| Élément | Limite |
|---|---|
| Entrées ZIP, métadonnées comprises | **11 000** |
| Manifeste | **4 Mio** |
| Entrée SQLite canonique `data/patrimoine.sqlite3` uniquement | **256 Mio** |
| Chaque autre fichier, dont original et table CSV | **128 Mio** |
| Total décompressé | **512 Mio** |

Un ratio de décompression supérieur à 200 est également refusé pour une entrée de plus de 1 Mio. Les chemins absolus ou traversants, liens, fichiers spéciaux et fichiers inattendus sont refusés. Les budgets CSV supplémentaires sont deux millions de cellules par table et 1 Mio par cellule. Le dépassement provoque un refus, sans troncature silencieuse. Le schéma reste **6** ; une version antérieure du lecteur peut toutefois avoir un budget plus faible.

Les [tests adversariaux de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) couvrent archives altérées, chemins hostiles, doublons, liens, références manquantes, schéma ou déclencheur injecté, confirmation absente, jetons périmés et altération du candidat après analyse. Ils vérifient le maintien de la génération active lors du refus. Les cas d’interruption vérifient le maintien de l’ancienne génération ou une nouvelle génération complète, avec retour à l’ancienne si la candidate est corrompue ou incomplète. Les [tests de capacité](../../crates/infrastructure/src/archives_capacity_tests.rs) exercent la frontière réelle de 11 000 entrées et le budget SQLite réservé à son nom canonique. Les [tests du secours CSV](../../scripts/test-recover-csv.py) vérifient séparément le lecteur Python.

## Exécution hors réseau et limites de qualification

Un lancement réel du **prototype local 0.1.0** par extraction AppImage a réussi dans un conteneur Debian, sous utilisateur non privilégié, avec `--network none` et sans Rust, Cargo, Node, npm, GCC ni `pkg-config`. Le test atteint la fenêtre Tableau de bord et obtient réellement le statut par IPC : schéma **6**, coffre neuf à K=0, données stockées hors du paquet. Le [journal de ce contrôle](../historique/preuves.md#preuve-067) identifie explicitement la version `0.1.0` ; il ne vaut pas à lui seul validation du candidat `0.1.1`.

Le [rapport de qualification phase 10](../historique/validation-phase-10.md) précise les artefacts finaux, leurs empreintes et les essais effectivement exécutés sur `0.1.1`, y compris mise à jour, retour et récupération. Le montage FUSE et l’exécution par extraction sont deux modes distincts ; la réussite du second n’établit pas celle du premier.

La revue est une inspection des configurations, dépendances et cas reproductibles disponibles. Elle ne comporte pas de test d’intrusion indépendant, de fuzzing exhaustif, de preuve formelle ni d’analyse complète des composants natifs et de la distribution hôte. Les deux avertissements Rust, la protection du compte et des supports, la provenance du paquet et la maintenance des dépendances restent des points de suivi explicites.

## Réexamen du 20 novembre 2026 pour la version 0.1.2

Les comparaisons portent sur les verrous de la version 0.1.2. Les deux bases d’avis publiques ont été téléchargées séparément, puis comparées localement. Le contrôle JavaScript examine 38 006 avis `github-reviewed` et 203 couples nom/version, dont 46 de production. Il trouve zéro correspondance et zéro plage non résolue. Le commit de la base GitHub est `e513a44dab27602cb7373052185a2b755b025a03`.

RustSec examine 1 555 avis du commit `f291af9b1cc0d21dfcea1709d6b037963cef262b` et 432 dépendances verrouillées. Sa liste de vulnérabilités est vide et les deux avertissements de 0.1.1 restent suivis. Un troisième avertissement signale `chrono 0.4.45` comme non maintenu. Ce paquet figure dans le verrou via une dépendance optionnelle de `serde_with`. La commande `cargo tree --workspace --locked --offline --all-features --target all -i chrono` ne trouve aucune dépendance active vers lui. [Avis RustSec RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543.html)

La revue hebdomadaire télécharge les bases publiques et compare les dépendances sur place. Le prochain réexamen est fixé au 12 décembre 2026. Les versions de WebKitGTK et des bibliothèques natives relèvent de la qualification du paquet Linux. [Revue automatisée](../../.github/workflows/advisories.yml) · [Politique de suivi](../../.github/advisory-review.json) · [Validation 0.1.2](../projet/validation-corrections-0.1.2.md)
