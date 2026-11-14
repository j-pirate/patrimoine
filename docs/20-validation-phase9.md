# Validation — phase 9 : tableaux de bord et rapports

Campagne du **12 novembre 2026, Europe/Paris**, sur des coffres isolés et des données synthétiques. Le [guide des tableaux de bord](21-tableaux-de-bord.md) expose les conventions de consultation. Le schéma SQLite reste **6** et les six migrations sont inchangées. Les preuves des phases précédentes restent leurs campagnes historiques.

## Vérification complète

`source scripts/cloud-env.sh` puis `CARGO_BUILD_JOBS=3 bash scripts/check.sh` : **réussi, sortie 0**, sur les sources finales. Le [journal](/workspace/.tooling/check-phase9.log) conserve la campagne.

| Campagne | Résultat |
|---|---|
| TypeScript / Vite | Types et compilation réussis, modules embarqués localement |
| Vitest | **163 tests**, 18 fichiers, aucun échec ni ignoré |
| Domaine Rust | **219 tests**, dont 14 nouveaux tests de tableaux de bord |
| Infrastructure Rust | **189 tests**, dont 11 nouveaux tests de tableaux de bord et rapports |
| Workspace Rust | **408 tests réussis**, aucun échec ni ignoré |
| Secours CSV Python | **13 tests réussis**, conservés dans la vérification complète |
| Formatage / Clippy | `rustfmt` conforme ; toutes cibles et fonctionnalités, `-D warnings`, réussi |
| Tauri Linux | Binaire `custom-protocol` construit avec interface embarquée |

Recharts **3.10.1**, compatible avec React 19, est épinglé avec ses dépendances résolues dans le lockfile. Il est embarqué dans le module chargé à la demande. Aucune requête à un service graphique externe n’est nécessaire.

## Calculs et périmètres

Le nouveau service charge les huit snapshots métier dans une transaction SQLite de lecture. Il résout K une seule fois avant de les charger, puis libère la transaction avant les calculs. Bilans, séries, membres, locations et société utilisent ces mêmes sources immuables. Les requêtes à K=0, les anciens K, les entités inconnues et les bornes futures de connaissance sont testées. Les lectures et exports ne créent aucune mutation.

Le domaine réutilise les bilans financiers existants ; il ne revalorise pas les sources dans JavaScript. Les éliminations exactes sont réparties sur les positions positives de la même source et de la même face, au prorata de leur valeur comptable. Les actifs et passifs résiduels réconcilient les sous-totaux du bilan avant filtrage. Une élimination sans face correspondante ou excessive provoque un refus explicite. Les garanties conditionnelles restent visibles hors total.

Les filtres pays, devise d’origine, type, propriétaire associé et recherche interviennent après ces éliminations. Ils sélectionnent des positions dans le périmètre déjà calculé ; ils ne recalculent pas une consolidation juridique d’un pays ou d’un propriétaire. Le filtre de propriétaire conserve les positions entières associées, avec ce libellé explicite. L’identité propriétaire, les droits économiques et la devise ne sont pas confondus. Les profils descriptifs suivent D et restent bornés par K.

Les trois répartitions — type, pays, devise — agrègent les fractions exactes avant arrondi final. Les valeurs négatives, le zéro confirmé et les montants inconnus restent distincts. Masquer une position inconnue par filtre ne certifie pas la couverture du périmètre source. Les vues de membres proviennent de leurs propres bilans ; leurs actifs et passifs bruts ne sont pas additionnés pour remplacer les éliminations familiales.

Une estimation sélectionnée par sa révision exacte et âgée de plus de **365 jours** déclenche une alerte informative, aussi dans les points historiques. Les tests distinguent 365 et 366 jours et excluent les estimations futures ou non retenues. Cette alerte ne change aucun montant ni la couverture déclarée. Les autres alertes viennent des moteurs de bilan et de location, avec leurs références.

## Historique, sociétés et rapports filtrés

Le calendrier garde le début exact, les fins civiles de mois ou d’année intermédiaires, puis D exactement. Années bissextiles, période d’une date, doublons de bornes et budget de **61 points** sont exercés ; le point supplémentaire est refusé sans troncature. Tous les points utilisent K fixe. La différence entre bornes est calculée en rationnels ; une différence de stocks n’est pas qualifiée de performance. Le pont des estimations reste le moteur historique partiel du registre, hors filtres de positions, avec exclusion des stocks métal dérivés et sans reprise de leur ancienne estimation générique.

Les observations d’une société sont calculées par le rapport métier puis sélectionnées par date de fin dans `(début, D]`, sans prorata. Les distributions utilisent `[début, D]`. Comptes, droits, financements et méthode restent observés à D. Les fréquences et devises des observations coexistent sans sommation implicite. La vue société ne produit pas de patrimoine net fictif en additionnant des indicateurs ; les tests conservent aussi leur ancienne révision à K fixe.

Les indicateurs locatifs reprennent le rapport existant sur `[début, D)`, pays sélectionné compris ; les autres filtres de positions ne les recalculent pas. Pour une période d’une date, cette section est absente plutôt qu’affichée comme zéro. Le guide et l’interface distinguent cette portée de celle du bilan.

L’export JSON enregistre contexte, filtres, D/K, calendrier, versions, fractions, résultats et références. Avec des filtres actifs, les détails non filtrés du bilan source, des locations et du pont sont omis ; `excluded_sections` et une note expliquent ces absences. Sans filtre, les rapports sources du périmètre restent inclus. Un test recherche dans tout le JSON le nom d’un actif exclu et confirme son absence. Les tests vérifient destination neuve privée `0600`, publication atomique, refus des chemins internes au coffre, des liens symboliques et de l’écrasement. Ce rapport n’est pas une archive de restauration.

## Interface et recherche globale

Les **10 tests d’interface du tableau de bord** couvrent synthèses partielles, changement de contexte, K=0, erreurs, réponses périmées, filtres, société, annulation du dialogue et export. Les **7 tests de graphiques** vérifient lacunes du net, zéro, valeurs au-delà de la précision JavaScript, tableaux et sources, répartitions, différences individuelles négatives et fréquences/devises des sociétés. Les appels Recharts sont isolés dans cette campagne pour examiner les données et options transmises ; elle ne prétend pas remplacer le parcours WebKit réel.

Les nombres JavaScript servent uniquement à la géométrie des graphiques. Les infobulles et tables affichent les textes décimaux calculés par Rust, sans recalcul des montants. Les lacunes ne sont pas reliées artificiellement. Les sous-totaux connus disposent d’un mode distinct du net complet. Les graphiques ont une alternative tabulaire contenant valeurs et sources.

Les **6 tests de recherche globale** couvrent chargement à la demande, descriptions et accents, identifiants, objets retirés, profils datés, refus des réponses à un autre K, réponses périmées et routage d’une caisse physique. La recherche lit bibliothèque documentaire et trésorerie au même K explicite ; elle annonce son périmètre de coffre entier. Le résultat ouvre le module indiqué et conserve l’identifiant visible. Une caisse ouvre directement l’onglet des caisses, un compte bancaire ouvre les banques ; une ancienne identité de compte devenue inconnue à ce K renvoie à l’historique.

## Parcours Linux natif

`bash scripts/test-desktop-cloud.sh` : **réussi, sortie 0**, sur le binaire final. Le [rapport](../.local/desktop-smoke.PofeQrAi/desktop-result.json) contient **50 étapes réussies**, aucune échouée ni non exécutée. Debian 13, WebKitGTK/Xvfb, utilisateur non privilégié, réseau désactivé et coffre neuf sont conservés. Les 46 étapes antérieures, dont les deux voies de récupération SQLite et CSV, restent exécutées.

Les quatre nouveaux scénarios vérifient :

- Bilan familial à K source **120**, strictement identique au rapport financier enregistré avant réimport. Les positions résiduelles et les trois répartitions se réconcilient par `fractions.Fraction`. Les actifs connus restent **355 949,88 EUR**, les passifs connus **100 242,00 EUR**, la différence connue **255 707,88 EUR** ; les sources locatives manquantes gardent le patrimoine net non calculable. La garantie reste hors bilan. Les bornes et le nombre de lignes du tableau historique accessible correspondent aux points calculés. Voir la [synthèse familiale](../.local/desktop-smoke.PofeQrAi/dashboard-family.png).
- Filtre **Or** appliqué dans l’interface : actif connu **1 707,58 EUR**, sans nouvelle addition de l’ancienne estimation générique. Le net reste inconnu. Le dialogue Linux exporte réellement le [rapport filtré](../.local/desktop-smoke.PofeQrAi/dashboard-filtered-report.json), dont requête, positions et sections exclues sont vérifiées. Voir la [sélection](../.local/desktop-smoke.PofeQrAi/dashboard-filtered.png).
- Vue individuelle identique à la répartition du membre calculée par Rust, puis [pilotage de société](../.local/desktop-smoke.PofeQrAi/dashboard-company.png) sans cartes de net fictif. Une observation `12345678901234567890.1234` reste exacte. Les comptes et observations viennent des rapports existants.
- Recherche d’un lot par UUID puis ouverture du module métal ; recherche d’une caisse par UUID puis ouverture directe des **Caisses physiques**, où cette caisse est présente. Ces consultations et l’export conservent la génération et la séquence active **121** issue du réimport CSV.

Les [oracles du parcours](../.local/desktop-smoke.PofeQrAi/dashboard-oracles.json) conservent rapports familial, filtré, individuel et société. Les saisies passent par WebDriver ; les dates émettent leurs événements DOM. Aucun montant ni état React simulé n’est injecté. Un premier essai a identifié un sélecteur de pilote qui comptait aussi les lignes des autres tableaux de graphiques ; la campagne finale compte uniquement celles de l’historique, sans retirer l’assertion. Le correctif de routage des caisses est inclus dans le binaire final.

## Inspections indépendantes et non-régression de portabilité

Le [script indépendant](../.local/desktop-smoke.PofeQrAi/independent-dashboard-inspection.py), utilisant uniquement la bibliothèque standard Python, produit un [résultat réussi](../.local/desktop-smoke.PofeQrAi/independent-dashboard-inspection.json). Il lit les rapports et la source SQLite sauvegardée, reconstruit les réductions internes par face et par source en rationnels, puis vérifie positions résiduelles, répartitions, sélection, membres et export. Le calendrier, les sources, l’arithmétique des points et les bornes sont vérifiés ; cette inspection ne réimplémente pas l’intégralité du moteur historique. Les identifiants de certaines positions calculées de dépôt ou d’avance sont dérivés de leurs sources ; ils ne sont pas présentés comme des lignes persistées supplémentaires.

Les deux récupérations produisent encore les mêmes données du schéma 6 : **62 tables, 758 lignes sources, 132 objets, 144 révisions, deux documents et deux originaux**. Le [secours CSV exécuté](../.local/desktop-smoke.PofeQrAi/independent-csv-recovery.json) et l’[inspection des archives](../.local/desktop-smoke.PofeQrAi/independent-phase8-inspection.json) confirment l’identité des données et historiques à K=120, l’événement technique unique à K=121 dans le réimport applicatif, les originaux, les migrations, les références et les huit rapprochements bancaires. Aucun nouveau tableau de bord n’ajoute une table persistante ou une source financière.

L’[inspection XLSX](../.local/desktop-smoke.PofeQrAi/independent-xlsx-inspection.json) répète la comparaison des 62 tables avant et après réenregistrement réel par LibreOffice, avec conservation des CRLF et des valeurs exactes. Les limites du lecteur openpyxl et le décodage OOXML indépendant restent ceux documentés en [phase 8](18-validation-phase8.md).

## Limites

Les tableaux de bord décrivent les sources enregistrées, sans attester un inventaire externe absent. Les filtres de positions ne créent pas de nouveau périmètre juridique. Les sociétés restent des vues de pilotage et les résultats indicatifs ne deviennent pas des positions supplémentaires. Le pont partiel du registre ne constitue pas une décomposition complète de performance des nouveaux stocks financiers.

Le calendrier est limité explicitement à 61 points et la comparaison à 200 membres, sans omission silencieuse. La recherche globale couvre le coffre entier connu à K ; elle ne constitue pas un contrôle d’accès entre membres. Cette campagne n’est pas une qualification de volume maximal, d’accessibilité complète ou de paquet Linux. Ces travaux et AppImage restent la **phase 10**. Aucun changement de configuration cloud ni publication n’est effectué.
