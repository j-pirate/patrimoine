# Validation — phase 8 : documents et portabilité

Campagne du **12 novembre 2026, Europe/Paris**, sur des coffres isolés et des données synthétiques. Le [guide](19-documents-et-portabilite.md) décrit les commandes et les conventions de récupération. Le schéma SQLite reste **6** ; les six migrations antérieures sont conservées à l’identique.

## Vérification complète

`source scripts/cloud-env.sh` puis `CARGO_BUILD_JOBS=3 bash scripts/check.sh` : **réussi, sortie 0**. Le [journal](/workspace/.tooling/check-phase8.log) conserve la campagne exécutée sur les sources finales.

| Campagne | Résultat |
|---|---|
| TypeScript / Vite | Types et compilation réussis, interface et modules embarqués localement |
| Vitest | **140 tests**, 15 fichiers, aucun échec ni ignoré |
| Domaine Rust | **205 tests**, conservant les calculs des phases précédentes |
| Infrastructure Rust | **178 tests**, dont 12 documentaires, 9 CSV, 4 XLSX d’intégration, 4 XLSX unitaires et 2 de syntaxe CSV ajoutés dans cette phase |
| Workspace Rust | **383 tests réussis**, aucun échec ni ignoré ; autres cibles et doctests sans cas supplémentaires |
| Secours CSV Python | **13 tests** avec la bibliothèque standard, intégrés à `scripts/check.sh` |
| Formatage / Clippy | `rustfmt` conforme, toutes cibles et fonctionnalités avec `-D warnings` |
| Tauri Linux | Binaire `custom-protocol` construit avec interface embarquée |

Les tests UI emploient des réponses IPC contrôlées. Ils vérifient bibliothèque, filtres, versions, associations, confirmations, lecture seule historique, choix de fichiers, annulation, erreurs et réponses périmées. Le parcours natif ci-dessous exerce séparément les commandes réellement reliées à SQLite et les dialogues Linux.

## Documents et historiques

Les tests de persistance distinguent identité documentaire, révision et contenu physique. Deux documents logiques partageant les mêmes octets restent distincts. Un remplacement conserve l’identité, les associations, l’ancien contenu et la date initiale ; l’export historique retrouve les octets même après disparition du fichier source. Retirer une association ne supprime ni l’autre association ni la pièce.

Les statuts actif, archivé et retiré sont versionnés. Le retrait exige une confirmation, conserve toutes les versions et ne purge aucun original. Les révisions attendues empêchent une correction périmée. Un début d’association absent, une période invalide, une cible inconnue à la date de saisie ou une nouvelle association à une cible retirée sont refusés sans mutation partielle. Les liens historiques restent consultables.

La validation d’archive examine aussi les anciennes versions cachées par une correction, les liens et les cibles connus à leur séquence. Une archive dont les empreintes ont été recalculées ne peut pas introduire des métadonnées historiques invalides ou une association anticipant sa cible. Les événements documentaires apparaissent dans l’historique et le sélecteur K ; le motif initial utilise celui du changement lorsqu’aucun motif de révision n’existait dans les anciens imports.

La préparation d’ouverture crée une copie privée vérifiée, sans remettre au lecteur système le chemin de l’original stocké. Les liens symboliques et contenus altérés sont refusés. Les formats sans lecteur autorisé restent exportables. Le test natif n’ouvre pas de lecteur externe ; le transfert à `xdg-open` et la présence d’un lecteur sur chaque poste ne sont pas qualifiés par cette campagne.

## CSV intégral et reconstruction

L’archive contient les **62 tables persistantes**, même vides, leurs en-têtes et toutes les lignes historiques. UTF-8, guillemets, virgules, tabulations et fins de ligne sont préservés. Les marqueurs de nullité distinguent NULL, texte vide et zéro. Les nombres décimaux restent textuels ; les entiers SQLite sont contrôlés sans conversion flottante. La capture cohérente des exports porte sur le coffre entier, indépendamment des filtres de consultation.

Les tests reconstruisent une base vierge avec le schéma livré, des insertions paramétrées et les validations applicatives. Ils comparent toutes les tables, les identifiants, révisions, sources, réglages, audit et originaux. Après validation des données sources, la candidate reçoit exactement un changement et un événement d’audit `csv_imported`, sur l’espace existant, sans révision métier supplémentaire. Un coffre exporté à K conserve donc ses anciens états ; le réimport applicatif est à K+1.

Les refus exercés comprennent marqueur NULL contradictoire, en-tête inconnu, clé primaire dupliquée, table manquante même vide, références orphelines, montants financiers invalides, cycle de propriété historique, original absent, format futur et fichier supplémentaire. Des archives réempreintées sont employées pour vérifier les invariants au-delà des seuls hashes. Traversées de chemin, entrées ZIP dupliquées et liens symboliques sont rejetés. Une destination existante n’est pas écrasée ; un jeton d’analyse périmé ne peut pas activer sa candidate.

L’analyse et son annulation laissent le coffre actif intact. La confirmation utilise l’activation par générations et la sauvegarde préalable déjà utilisées pour SQLite. La première version CSV livrée prend en charge le schéma 6 ; les anciennes archives SQLite 1–5 gardent leur parcours de migration sur copie. Aucun SQL fourni par une archive CSV n’est exécuté.

## XLSX et lecture bureautique

Les tests lisent directement les fichiers OOXML du ZIP sans utiliser les fonctions de l’exporteur. Ils vérifient les tables, en-têtes, nombres de lignes, marqueurs NULL, historiques, dictionnaire et limites. Identifiants, dates et décimaux sont des cellules texte. Les chaînes commençant par `=`, `+`, `-` ou `@`, les URL et les motifs littéraux `_xHHHH_` conservent leur valeur sans formule, macro, lien actif ou relation externe.

Le découpage des feuilles est testé avec une limite réduite ; les en-têtes et le dictionnaire suivent chaque feuille. Les textes longs sont découpés selon la capacité UTF-16 des cellules, sans couper un caractère ni tronquer la source. Les fragments sont des chaînes JSON complètes et leur concaténation après décodage restitue exactement le texte. Les caractères XML interdits utilisent également ces fragments.

Une ouverture et un réenregistrement réels par LibreOffice ont révélé que des retours chariot encodés par référence XML étaient normalisés en saut de ligne. Le correctif final place aussi les textes contenant `CR` dans les fragments JSON. Un test de description courte avec CRLF vérifie sa référence, son contexte de révision et son décodage exact ; la comparaison indépendante après LibreOffice vérifie ce cas sur le classeur réel. Les originaux binaires restent dans les ZIP, le XLSX ne contenant que leurs métadonnées.

## Parcours Linux natif

`bash scripts/test-desktop-cloud.sh` : **réussi, sortie 0**. Le [rapport JSON](../.local/desktop-smoke.CNTvHv2v/desktop-result.json) contient **46 étapes réussies**, aucune échouée ni non exécutée. Le binaire final tourne dans Debian 13 sous WebKitGTK/Xvfb, avec utilisateur non privilégié, réseau désactivé et coffre neuf. Les 42 étapes antérieures restent exécutées.

Le scénario documentaire passe par les formulaires et dialogues natifs : description, deux associations, retrait d’un seul lien, remplacement d’original, export des anciens octets, second document partageant un blob, retrait confirmé et consultation historique en lecture seule. La [bibliothèque actuelle](../.local/desktop-smoke.CNTvHv2v/documents-current.png) et la [bibliothèque historique](../.local/desktop-smoke.CNTvHv2v/documents-historical.png) sont capturées. Une fixture complémentaire via IPC réel fournit la description avec formule littérale, CRLF, tabulation, accents et `_x0000_`, ainsi qu’un montant source exact `12345678901234567890.1234`, un zéro, du texte vide et des NULL.

Les deux voies de récupération sont exercées séparément :

1. Sauvegarde SQLite complète à **K=120**, analyse et annulation sans mutation, modification des réglages, confirmation avec sauvegarde préalable à K=121 et activation d’une nouvelle génération restaurant K=120. Les snapshots et rapports des phases précédentes, les documents et les anciens K sont comparés.
2. Export CSV et XLSX à **K=120**, analyse CSV et annulation, nouvelle analyse et confirmation avec sauvegarde préalable. La base reconstruite est activée à **K=121** avec sa provenance technique. Les neuf snapshots métier à K source sont identiques ; les [neuf rapports](../.local/desktop-smoke.CNTvHv2v/csv-roundtrip-reports.json), dont bilan, métaux, société, locations, financement et quatre comptes, sont intégralement identiques à K=120. Une nouvelle sauvegarde de cette génération permet la comparaison indépendante de ses tables.

Les saisies textuelles utilisent le clavier WebDriver ; les champs date émettent leurs événements DOM puis attendent la stabilisation. Les fixtures complémentaires appellent les commandes Tauri réelles. Aucun résultat financier simulé ni état React n’est injecté. Le pilote attend deux images avant les clics, attend explicitement le chargement documentaire après un changement de K et désactive le défilement animé pour les captures. Une première tentative a identifié cette attente manquante dans le pilote ; l’assertion de lecture seule est conservée dans la campagne finale. La restauration SQLite réussit en **34,07 secondes**, sauvegarde préalable et contrôles compris, avec un délai maximal de 60 secondes ; ces mesures ne constituent pas la qualification de performance de phase 10.

## Comparaisons indépendantes

Le [script d’inspection des archives](../.local/desktop-smoke.CNTvHv2v/independent-phase8-inspection.py) emploie seulement la bibliothèque standard Python : ZIP, SHA-256, SQLite, CSV et fractions rationnelles. Son [résultat](../.local/desktop-smoke.CNTvHv2v/independent-phase8-inspection.json) est **réussi**, sans appeler le backend ni exécuter de SQL reçu de l’archive.

La source contient **62 tables, 758 lignes, 132 objets, 144 révisions, deux documents logiques et deux contenus physiques**. Les inventaires, tailles, empreintes, six migrations, intégrité SQLite et clés étrangères sont vérifiés. Les **221 NULL**, **67 textes vides** et **320 entiers** sont conservés exactement. Les données incluent six versions documentaires et trois versions d’association, dont un retrait ; l’ancien contenu partagé reste présent. L’archive CSV manifeste 66 fichiers : 62 CSV, deux originaux, dictionnaire et notice, sans base SQLite incluse.

Toutes les valeurs typées des 62 tables du CSV et de la sauvegarde préalable au réimport sont identiques à la source SQLite. La sauvegarde post-réimport conserve exactement les données métier et leurs historiques ; seuls `change_sets` et `audit_events` gagnent chacun la ligne technique attendue. Les huit liens bancaires courants sont contrôlés par sources, révisions, lignes, dates, devises, titulaires, signes et capacités partagées ; les recalculs rationnels des banques, financement, dépôt, métal et dividende sont conservés.

Le [secours autonome](../scripts/recover-csv.py) est réellement exécuté sur cette archive, vers un fichier SQLite et un dossier de documents neufs. Le [résultat](../.local/desktop-smoke.CNTvHv2v/independent-csv-recovery.json) et l’inspection indépendante confirment les 62 tables et originaux identiques à K=120, sans événement ajouté dans cette copie externe. Ses 13 tests couvrent notamment schéma local altéré, SQL hostile dans le dictionnaire, versions futures, types, nullité, références, doublons, chemins et refus d’écrasement. Ce script de secours ne réexécute pas les invariants financiers Rust ; le réimport applicatif conserve ces contrôles.

Le [second inspecteur](../.local/desktop-smoke.CNTvHv2v/independent-xlsx-inspection.py) utilise XML/ZIP et openpyxl pour lire le classeur. Il reconstitue les marqueurs NULL et fragments, puis compare chaque valeur typée et l’empreinte canonique de chacune des 62 tables avec la source SQLite. Le [résultat XLSX](../.local/desktop-smoke.CNTvHv2v/independent-xlsx-inspection.json) consigne aussi l’ouverture et le réenregistrement réels par LibreOffice, puis la même comparaison intégrale sur le fichier réenregistré. La commande exécutée emploie LibreOfficeDev **26.8.0.0.alpha0** (`2c87e51eeaa2b413ff4ae097b2705eea1995d8e5`), filtre « Calc MS Excel 2007 XML », sortie 0 ; les 758 lignes des 62 tables sont identiques après réenregistrement. Le classeur exporté d’origine reste inchangé. Cette preuve porte sur cette version précise du lecteur.

L’inspection distingue une limitation observée d’openpyxl 3.1.5 sur les échappements SpreadsheetML de l’intégrité du fichier : le lecteur de chaînes partagées retire certains motifs `x005F_`. Les valeurs canoniques sont donc décodées directement depuis l’OOXML, une seule fois selon la convention SpreadsheetML, et comparées strictement ; aucune normalisation des CRLF perdus ou des textes sources n’est admise. La présence d’un second lecteur ne remplace pas cette comparaison.

## Limites

La suppression documentaire reste logique. Le XLSX est un format de consultation, sans réimport applicatif ni originaux binaires embarqués. Les CSV conservent les textes sources, y compris les préfixes de formule ; le XLSX fournit les cellules textuelles adaptées à la consultation bureautique. Les limites de taille documentées provoquent un refus explicite, sans troncature.

Les archives sont en clair ; les empreintes ne constituent pas une signature d’authenticité. Cette campagne synthétique ne qualifie ni tous les lecteurs Linux ni un volume maximal de production. Les tableaux de bord restent la phase 9 ; AppImage et performances la phase 10. Aucun changement de configuration cloud ni publication n’est effectué dans ce lot.
