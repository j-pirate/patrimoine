# Validation indépendante — phase 4

Cette campagne vérifie la persistance du change manuel, les deux axes temporels `D,K`, les profils civils datés et la récupération au schéma 3. Les fixtures de `crates/infrastructure/tests/fx_history.rs` sont synthétiques, créées dans des répertoires temporaires, sans service réseau. Les calculs purs du domaine, l’interface et le parcours desktop sont des campagnes séparées.

## Critères et oracles

| Domaine | Oracle indépendant attendu |
|---|---|
| Cours datés | Au 30 juin, `1 000 TND × 0,30 = 300 EUR` ; le cours de juillet ne s’applique pas. Une date antérieure au premier cours reste non calculable. |
| Sens et précision | Direct `100 CHF × 1,05 = 105 EUR` ; inverse exact et unité mineure TND à trois décimales. Aucun calcul en flottants. |
| Croisements | `TND→CHF = 0,30 / 1,04 = 15/52` ; deux révisions/dates conservées. Un pivot USD peut relier AED à EUR, sans parité AED/USD supposée. |
| Sources | Paire directe prioritaire ; origine, taux, sens et date consultables ; sources contradictoires signalées. |
| Valeur d’origine | Montant et devise d’origine persistent après consolidation ; un stock utilise le change à la date du rapport, pas à la date de son ancienne estimation. |
| Connaissance K | Une correction rétroactive de cours ou de valeur change la lecture actuelle, mais reproduit l’ancien résultat à son ancienne séquence. Une borne future est refusée. |
| Profils civils | Une période référence une révision historique précise ; noms et pays suivent `[début, fin)`. Une révision saisie après K reste absente des données retournées à K. |
| Intégrité | Cours invalide, doublon concurrent, conflit de révision, chevauchement ou référence de profil incorrecte : rejet sans modification du registre, de l’audit ni de la séquence. |
| Récupération | Sauvegarde/restauration : mêmes UUID, révisions, décimaux, séquences, historique, originaux et résultats FX/consolidés. |
| Pont P4.3 | Début, fin et composantes exactes réconciliés suivant l’ordre conventionnel ; base antérieure manquante explicitement incomplète, sans rendement inventé. |

La migration des vrais coffres/archives du schéma 2 est couverte par la campagne `registry_migrations`, séparément de cette suite. Les modules de trésorerie, de dettes saisissables et de flux ne sont pas déduits de ces tests de stock ; le sous-total patrimonial reste soumis à leur limite de couverture.

## Résultats exécutés

Exécution locale du 5 novembre 2026, dans l’environnement Linux configuré par `scripts/cloud-env.sh`, sans accès réseau. Commandes réellement exécutées, avec `CARGO_BUILD_JOBS=3` :

| Commande | Résultat observé |
|---|---|
| `cargo test -p patrimoine-infrastructure -p patrimoine-application --offline` | **76 tests réussis** : 18 FX/histoire, 29 récupération adverse, 17 registre, 8 migrations, 4 coffre. Aucun test ignoré. Les tests unitaires et doctests de ces deux crates ne contiennent pas de cas supplémentaires. |
| `cargo test -p patrimoine-infrastructure --offline --test fx_history` | **18/18**, relancés après ajout des bornes de taux et des annulations de profils. |
| `rustfmt --edition 2021 --check crates/infrastructure/src/*.rs crates/infrastructure/tests/*.rs crates/application/src/*.rs` | Réussi sur le périmètre infrastructure/application. |
| `cargo clippy -p patrimoine-infrastructure -p patrimoine-application --all-targets --offline -- -D warnings` | Réussi, dépendances du domaine incluses. |

Les 18 scénarios FX/histoire vérifient les calculs directs, inverses et croisés ; la priorité du direct et les alertes contradictoires ; le refus d’un cours futur ; la précision d’origine ; les corrections de cours et de valeurs à K ; l’annulation d’un cours ; le refus atomique de sources invalides, doublons et révisions périmées ; les profils de personnes/sociétés et d’actifs sur intervalles semi-ouverts ; les références d’une autre identité, d’un autre type ou d’une annulation ; les chevauchements ; le refus d’un K futur dans les quatre API de lecture ; le pont prix/change et sa base manquante ; enfin une sauvegarde/restauration avec originaux documentaires.

Les bornes incluent un taux entier de 36 chiffres, un taux de 36 chiffres significatifs dont 18 décimales, et une fraction à 18 décimales vérifiée sans arrondi. La saisie de taux `1,0500` est persistée sous la forme canonique `1.05`. Le montant de conversion `1,2500` devient `1.25` à la frontière du service ; les espaces de groupement dans `1 000` restent refusés. Les taux nuls, négatifs, non finis, exponentiels, à 37 chiffres significatifs ou à 19 décimales sont refusés sans insertion d’objet, révision, événement d’audit ni changement de séquence. Un profil annulé sans dépendance peut disparaître de la vue courante tout en conservant son histoire, y compris après réouverture.

La campagne de migration construit les schémas historiques avec les scripts approuvés `0001` et `0002`, sans déclasser une base actuelle. Elle couvre l’ouverture et la restauration de ces deux versions, les documents et UUID, les révisions civiles dont les horodatages sont égaux, la lecture selon leur séquence, le WAL validé et la reprise d’activation interrompue. Les fichiers de génération et archives sources restent identiques ; la génération migrée a le schéma 3 ; aucun cours ni période civile n’est inventé. Un candidat altéré ou un schéma inconnu reste refusé, même après recalcul des empreintes.

Deux attentes de fixtures ont été précisées : sans aucun poste calculable, la consolidation renvoie `non_calculable` ; la séquence d’une inspection de restauration lie le jeton au coffre actif, tandis que la séquence importée se vérifie après activation. La régression qui empêchait l’annulation d’un profil sans dépendance a été corrigée dans le domaine et est reproduite par le test de persistance.

## Vérification globale du workspace et de l’interface

`source scripts/cloud-env.sh`, puis `CARGO_BUILD_JOBS=3 bash scripts/check.sh` : **réussi** le 5 novembre 2026, après les corrections finales. La commande exécute TypeScript, Vitest, Vite, rustfmt, les tests du workspace, Clippy strict sur toutes les cibles/fonctionnalités et la compilation du binaire desktop avec `custom-protocol`.

- **122 tests Rust réussis**, aucun échec ni test ignoré : 46 domaine et 76 infrastructure. Les crates application/desktop et les doctests n’ajoutent aucun cas de test.
- Domaine : 5 précision source/arrondi, 19 attribution, 11 FX, 4 profils et 7 variation. Les oracles vérifient notamment tiers inversés exacts, `15/52` en conversion croisée, seuil de 30/31 jours incluant le 29 février, priorités indépendantes de l’ordre de stockage, refus de chemins à trois branches, demi-pair TND, références exactes des profils, annulations et identités hostiles. Les variations isolées prix/change, périmètre, droits, bases absentes et interactions sont réconciliées avant l’arrondi d’affichage.
- **40 tests UI réussis dans 5 fichiers**, aucun test ignoré : les 22 cas de phase 3 sont conservés. Les 18 nouveaux cas couvrent FX et alertes, historique/pont partiel, changement de K dont K=0, écritures désactivées, profils de plusieurs révisions partageant un même ID, dates de fin exclusives, édition du profil de base, périodes d’entité/actif et estimations TND transmises en texte. Ces tests utilisent des réponses IPC contrôlées ; ils ne constituent pas une exécution de WebKitGTK.
- TypeScript, build Vite, formatage Rust et Clippy avec `-D warnings` : réussis. Le binaire Linux embarquant l’interface est construit ; aucune AppImage n’est qualifiée par cette commande.

Une première vérification globale a relevé un style d’initialisation de fixture incompatible avec Clippy strict. La fixture a été corrigée, puis la commande complète relancée avec succès ; aucun cas n’a été désactivé. La précision de destination inconnue, auparavant arrondie implicitement à deux décimales, est désormais explicitement refusée et testée. Le dictionnaire des ZIP décrit les taux manuels du schéma 3 et leurs conventions.

## Parcours graphique Linux réel et inspection indépendante

`bash scripts/test-desktop-cloud.sh` : **réussi**, sortie 0, sur le binaire final le 5 novembre 2026. Le rapport [desktop-result.json](preuves.md#preuve-014) contient **24 étapes réussies**, aucune échouée, ignorée ou non exécutée, avec `native_dialogs_requested: true`. L’application tourne sous WebKitGTK/Xvfb dans Debian 13, utilisateur non privilégié et réseau désactivé (`--network none`), sur un coffre synthétique neuf.

Le parcours conserve les contrôles de copropriété et détention indirecte, puis vérifie réellement les nouveaux écrans/commandes :

- Saisie UI d’un cours `0,3`, conservé exactement en `0.3` ; prévisualisation Rust `1 000 TND → 300,00 EUR`, inverse `300 EUR → 1 000,000 TND`, et date sans cours non calculable.
- Correction du taux en `0.31` : sous-total courant `350 310 EUR`, résultat à K=27 `350 300 EUR`. La sélection historique désactive les écritures et reproduit l’ancien montant dans le contrôle UI.
- Croisement `1 000 TND → 250 CHF` par EUR avec les deux révisions et le cours le plus ancien au 1er janvier 2020, signalé ancien.
- Profils corrigés de personne et d’actif : deux révisions de chaque identité restent retournées, avec quatre périodes référant leurs versions exactes et le journal visible.
- Pont réel depuis le 1er janvier 2026 : sous-total initial connu `30 000 EUR`, final `350 310 EUR`, différence `320 310 EUR`. Les références initiales absentes produisent des composantes **Non attribuable** et un résidu explicite, sans rendement inventé. Le bouton UI utilise le même résultat Rust.
- Import/export d’un original par dialogues GTK, sauvegarde complète, analyse et annulation sans mutation, confirmation avec sauvegarde préalable, nouvelle génération activée, puis égalité de toutes les collections, révisions, séquences et résultats actuels/historiques après restauration. La comparaison conserve plusieurs versions d’un profil portant le même ID.

Captures contrôlées visuellement : [conversion](preuves.md#preuve-015), [ancienne connaissance](preuves.md#preuve-016) et [variation partielle](preuves.md#preuve-018). Le premier essai du pilote attendait un attribut `value` absent des options de devise ; les valeurs explicites ont été ajoutées, puis le parcours complet relancé avec succès. Les libellés/montants du pont ont été espacés après inspection de la capture ; la dernière exécution couvre cet affichage final.

Une inspection supplémentaire, avec `zipfile`, `hashlib` et `sqlite3` de Python sans utiliser l’API applicative, a contrôlé les deux ZIP du parcours final. [Résultat indépendant](preuves.md#preuve-017) : quatre fichiers manifestés par archive, taille et SHA-256 exacts, comptages de toutes les tables conformes au manifeste, `integrity_check = ok`, aucune erreur FK, `user_version = 3`. La sauvegarde principale est à la séquence 36, la sauvegarde préalable à 37. Toutes deux contiennent trois révisions de taux (`0.3`, `0.31`, `1.24`), quatre périodes, six profils d’entités, quatre profils d’actifs et un document original. Taux et montants SQLite sont de type `text`. Le dictionnaire autonome mentionne correctement le change manuel et sa convention.

## Répétabilité de l’environnement cloud

`bash scripts/setup-cloud.sh` a été réexécuté avec succès : Rust 1.99.1/rustfmt/Clippy, GTK 3.24.49, WebKitGTK 2.54.0, `npm ci` et `cargo fetch --locked`. Node 24.22.1 et npm 11.9.0 ont été vérifiés. Les étapes de démarrage sauvegardées — activation de `scripts/cloud-env.sh`, TypeScript et `cargo check --workspace --locked` — ont également réussi. Les index, signatures et TLS restent vérifiés.

Le brouillon de configuration cloud a confirmé `status: saved` pour **install_script** et **start_skill**. Les instructions conservent le checkout isolé existant, activent les outils locaux, précisent l’absence de service distant et le parcours Docker graphique sans réseau. Aucun secret ni domaine supplémentaire n’a été demandé. Ce résultat sauvegarde les instructions ; il ne publie pas l’environnement et ne valide pas sa restauration dans une nouvelle tâche. La revue/enregistrement puis la publication se font dans les paramètres de l’environnement.

## Portée et limites

Ces tests utilisent des données synthétiques et de vraies bases SQLite temporaires. La récupération interrompue est simulée par des états de fichiers et de journal ; aucune coupure physique, panne de disque, course entre processus externes ou exécution Windows/macOS n’a été simulée. La cohérence des lectures est assurée dans le code par une transaction SQLite commune à la séquence et au chargement des données ; cette campagne n’est pas un test de charge concurrente.

Les calculs purs, tests d’interface et parcours desktop ont des portées complémentaires. Le pont de variation porte sur le sous-total d’actifs enregistrés, sans rendement calculé et sans suppléer les modules de trésorerie, de dettes ou de flux encore absents.
