# Validation indépendante — phase 3

Ce document distingue les critères prévus des commandes effectivement exécutées. Les fixtures sont synthétiques, créées dans des répertoires temporaires ; elles ne contiennent aucune donnée utilisateur. Le périmètre est celui de P3.1–P3.4 : registre historisé, droits datés, consolidation exacte EUR et contrats de dette du domaine pur. Le change, les tableaux de bord et la saisie des dettes appartiennent aux phases suivantes.

## Critères prévus avant exécution

| ID | Scénario | Preuve attendue |
|---|---|---|
| P3-R01 | Créer, corriger et rouvrir les enregistrements | Identités stables, nouvelles révisions, correction motivée, versions antérieures lisibles à leur séquence de connaissance. |
| P3-R02 | Révision attendue périmée, type de référence incorrect ou entrée invalide | Rejet sans changement des données, des révisions, de l'audit et de la séquence de mutation. |
| P3-G01 | Cycle actif seulement dans le passé ou le futur | Rejet atomique en contrôlant les frontières de toutes les périodes ; relations réciproques disjointes autorisées. |
| P3-G02 | Doublon, chevauchement et somme des droits supérieure à un | Rejet atomique, y compris lorsque le dépassement n'existe qu'à une frontière intérieure. |
| P3-N01 | Droit économique inconnu ou propriété incomplète | Droits inconnus conservés comme tels, aucun remplacement par le droit juridique ou par zéro, rapport incomplet explicite. |
| P3-N02 | Valorisation étrangère sans module de change | Valeur d'origine conservée ; conversion signalée indisponible, aucune parité inventée. |
| P3-H01 | Archiver un profil ou terminer une appartenance | Les droits et rapports historiques restent reproductibles ; une fin exclusive de périmètre s'applique à sa date. |
| P3-B01 | Sauvegarder puis restaurer un registre riche | UUID, révisions, chaînes décimales, dates, versions historiques et consolidation EUR identiques. |
| P3-M01 | Ouvrir un coffre réel du schéma 1 | Migration vers une génération 2 distincte, ancienne génération intacte, documents et UUID préservés, seconde ouverture idempotente. |
| P3-M02 | Source du schéma 1 invalide | Refus sans remplacement de la génération active ni mutation de l'original. |

Les calculs purs, l'interface et le parcours desktop sont des validations distinctes. Une compilation seule ne prouve ni un résultat patrimonial, ni l'interaction native.

## Résultats exécutés

Exécution indépendante observée le 4 novembre 2026 depuis `/workspace/patrimoine`, avec Rust 1.99.1 :

```bash
source scripts/cloud-env.sh
CARGO_BUILD_JOBS=3 cargo test -p patrimoine-infrastructure --test registry_adversarial --offline
CARGO_BUILD_JOBS=3 cargo clippy -p patrimoine-infrastructure --test registry_adversarial --offline -- -D warnings
rustfmt --edition 2021 --check crates/infrastructure/tests/registry_adversarial.rs
git diff --check
```

Résultat de la suite : **17 tests exécutés, 17 réussis, aucun échec ni test ignoré**, code de sortie 0. Clippy, formatage ciblé et contrôle du diff : réussite, code 0.

### Preuves obtenues par cette suite

- Création puis correction d’un profil : même identité, nouvelle révision et état conservé après fermeture/réouverture du coffre.
- Révision attendue périmée et tentative de changement de nature personne→société : refus sans mutation du registre ni des nombres d’objets, révisions, changements et événements d’audit.
- Cycles de sociétés actifs seulement dans le passé ou dans le futur : refus atomique. Les relations réciproques sur périodes disjointes, avec frontière commune exclusive, sont acceptées.
- Appartenances familiales chevauchantes et doublons de droits : refus. Appartenances adjacentes : acceptation sans double inclusion à leur frontière.
- Droits dépassant 100 % uniquement sur un intervalle intérieur : refus. Les sommes juridiques et économiques sont contrôlées séparément.
- Droit économique inconnu conservé comme `None`, sans substitution par son titre juridique ; sa confirmation ultérieure à `0/1` enlève l’alerte de droit inconnu, sans disparition de l’avertissement général sur les passifs hors périmètre.
- Valorisation en TND : devise et décimal d’origine conservés ; aucun montant EUR n’est inventé tant que le change n’est pas implémenté.
- Archivage d’un profil : aucune perte des droits ou de leur attribution économique. Une fin d’appartenance familiale s’applique à sa date exclusive.
- Correction rétroactive d’une valeur : 400 000 EUR à la séquence de connaissance d’origine, 450 000 EUR avec la connaissance actuelle, pour la même date économique.
- Annulation d’une personne encore référencée par une appartenance : refus sans laisser une référence métier orpheline.
- Sauvegarde/restauration du registre : mêmes UUID, révisions, fractions, dates, séquence de connaissance, pièce originale et empreinte. L’appartement de 400 000 EUR détenu à 50/30/20 % donne bien **320 000 EUR pour la famille, 200 000 pour A et 120 000 pour B** après restauration.
- Plusieurs chemins persistés vers une filiale : `(60 % × 50 % + 20 %) × 400 000 = 200 000 EUR`. Après passage de la société mère à une valeur globale nette de 500 000 EUR : `60 % × 500 000 + 20 % × 400 000 = 380 000 EUR`. Le chemin passant par la mère opaque est arrêté ; la détention directe de filiale reste visible.
- Archive hostile dont les empreintes ont été recalculées : réaffecter le profil d’Alice à l’identité existante de Bob garde des clés étrangères SQLite valides, mais est refusé par le validateur d’identité du registre avant activation. L’état actif reste inchangé.

Les migrations des anciens coffres, les tests purs de calcul, les autres tests de récupération, l’interface et le parcours desktop sont des campagnes complémentaires : leurs résultats doivent être consignés séparément par les commandes qui les exécutent. Les 17 tests ci-dessus ne sont pas présentés comme une validation de modules absents.

## Validation finale du lot

Commande exécutée dans le checkout le 4 novembre 2026 :

```bash
source scripts/cloud-env.sh
CARGO_BUILD_JOBS=3 bash scripts/check.sh
```

Résultat : code de sortie 0. TypeScript strict, Vite production, formatage Rust, Clippy sur tout le workspace/toutes cibles et fonctionnalités, puis compilation du binaire Linux avec interface embarquée : réussis.

| Suite | Exécutés | Réussis | Échecs / ignorés |
|---|---:|---:|---:|
| Nombres exacts du domaine | 5 | 5 | 0 / 0 |
| Attribution et contrats de prêts du domaine | 19 | 19 | 0 / 0 |
| Récupération et archives adverses du socle | 29 | 29 | 0 / 0 |
| Registre persistant indépendant | 17 | 17 | 0 / 0 |
| Migration de vrais coffres/archives du schéma 1 | 6 | 6 | 0 / 0 |
| Persistance/sauvegarde du socle | 4 | 4 | 0 / 0 |
| **Total Rust** | **80** | **80** | **0 / 0** |
| Interface Vitest / Testing Library | **22** | **22** | **0 / 0** |

Les cibles sans tests et les doc-tests vides sont séparés de ces comptes. Les tests métier utilisent des valeurs attendues : copropriété 50/30/20 %, chaîne 60 %×50 %, chemins multiples, tiers, fractions `1/3`, droits inconnus, bornes exclusives, changement de méthode, valeurs étrangères/futures, prêts internes et dettes personnelles. Une dette personnelle de 150 000 EUR pour A possédant 50 % d’un bien de 400 000 EUR donne bien un brut de 200 000, une dette de 150 000 et un net analytique de 50 000, sans répartition automatique suivant la propriété.

Les tests de migration conservent UUID, révisions et octets documentaires, vérifient l’équivalence structurelle des bases neuves et migrées, lisent les écritures WAL validées et gardent la génération source. Une ancienne archive est d’abord validée dans son schéma original, puis migrée dans la candidate ; une altération postérieure de la candidate est refusée.

Les tests UI couvrent les corrections attendues, l’inconnu distinct de zéro, les confirmations, le contexte date/personne, la lecture refusée, l’avis de migration et le retour au périmètre familial lorsqu’un membre sélectionné n’existe plus dans le coffre restauré. Une estimation non calculable affiche « Non calculable », sans présenter un faux zéro patrimonial.

### Parcours desktop réel, réseau coupé

Commande exécutée : `bash scripts/test-desktop-cloud.sh`, sous utilisateur non privilégié dans Debian 13 avec `--network none`. Le binaire final utilise la base schéma 2 et l’IPC Tauri réel. Aucun état financier n’est injecté dans React. La première personne, le premier actif et son estimation sont créés par les formulaires ; le reste de la fixture est créé par les commandes Rust réelles.

Résultat final : **14 étapes réussies**, code de sortie 0 :

- Rendu et statut du coffre, paramètres via l’interface, création de personne.
- Actif de 400 000 EUR à 50/30/20 % : famille 320 000, A 200 000, B 120 000.
- Deux sociétés avec un chemin 60 %×50 % vers un actif de 100 000 EUR : attribution indirecte de 30 000 à A ; total familial 350 000 et individuel A 230 000. Une valeur indicative des parts à 999 999 reste exclue en mode détaillé.
- Contrôle affiché avec ses sources et limite explicite sur les passifs non inventoriés.
- Import/export natifs d’un original identique, sauvegarde ZIP, analyse/annulation sans mutation et restauration confirmée avec sauvegarde préalable.
- Identités, révisions, fractions, valeurs, séquence de connaissance et total de 350 000 EUR reproduits après restauration ; captures enregistrées.

Artefacts de cette campagne : `.local/desktop-smoke.Gbwwmc9C/desktop-result.json`, `registry.png`, `desktop.png` et archives synthétiques. Ces fichiers sont ignorés par Git ; chaque nouvelle exécution crée un dossier distinct.

Disponibilité actualisée : ces originaux sont [conservés seulement dans l’environnement](preuves.md#disponibilité-par-campagne), hors ZIP distribué ; le présent compte rendu est la pièce versionnée de cette campagne.

La séquence à comparer est celle du snapshot effectivement sauvegardé : l’import documentaire est lui-même une mutation postérieure à la création des données patrimoniales. Le scénario vérifie que cette mutation ne change aucun enregistrement du registre, puis compare la restauration à l’état exact d’avant sauvegarde, séquence comprise. Le pilote attend également la stabilisation du rendu React entre touches WebKit, sans remplacer les valeurs par une injection d’état.

### Récupération indépendante vérifiée

L’archive finale a aussi été ouverte avec `zipfile` et `sqlite3` de Python, sans l’application : toutes les tailles et empreintes du manifeste sont vérifiées ; `integrity_check=ok`, aucune violation FK et `user_version=2`. Elle contient cinq entités, deux appartenances, quatre participations, deux actifs, quatre propriétés, trois estimations, un document et 23 révisions. Les montants `100000`, `400000` et `999999` sont des valeurs SQLite de type texte, en EUR ; les nombres de lignes correspondent au manifeste.

## Limites

Cette campagne ne simule pas une coupure électrique, un disque plein ou un processus hostile du même utilisateur remplaçant les répertoires du coffre. Elle ne prouve pas les modules de change, de location, de trésorerie, de saisie de dette ou d'export CSV/XLSX, hors périmètre de cette phase.
