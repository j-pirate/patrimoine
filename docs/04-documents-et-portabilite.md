# Documents, export, sauvegarde et restauration

Statut : proposition de conception de phase 1. Aucun mécanisme décrit ici n’est encore implémenté ou validé par un test logiciel. Les noms des tables suivent le [modèle relationnel](02-modele-relationnel.md).

## 1. Contrat de récupération

Les données ne dépendent d’aucun compte, service, format propriétaire ou installation de l’application. Quatre sorties ont des fonctions distinctes :

| Sortie | Contenu | Usage et limites |
| --- | --- | --- |
| Sauvegarde ZIP complète | SQLite cohérent, originaux documentaires, manifeste, empreintes | Restauration fidèle, historique et relations compris ; chemin prioritaire de secours. |
| Archive CSV structurée | Toutes les tables persistantes utiles, dictionnaire, relations, originaux documentaires, empreintes | Reconstruction dans une base vierge ; format d’échange réimportable, versionné. |
| Classeur XLSX | Feuilles lisibles de toutes les données métier, relations et dictionnaire | Consultation et exploitation bureautique ; aucun import XLSX implicite. Les originaux documentaires restent dans les archives ZIP. |
| Copie d’un document | Octets originaux et nom choisi par l’utilisateur | Consultation indépendante ; ni conversion ni filigrane. |

Une somme SHA-256 détecte une altération par rapport au manifeste. Un manifeste non signé ne garantit pas l’authenticité d’une archive malveillante ; les contrôles structurels et métier restent obligatoires. La V1 n’ajoute pas de chiffrement propriétaire. Les permissions locales et le chiffrement de disque du système constituent la protection initiale ; une éventuelle protection d’archive ultérieure devra utiliser un standard documenté et récupérable hors application.

## 2. Emplacements Linux et générations

Emplacements indépendants de l’AppImage et du dépôt :

```text
${XDG_DATA_HOME:-~/.local/share}/patrimoine/
  CURRENT                         # nom relatif d’une génération, texte UTF-8
  generations/<uuid>/
    patrimoine.sqlite3            # SQLite standard, données métier et historique
    documents/sha256/ab/<hash>     # originaux immuables ; ab = préfixe du hash
  staging/<uuid>/                 # imports/restaurations en validation
  recovery/                      # journal d’activation et métadonnées de reprise
${XDG_CONFIG_HOME:-~/.config}/patrimoine/
  preferences.json               # préférences non métier, versionnées
${XDG_STATE_HOME:-~/.local/state}/patrimoine/
  logs/                          # erreurs locales avec données sensibles minimisées
${XDG_CACHE_HOME:-~/.cache}/patrimoine/
  thumbnails/                    # caches reconstruisibles, exclus des sauvegardes
```

Les valeurs XDG ne sont utilisées que si elles sont absolues ; sinon retour au chemin par défaut. `CURRENT` contient exclusivement un identifiant de génération validé, jamais un chemin libre. Le répertoire de données est résolu une fois ; tous les accès sensibles sont ensuite confinés à sa racine. Répertoires privés `0700`, fichiers `0600`, sous réserve des capacités du système de fichiers. Aucun original n’est conservé seulement dans `/tmp`, le cache ou le dossier d’installation.

Une seule instance détient le verrou d’écriture du coffre. Les lectures peuvent être concurrentes, les écritures sont sérialisées par le service Rust. SQLite utilise les clés étrangères activées sur chaque connexion, WAL et synchronisation durable (`synchronous=FULL`) sur système de fichiers local. Les performances seront mesurées avant tout assouplissement ; un partage réseau n’est pas un support V1 garanti.

Les générations isolent une restauration de l’état actif. Une génération active reste modifiable par les transactions normales ; elle n’est pas un snapshot immuable. Lors d’une activation, son état arrêté est conservé comme récupération. L’ancien état n’est pas supprimé avant vérification et politique de rétention explicite. Un identifiant de génération est attaché aux commandes : aucune commande en attente ne doit écrire dans l’ancienne génération après bascule.

## 3. Import et cycle de vie documentaire

`document_blobs` décrit un contenu physique immuable (SHA-256, taille, chemin relatif contrôlé). `documents` identifie le document logique ; `document_versions` conserve nom original, type, description et versions successives. `document_links` relie le document à un ou plusieurs `objects` par clés étrangères réelles. Deux documents logiques peuvent partager le même blob sans perdre leurs descriptions ou associations.

Procédure d’ajout :

1. L’utilisateur choisit un fichier local ; contrôler qu’il s’agit d’un fichier ordinaire, sa taille et l’espace disponible. Aucun import récursif de répertoire ou suivi de lien inattendu.
2. Copier en flux vers un fichier temporaire privé dans le stockage cible en calculant SHA-256 et taille ; détecter toute modification de source qui rendrait la copie incohérente. Le hash décrit les octets effectivement importés.
3. Synchroniser le fichier, déplacer atomiquement vers le chemin dérivé du hash, synchroniser le répertoire. Si le hash existe, vérifier le contenu et réutiliser le blob. Un nom fourni par le fichier ne choisit jamais le chemin interne.
4. Insérer blob, document, version et associations dans une transaction SQLite avec l’audit. Si elle échoue, un blob orphelin peut subsister ; aucune ligne validée ne doit pointer vers un fichier absent.

Le type MIME et l’extension sont des métadonnées, pas une autorisation d’exécuter. Les prévisualisations restent limitées aux formats traitables sans code actif. L’ouverture externe nécessite une action explicite et une copie de consultation à extension adaptée, confiée au système ; pas de commandes shell assemblées à partir du nom original. Les exports vérifient les noms de destination et évitent les écrasements silencieux.

Retirer une association n’efface pas le document. Supprimer un document demande confirmation et crée une version retirée ; les versions historiques et leurs blobs restent récupérables. Un remplacement de pièce crée une nouvelle version et un nouveau hash. Une purge physique, distincte et explicitement confirmée, n’est permise que pour un blob sans référence dans les versions conservées, les générations de récupération et les snapshots en cours. La V1 privilégie la conservation ; les sauvegardes déjà exportées ne peuvent pas être effacées à distance.

## 4. Snapshot commun à tous les exports

Les sauvegardes et exports utilisent le même service de snapshot afin d’obtenir le même état de base et de documents :

1. Prendre une barrière de mutations du coffre, attendre les imports et transactions en cours, puis interdire temporairement nouvelles écritures, changements de génération et purges.
2. Produire un SQLite de destination au moyen de **SQLite Online Backup API** (par exemple via l’API `backup` de `rusqlite`) ; ne jamais copier simplement le fichier vivant, ni oublier un WAL.
3. Dans ce snapshot, relever les blobs référencés, y compris par les versions historiques, et épingler cet ensemble contre toute purge. Vérifier l’intégrité SQLite et la clôture des références.
4. Relâcher la barrière après sauvegarde SQLite terminée et épinglage enregistré. L’application peut reprendre ses écritures ; les blobs épinglés sont immuables. Lire exclusivement le SQLite du snapshot pour générer CSV/XLSX et manifeste.
5. Copier chaque blob requis, vérifier taille et hash, puis finaliser les fichiers. Une pièce manquante fait échouer la sauvegarde complète avec diagnostic précis ; aucune réussite partielle silencieuse.
6. Écrire l’archive dans un fichier temporaire voisin de la destination choisie, terminer le ZIP, synchroniser, puis renommer atomiquement. En cas d’erreur, conserver l’ancien fichier exporté et supprimer le temporaire contrôlé ; libérer les épingles.

La barrière couvre toute la création SQLite en V1, choix simple et vérifiable. Une optimisation future pourra réduire sa durée à condition de prouver la même cohérence. Les documents retirés encore référencés par l’historique sont inclus ; les caches, verrous, fichiers WAL et temporaires ne sont pas des données d’archive. Les préférences métier sont dans SQLite ; les préférences d’interface utiles peuvent être exportées dans un JSON explicite, jamais avec des secrets ou chemins machine imposés à la restauration.

## 5. Format de sauvegarde complète

```text
sauvegarde-patrimoine-<date>-<uuid>.zip
  manifest.json
  data/patrimoine.sqlite3
  documents/sha256/ab/<hash>
  recovery/LISEZMOI.md
  dictionary/schema.json
```

Le manifeste contient au minimum : `archive_kind=full_backup`, `format_version`, `schema_version`, `app_version`, `calculation_policy_version`, UUID d’export, date UTC ISO 8601, génération d’origine, compteur de mutation capturé, liste exhaustive des fichiers avec taille décompressée et SHA-256, nombres de lignes par table, nombres de documents logiques et de blobs distincts, conventions des dates/montants/unités. La liste des fichiers inclut tout sauf le manifeste lui-même, qui ne peut pas se hacher récursivement. Un fichier annexe d’empreinte du manifeste peut être fourni, sans prétendre à une signature.

Le SQLite de sauvegarde est autonome, sans dépendance à un `-wal`, à une extension privée ou à un mot de passe connu seulement de l’application. Les métadonnées documentaires désignent des chemins relatifs recalculables à partir du hash. Le dictionnaire décrit aussi les clés primaires, étrangères, contraintes et conventions historiques. `backup_runs` trace les tentatives locales et leur résultat ; le journal de succès de l’export est nécessairement postérieur au snapshot et ne fait pas partie de son contenu. Cette différence technique est documentée lors des comparaisons.

## 6. Archive CSV réimportable

Format distinct `archive_kind=structured_csv` ; même snapshot et même sélection de documents que la sauvegarde complète. Une table persistante par fichier `tables/<table>.csv`, y compris les tables vides avec leurs en-têtes. Inclure toutes les identités, versions, changements, dates, participations, propriétés, valorisations, flux, financements, réglages métier, métadonnées documentaires, associations et audit nécessaires à reconstruire l’état et son histoire. Les caches calculés sont exclus et explicitement identifiés ; les tables techniques non rejouables (verrous, état temporaire, journal d’export post-snapshot) sont décrites comme telles dans le dictionnaire.

Contrat tabulaire version 1 :

- UTF-8 sans BOM, séparateur virgule, guillemets doubles et échappement des guillemets selon RFC 4180 ; fins de ligne CRLF ; aucune convention dépendant de la langue de la machine.
- En-têtes stables identiques aux noms techniques ; ordre des colonnes fixé par le dictionnaire ; lignes ordonnées par clé primaire pour un résultat déterministe. Dates civiles `YYYY-MM-DD`, instants UTC ISO 8601 avec précision conservée.
- Décimaux exacts sous forme textuelle canonique avec point et sans séparateur de milliers ; aucun passage par un `float` ou un nombre JavaScript. Identifiants et hashes restent des chaînes ; devises et unités explicites.
- Pour chaque colonne nullable, une colonne auxiliaire `<nom>__is_null` vaut `0` ou `1`. Une valeur nulle est vide dans la colonne métier et marquée `1` ; une chaîne vide réelle est marquée `0`. Le dictionnaire identifie ces auxiliaires, dont les noms sont réservés et absents du schéma métier. Ainsi NULL, vide, zéro et chaîne littérale sont réversibles sans sentinelle ambiguë.
- Les cellules textuelles sont exportées sans modification, même si elles commencent par `=`, `+`, `-` ou `@`. L’archive CSV est une source de données à importer avec types explicites ; le lecteur d’accompagnement recommande le XLSX pour une consultation bureautique sans interprétation de formules. Préfixer les valeurs dans l’export réimportable détruirait leur identité.

`manifest.json` fournit version d’export et de schéma, fichiers, empreintes, nombres de lignes, encodage, dialecte, ordre des tables, clés et relations ; `dictionary/schema.json` documente chaque colonne, type logique, nullabilité, unité, monnaie, règle de référence, contrôle et version d’introduction. Une notice lisible explique les relations en français. Les originaux sont présents dans `documents/` ; un export tabulaire volontairement sans pièces, s’il est ajouté plus tard, doit porter une désignation différente et signaler qu’il n’est pas une récupération intégrale.

## 7. Classeur XLSX de consultation

Feuilles métier séparées couvrant le catalogue du cahier des charges, y compris valeurs historiques, lots, mouvements, échéanciers, remboursements, paiements/allocation d’échéances, garanties, participations et associations. Une feuille `Dictionnaire` relie les noms français courts (limite Excel de 31 caractères) aux tables et colonnes techniques. `Informations` rappelle la date de snapshot, les versions, les conventions et le périmètre. Si une table dépasse la limite de lignes d’une feuille, créer des feuilles numérotées, référencées dans le dictionnaire ; ne jamais tronquer silencieusement. Les textes dépassant la limite de 32 767 caractères d’une cellule sont répartis dans des feuilles `Textes longs` avec table, clé primaire, colonne et numéro de fragment ; la cellule d’origine renvoie explicitement à cette représentation réversible. Contrôler aussi les caractères incompatibles XML et les encoder de manière documentée dans ces fragments, sans suppression silencieuse.

Les identifiants, dates ISO et décimaux exacts sont écrits comme **cellules texte explicites**. Les données utilisateur ne deviennent jamais des formules, macros, URL actives ou liens externes automatiques. Des colonnes numériques d’aide à l’analyse peuvent être proposées, clairement arrondies et non canoniques ; la colonne exacte reste toujours présente. Pas de formule servant de source de vérité financière. Le fichier contient toutes les métadonnées des pièces, mais ne prétend pas embarquer les originaux. Validation de lecture avec LibreOffice Calc et un lecteur XLSX indépendant, notamment pour accents, longues notes, décimaux, zéros initiaux et valeurs commençant par `=`.

## 8. Restauration complète sécurisée

La restauration remplace un coffre entier, jamais quelques fichiers de l’état actif. Deux étapes visibles : **analyser la sauvegarde**, puis **confirmer la restauration**.

### Validation sans activation

1. Lire le manifeste avec des limites de taille et de profondeur JSON ; vérifier le type, le format, les fichiers annoncés et les versions. Refuser un schéma plus récent que celui compris par l’application. Un ancien schéma compatible est migré exclusivement dans la zone de préparation.
2. Estimer l’espace nécessaire pour archive extraite, migrations, état actuel et récupération ; vérifier les droits. Fixer des limites explicites de fichiers, tailles individuelles/totales et ratio de décompression, configurables après diagnostic. Une archive bombée ou dépassant les limites ne doit jamais être extraite sans contrôle.
3. Extraire dans une nouvelle zone privée sur le même système de fichiers que `generations/`. N’accepter que les chemins relatifs normalisés du format : ni `..`, ni chemin absolu, ni préfixe lecteur, ni NUL, ni doublon normalisé, ni lien symbolique/dur, périphérique, socket ou nom ambigu. Vérifier le confinement au moment de chaque ouverture sans suivre de lien ; ne pas se contenter de comparer des chaînes de préfixe.
4. Vérifier liste exhaustive, tailles et SHA-256 en flux. Tout fichier inattendu, absent ou altéré est un échec. Ne jamais exécuter SQL, script ou migration fourni par l’archive ; seules les migrations livrées avec l’application sont admises.
5. Ouvrir SQLite comme entrée non fiable, sans chargement d’extension, avec schéma attendu vérifié avant lecture métier, `trusted_schema=OFF` et requêtes préparées. Exécuter `integrity_check`, `foreign_key_check` puis les validations métier Rust : décimaux, devises, dates, versions, pourcentages et intervalles, absence de cycle à toute date de changement, cohérence des flux et allocations, unicité des identifiants, documents et relations. Un total financier identique ne remplace pas ces contrôles.
6. Si une migration locale est nécessaire, conserver l’entrée intacte, migrer une copie dans une transaction appropriée, puis recommencer les vérifications. Produire un résumé : date/version, membres, sociétés, actifs, nombres de lignes, documents, différences de schéma, alertes et indicateurs de référence calculables. Des données métier incomplètes explicitement déclarées sont distinguées d’une corruption structurelle.

### Confirmation et activation atomique

1. Présenter le résumé et proposer une sauvegarde complète de l’état actuel, activée par défaut ; laisser annuler sans effet. Une décision explicite de ne pas exporter cette sauvegarde ne supprime pas la génération locale de récupération.
2. Lier la confirmation à l’empreinte de l’archive, à la génération active et à son compteur de mutation. Si le candidat ou l’état courant change entre analyse et confirmation, refaire l’analyse ou présenter l’écart avant activation. Revérifier ces jetons sous la barrière d’activation pour éviter de remplacer des écritures postérieures à la confirmation sans nouvelle décision. L’interface indique clairement le remplacement intégral et n’efface rien avant cette confirmation.
3. Prendre le verrou d’activation et la barrière de mutations ; achever/annuler proprement les travaux en cours, fermer toutes les connexions à la génération active après checkpoint SQLite. Écrire un journal de reprise contenant anciens/nouveaux identifiants et phase, puis synchroniser fichiers et répertoires.
4. Déplacer la génération candidate validée dans `generations/<nouvel-uuid>`, synchroniser ses fichiers et son répertoire. Écrire `CURRENT.new` avec son UUID, synchroniser ce fichier, remplacer `CURRENT` par renommage atomique, synchroniser le répertoire parent. Tous ces renommages restent sur le même système de fichiers.
5. Rouvrir les connexions, vérifier la génération et une requête fonctionnelle, reconstruire les caches, puis annoncer le succès et clôturer le journal de reprise. Ne supprimer l’ancienne génération ni au commit ni au premier redémarrage ; fournir une action ultérieure de nettoyage avec rétention documentée.

Une interruption avant remplacement du pointeur laisse l’ancien coffre actif. Après remplacement, le pointeur désigne soit l’ancien soit le nouveau nom complet, jamais une moitié ; le journal et les générations retenues permettent d’achever ou revenir en arrière. Au démarrage, une génération absente ou illisible produit un écran de récupération, sans créer silencieusement une base vide. Si l’ouverture du nouveau coffre échoue, restaurer atomiquement l’ancien pointeur et signaler la restauration non aboutie. Les garanties de durabilité reposent sur le renommage atomique et `fsync` du système de fichiers local ; elles devront faire l’objet de tests d’injection de panne.

## 9. Réimportation CSV dans une base vierge

Cette opération est distincte de la restauration SQLite : elle reconstruit les tables avec le schéma livré et des `INSERT` paramétrés ; elle n’exécute aucun DDL du fichier importé. V1 : pas de fusion avec le coffre actif et pas de résolution automatique de doublons entre coffres.

Après les contrôles communs d’archive, créer un SQLite vide dans une génération candidate, appliquer les migrations approuvées, puis importer dans une transaction globale avec clés étrangères différées lorsque nécessaire. Pour un ancien format CSV supporté, utiliser un adaptateur de version livré avec l’application ou importer dans le schéma ancien approuvé puis migrer la candidate ; ne jamais injecter des colonnes anciennes sans correspondance dans le schéma courant. Respecter l’ordre topologique des dépendances ; gérer les références de révision et l’historique sans fabriquer de nouvelles identités. Une clé dupliquée, une référence inconnue, un champ manquant, une colonne inconnue non prévue par la version, un décimal non canonique ou une nullabilité contradictoire font échouer l’ensemble avec table, ligne, colonne et cause. Ne pas désactiver les contraintes pour accepter un fichier incohérent.

L’importeur conserve les dates de saisie d’origine et les UUID ; la provenance de l’import est ajoutée dans un nouvel événement technique. Il valide les mêmes invariants et les mêmes pièces que la restauration SQLite. Comparer nombres d’enregistrements et empreintes canoniques des données aux déclarations du manifeste. Le manifeste ne sert jamais à contourner une règle métier. Recalculer les indicateurs à plusieurs dates, puis présenter le résumé et utiliser exactement la même procédure de confirmation/activation que pour la restauration intégrale. En cas d’échec, supprimer uniquement la candidate et garder le coffre courant intact.

## 10. Récupération sans l’application

La notice incluse dans chaque archive fournit ces opérations, avec les noms réels de l’archive :

1. Conserver une copie originale de la sauvegarde. Lister les fichiers du ZIP avec un outil standard, puis extraire une archive de confiance dans un dossier neuf. Pour une source inconnue, appliquer un extracteur protégé contre les chemins dangereux avant ouverture.
2. Ouvrir `data/patrimoine.sqlite3` avec `sqlite3`, DB Browser for SQLite ou un outil équivalent. Le dictionnaire fournit les relations et la convention de décimaux textuels ; aucun module de l’application n’est nécessaire pour lire les tables.
3. Pour lire les fichiers de travail plutôt qu’une archive, fermer d’abord l’application, lire l’UUID dans `CURRENT`, puis ouvrir la base correspondante. Si SQLite doit être copié pendant l’exécution, utiliser la commande standard `.backup` ou l’API de sauvegarde, jamais `cp` du seul fichier principal. Pour récupérer également les pièces cohérentes, préférer une sauvegarde complète.
4. Dans un ZIP CSV, lire `manifest.json` et `dictionary/schema.json`, puis importer les tables comme UTF-8/virgule avec les types textuels appropriés. Les colonnes `__is_null` restituent les valeurs nulles distinctes des chaînes vides. Les relations sont explicites par identifiants, même sans SQLite.
5. Ouvrir le XLSX avec LibreOffice Calc ; conserver les colonnes exactes en texte avant calculs bureautiques. Un tableur peut arrondir les grandes valeurs lorsqu’elles sont converties en nombres.
6. Les originaux sont dans `documents/sha256/`. Associer `document_versions.blob_sha256` à `document_blobs.sha256`, puis utiliser `original_name` pour exporter une copie nommée à convenance. Plusieurs noms peuvent correspondre au même contenu. Vérifier les SHA-256 avec un outil standard ; PDF, images et autres pièces gardent leurs octets d’origine.

Les montants ne doivent pas être additionnés naïvement avec les opérations SQLite qui convertissent les textes en flottants : la lecture indépendante est immédiate, les calculs exacts nécessitent un outil décimal ou rationnel standard et les règles publiées dans [les règles métier](03-regles-metier.md).

## 11. Migrations et reprise

Chaque migration porte un numéro, une description et une empreinte dans `schema_migrations`. Avant toute migration de données existantes, produire une sauvegarde SQLite cohérente et conserver les pièces référencées ; une migration destructive ou impliquant les documents exige une sauvegarde complète vérifiée. Ne jamais migrer la seule copie disponible.

Une transaction enveloppe les opérations SQLite compatibles. Pour une migration multi-étapes ou touchant des fichiers, utiliser une génération candidate et la même activation sûre que la restauration. Ne pas promettre un retour arrière SQL universel : restaurer la génération/sauvegarde d’avant migration. Un échec laisse les données d’origine accessibles et affiche la version et la cause ; aucun effacement ni réinitialisation automatique. Journaliser les résultats sans copier les adresses, numéros de compte ou contenus documentaires dans les logs.

## 12. Validation attendue avant livraison

Fixture commune : trois membres, une personne extérieure, deux sociétés en chaîne et deux chemins indirects, copropriété, immeuble et unités, baux successifs, échéances partiellement réglées, comptes EUR/TND/CHF/AED et caisse, transferts et conversion avec frais, prêt familial et dette externe, or/argent et vente partielle, taux et valorisations datés, changement de périmètre et correction rétroactive. Inclure documents partagés, versions remplacées et retirées, noms accentués, contenu identique sous deux noms, pièce binaire, chaîne vide/NULL, UUID stables et décimaux de précision maximale autorisée.

| Test prioritaire | Preuve attendue |
| --- | --- |
| SQLite → ZIP → coffre vierge | Mêmes lignes, identifiants, relations, valeurs, historiques et hashes ; mêmes résultats exacts à plusieurs dates. |
| CSV → ZIP → SQLite vierge | Même comparaison indépendante ; aucune perte des NULL, notes multilignes, décimaux et liens. |
| Export XLSX | Toutes les feuilles attendues, aucun tronquage, chaînes/formules adverses inertes, lecture LibreOffice et lecteur indépendant. |
| Snapshot sous mutations | Les nouvelles opérations apparaissent soit avant soit après le snapshot ; aucune pièce manquante, aucun demi-transfert. |
| Activation interrompue | Pannes injectées avant/après chaque écriture, synchronisation et renommage : ancien ou nouvel état complet, jamais un mélange ni base vide. |
| Archive hostile | Traversée, chemin absolu, doublons, liens, bombe, faux hash, fichier supplémentaire, schéma inattendu et SQLite malformé refusés avant activation. |
| CSV invalide | Identifiant dupliqué, FK orpheline, dates incompatibles, taux nul, cycle historique et pourcentages incohérents refusés ; coffre inchangé. |
| Espace/droits/panne disque | Échec explicite, archive précédente et coffre préservés, nettoyage limité aux temporaires de l’opération. |
| Compatibilité | Ancien format supporté migré en candidate ; format/schéma futur refusé ; aucune migration reçue de l’archive exécutée. |
| Reprise indépendante | Une personne disposant seulement du ZIP, de la notice et d’outils standards retrouve tables et pièces originales. |

Comparer le contenu logique canonique, pas les octets du fichier SQLite (ordre des pages variable). Les caches, l’identifiant de génération et les nouveaux événements de restauration/import sont des différences techniques attendues, isolées et explicitement vérifiées ; aucune différence métier n’est acceptée. Ces tests seront implémentés progressivement dès la phase 2 et complétés en phase 8. Aucun résultat de test n’est revendiqué à la phase 1.
