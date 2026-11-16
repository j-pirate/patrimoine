# Revue des améliorations possibles

Revue commencée le **14 novembre 2026, Europe/Paris**, à la demande de l’utilisateur. Référence de lecture : commit `af8c88c319245454ec25c1966f0715198185f952`.

Ce fichier contient des observations et des propositions. Il ne met en œuvre aucune correction. Les correctifs issus de la revue restent différés jusqu’à son achèvement. Exception explicitement demandée le 15 novembre : retrait du paquet `.deb`, de ses liens et de sa génération, en conservant l’AppImage et les archives de validation. La reconstruction compatible Pop!_OS du 14 novembre est intervenue entre les parties 4 et 5, sans changement des calculs, dépendances verrouillées ou migrations. Les correctifs personnels annoncés par l’utilisateur seront intégrés à réception du fichier texte.

## Point de reprise

- **Dernière partie terminée : 10 — Tests transversaux, outils, CI, dépendances et maintenance.**
- **Revue des dix parties achevée le 16 novembre 2026, Europe/Paris. Prochaine phase : préparer les corrections en intégrant le fichier personnel annoncé ; aucun correctif de revue appliqué.**
- La partie suivante commence seulement après la clôture de la partie en cours.
- « Terminée » signifie que le périmètre de revue et ses observations sont consignés, pas que les améliorations sont réalisées.

## Ordre de la revue

| Partie | Périmètre | État |
|---|---|---|
| 1 | Documentation, cahier des charges et explications | Terminée |
| 2 | Architecture applicative, contrats et primitives de calcul | Terminée |
| 3 | Registre, propriété, consolidation, devises et historique | Terminée |
| 4 | Immobilier et loyers | Terminée |
| 5 | Trésorerie, financements et bilan financier | Terminée |
| 6 | Métaux précieux et sociétés | Terminée |
| 7 | Documents, sauvegardes et portabilité | Terminée |
| 8 | Tableaux de bord, recherche et composants d’interface communs | Terminée |
| 9 | SQLite, migrations, fichiers et interface native Tauri | Terminée |
| 10 | Tests transversaux, outils, CI, dépendances et maintenance | Terminée |

## Lecture des observations

- **P1 — Important :** peut affecter l’intégrité, l’interprétation financière, la récupération, la sécurité ou l’usage courant.
- **P2 — Utile :** améliore la compréhension, la maintenance ou les parcours.
- **P3 — Confort :** simplification ou amélioration secondaire.
- **Constat :** comportement ou contenu observé dans les références citées.
- **Proposition :** amélioration possible ; son absence ne constitue pas à elle seule un défaut.
- **À vérifier :** hypothèse qui nécessite un contrôle complémentaire avant de conclure à un défaut.

Les preuves de qualification antérieures ne sont pas présentées comme des tests relancés pendant cette revue. Les contrôles effectués spécifiquement ici sont indiqués dans chaque partie.

## Partie 1 — Documentation, cahier des charges et explications

Périmètre : `README.md`, `distribution/README.md`, les 28 fichiers Markdown de `docs/` antérieurs à cette revue, dont le cahier des charges, les guides, décisions et rapports des phases 2 à 10.

Contrôle effectué : **220 liens locaux examinés**, aucune destination manquante dans cet environnement ; **77 références pointent vers des fichiers non suivis par Git**. Les copies adaptées incluses dans le ZIP de documentation ont une portée distincte des pages Markdown affichées sur GitHub. Les tests financiers cités dans les rapports n’ont pas été relancés pour cette partie.

### DOC-01 — Clarifier le net familial lorsqu’une créance interne est dépréciée

**P2 · Ambiguïté documentaire ; piste de bug du moteur écartée en partie 5.** [Règles initiales](../technique/regles-metier.md#L71), [éliminations](../technique/regles-metier.md#L148), [décision de financement](../technique/decisions-architecture.md#L90).

Les règles initiales annoncent un net familial égal à la somme des nets individuels et une élimination identique sur les deux faces. La décision ultérieure retient un appariement nominal et une réduction de la créance proportionnelle à sa valeur comptable. Pour un prêt familial nominal de 100 000 EUR déprécié à 80 000 EUR, les deux conventions ne donnent pas nécessairement le même résultat consolidé.

**Idée :** ajouter cet exemple chiffré à la décision, indiquer explicitement les réductions de chaque face et les conditions de l’égalité des nets. Contrôle de partie 5 : `finance.rs:998–1005` apparie le nominal, calcule la valeur comptable commune puis retranche **la même somme des deux côtés**. Pour 100 000 de nominal et 80 000 de créance : élimination 80 000 sur chaque face, créance résiduelle 0, dette résiduelle 20 000, net familial −20 000, égal à la somme des nets individuels. Le test `creditor_impairment_leaves_an_unmatched_debtor_residual` conserve ce reliquat. Préciser ce mécanisme dans la décision ; aucun défaut du moteur n’est établi.

### DOC-02 — Harmoniser les explications sur les documents à une ancienne connaissance K

**P2 · Contradiction observée dans les guides.** [Devises/historique](../utilisateur/devises-et-historique.md#L32), [immobilier](../utilisateur/immobilier-et-loyers.md#L82), [financements](../utilisateur/comptes-et-financements.md#L185), [guide documentaire](../utilisateur/documents-et-portabilite.md#L14).

Les trois premiers textes annoncent que les documents concernent toujours le coffre actuel ; le guide documentaire explique que leur consultation suit K et que les écritures sont bloquées à K fixe. Ces pages restent accessibles depuis le parcours utilisateur courant.

**Idée :** distinguer dans chaque guide la bibliothèque consultée à K des paramètres et exports intégraux du coffre actuel. Ajouter un renvoi vers le guide documentaire. Critère : les mêmes notions D/K donnent la même réponse dans tous les guides ; le comportement réel sera examiné en partie 7.

### DOC-03 — Rendre les renvois documentaires utilisables depuis GitHub

**P2 · Limitation de navigation observée.** [Guide Linux, lien du kit](../utilisateur/installation-linux.md#L126), [rapport courant](../historique/validation-phase-10.md#L7), [explication du README](../../README.md#L15), [téléchargements versionnés](../../distribution/README.md#L45).

Les 77 renvois vers `.local/` ou `/workspace/.tooling/` fonctionnent dans le cloud mais pas dans un clone ordinaire. Le kit possède déjà une destination versionnée sous `distribution/0.1.1/`. Le ZIP documentaire fournit aussi des copies adaptées des preuves : ce constat ne signifie pas que les preuves finales sont perdues. **Actualisation du 16 novembre :** le reclassement distant intégré pendant la revue a corrigé le lien du kit dans le guide courant. Le contrôle final de 56 fichiers Markdown trouve 424 liens locaux, aucun fichier absent dans cet environnement, mais toujours 77 destinations non suivies par Git. La limitation des preuves hors clone reste donc active ; le lien du kit n’est plus à corriger.

**Idée :** conserver le lien versionné du kit désormais corrigé ; créer un index stable des preuves et renvoyer les pages GitHub vers cet index ou le ZIP correspondant. Distinguer les pièces publiées, les anciennes pièces conservées seulement dans l’environnement et les commandes permettant de reproduire les contrôles. Vérifier les liens à partir des seuls fichiers versionnés, en complément du contrôle local.

### DOC-04 — Relier les exigences au périmètre effectivement reçu

**P2 · Proposition de suivi.** [Besoin de variation expliquée](cahier-des-charges.md#L890), [critères initiaux](plan-validation-initial.md#L130), [limite du pont livré](../technique/decisions-architecture.md#L130), [avancement](avancement.md#L142).

Les limites du pont de variation sont déclarées : il conserve une portée de registre et ne décompose pas tout le bilan intermodules en flux, valorisation, change et périmètre. Le plan initial seul ne permet toutefois pas de suivre ce reliquat parmi les phases marquées implémentées.

**Idée :** maintenir une matrice courte « exigence → livré / partiel / reste à faire → décision → preuve ». Y intégrer le pont intermodules et les autres conventions de livraison, sans réécrire le cahier des charges d’origine ni présenter les limites déclarées comme des défauts cachés.

### DOC-05 — Ajouter une correspondance entre conception et schéma livré

**P2 · Proposition de maintenance.** [Catalogue conceptuel](../technique/modele-relationnel.md#L5), [plan initial de migrations](../technique/modele-relationnel.md#L248), [décisions du schéma](../technique/decisions-architecture.md#L48), [portabilité au schéma 6](../technique/decisions-architecture.md#L120).

Le catalogue historique emploie notamment des sous-types et une migration documentaire `0007` qui ne correspondent pas directement au schéma livré. Son caractère prévisionnel est indiqué, mais retrouver la correspondance impose de reconstituer le journal des décisions.

**Idée :** ajouter un tableau « concept initial → table/commande livrée → migration → décision ». Identifier localement les dispositions remplacées : seuils d’ancienneté proposés comme paramétrables, mode strict et filiation automatique des lots, par exemple. Préserver les migrations livrées à l’identique.

### DOC-06 — Unifier la borne de pureté des métaux

**P2 · Contradiction documentaire observée.** [Modèle proposé](../technique/modele-relationnel.md#L147), [règles proposées](../technique/regles-metier.md#L270).

Le premier texte impose une pureté dans `]0,1]`, le second dans `[0,1]`.

**Idée :** choisir une borne unique et la distinguer de la quantité restante, qui peut être nulle après une vente complète. Contrôle de partie 6 : `metals.rs:401–404` applique `]0,1]` ; un probe refuse 0 et accepte 1. Harmoniser les textes avec cette règle livrée, sans correction du moteur sur ce point.

### DOC-07 — Distinguer les programmes couverts par chaque campagne

**P2 · Proposition de traçabilité.** [Parcours du paquet](../historique/validation-phase-10.md#L40), [accessibilité](../historique/validation-phase-10.md#L59), [manifeste de construction](../../distribution/0.1.1/build-manifest.json).

Le parcours natif et la mise à jour sont bien rattachés à l’AppImage finale par SHA-256. Les contrôles d’accessibilité utilisent un ELF de développement et les performances plusieurs binaires de service ; leurs identités sont précisées dans les preuves, mais dispersées. Le manifeste de construction contient les empreintes des dépendances verrouillées, sans commit ni arbre source.

**Idée :** ajouter une matrice « campagne → date → type/empreinte du programme → sources connues → portée ». Pour les constructions futures, enregistrer commit, état modifié et empreinte des sources. Pour les anciennes, signaler les métadonnées non enregistrées sans les inventer. Actualisation : le manifeste Pop!_OS ajoute `source_commit` et les preuves d’identité séparées ; la recette conserve toutefois `application_sources_unchanged=True` en littéral. Dériver cette attestation des empreintes/diffs à l’avenir. Le digest Ubuntu est épinglé, les paquets APT ne proviennent pas d’un snapshot figé ; l’identité binaire de deux reconstructions n’est déjà pas revendiquée. Le ZIP des sources correspond à `2c2fe966` ; les commits suivants jusqu’à la base de revue ajoutent README et distribution, sans divergence applicative démontrée.

### DOC-08 — Inventorier les anciennes preuves et les critères de réception

**P3 · Proposition de lisibilité des archives.** [Campagne phase 2](../historique/validation-phase-2.md#L99), [phase 3](../historique/validation-phase-3.md#L95), [critères de phase 2](../historique/validation-phase-2.md#L9).

Les anciens dossiers desktop cités en phases 2 et 3 ne figurent pas dans le ZIP distribué ; les preuves finales et celles des phases 4 à 10 sont présentes. Le lien entre identifiants de critères et observations reste souvent en prose.

**Idée :** inventorier les campagnes selon « livrée / conservée seulement dans l’environnement / non retrouvée » et relier chaque critère aux preuves existantes et limites. Séparer contrôle statique, refus adversarial réellement exécuté et panne physique non exercée. Ne pas recréer un résultat ancien à partir d’un test actuel.

**Partie 1 terminée — documentation, cahier des charges et explications.** Huit pistes consignées ; aucune correction appliquée. Prochaine partie : architecture applicative, contrats et primitives de calcul.

## Partie 2 — Architecture applicative, contrats et primitives de calcul

Périmètre : les dix fichiers de contrats de `crates/application/src/`, les primitives `exact.rs`, `operations.rs`, `profiles.rs`, les contrats communs du domaine, les types de transport TypeScript et leur appelant `src/services/api.ts`. Les validations communes sont lues ; la validation temporelle détaillée du registre appartient à la partie 3.

Contrôles ciblés effectués : **8 tests existants réussis**, sans échec ni test ignoré, via `cargo test -p patrimoine-domain --lib --offline exact::tests`, `cargo test -p patrimoine-domain --test fx_tests --offline precision` et `cargo test -p patrimoine-domain --test attribution --offline french_form_normalization`.

À conserver : sources bornées avant conversion en grands entiers, normalisation distincte du stockage canonique, calcul rationnel et arrondi final au pair, précision TND de trois décimales et refus d’une précision implicite pour une devise inconnue. Aucun défaut numérique ni incompatibilité fonctionnelle entre les DTO comparés n’a été établi. Le fait que la couche application expose surtout des ports ne justifie pas à lui seul une refonte.

### ARCH-01 — Contrôler la correspondance des contrats Rust et TypeScript

**P2 · Proposition de prévention des régressions.** [Documents Rust](../../crates/domain/src/documents.rs#L6), [documents TS](../../src/services/documents-types.ts#L1), [portabilité Rust](../../crates/domain/src/portability.rs#L6), [portabilité TS](../../src/services/portability-types.ts#L1), [appels typés](../../src/services/api.ts#L79).

Les champs comparés concordent actuellement, mais les structures sont recopiées et les types de retour d’`invoke<T>` ne valident pas eux-mêmes les réponses. Un changement Rust peut donc laisser le frontend compiler avec un contrat périmé.

**Idée :** générer les types de transport depuis Rust ou contrôler les schémas sérialisés en CI. Vérifier champs, propriétés obligatoires, nullabilité et valeurs finies, sans déplacer les calculs financiers dans TypeScript. Distinguer également les champs facultatifs d’une ancienne forme acceptée des propriétés normalement émises par le DTO courant : `serde(default)` ne signifie pas une omission en sortie.

### ARCH-02 — Exploiter des codes d’erreur et statuts stables

**P2 · Proposition de maintenance et d’explication.** [AppError](../../crates/domain/src/lib.rs#L44), [présentation d’erreur](../../src/services/api.ts#L68), [statuts documentaires](../../src/services/documents-types.ts#L3).

Les codes d’erreur et plusieurs valeurs finies sont des chaînes libres. Le présentateur commun retient surtout le message, ce qui ne structure pas les actions propres à un conflit de révision, une validation ou une erreur d’accès au support. Cela ne signifie pas une absence de validation backend des valeurs actuelles.

**Idée :** définir des enums sérialisées conservant les valeurs existantes et partager leur contrat avec TypeScript. Associer aux codes des explications et actions adaptées, tout en conservant les messages français. Vérifier les branches inconnues et la compatibilité des anciens codes plutôt que l’orthographe d’un message complet.

### ARCH-03 — Indexer une fois les capacités des lignes de trésorerie

**P2 · Complexité observée, optimisation proposée.** [Agrégation](../../crates/domain/src/operations.rs#L13), [recherches répétées](../../crates/domain/src/operations.rs#L49).

Après agrégation par `(transaction_revision_id, line_id)`, chaque clé provoque une recherche linéaire dans les transactions puis leurs lignes. Avec U clés utilisées et T transactions, le coût peut atteindre `O(U × T)`, hors recherche des lignes. Aucune latence propre à cette fonction n’a été mesurée ici.

**Idée :** construire un index des capacités par clé avant de vérifier les sommes. Garder les mêmes références exactes, résultats, refus et ordre déterministe. Comparer les cas de capacité partagée existants et mesurer un jeu comprenant plusieurs milliers de lignes ; ne pas retirer les validations pour obtenir un gain.

### ARCH-04 — Compléter quelques frontières des normalisations

**P3 · Proposition de couverture, aucun comportement incorrect observé.** [Normalisation des montants](../../crates/domain/src/exact.rs#L32), [zéro négatif](../../crates/domain/src/exact.rs#L50), [normalisation des taux](../../crates/domain/src/exact.rs#L126).

Les tests couvrent déjà de nombreuses bornes et valeurs non canoniques.

**Idée :** compléter ultérieurement les cas `-0` et `-0,000 → 0`, l’idempotence de la normalisation et les longueurs exactes de 64/65 octets. Ces cas visent les frontières du contrat ; ils ne nécessitent pas une nouvelle représentation monétaire.

**Partie 2 terminée — architecture applicative, contrats et primitives de calcul.** Quatre pistes supplémentaires consignées ; aucune correction appliquée. Prochaine partie : registre, propriété, consolidation, devises et historique.

## Partie 3 — Registre, propriété, consolidation, devises et historique

Périmètre : contrats et validations du registre, poids et contributions de consolidation, profils datés, FX et pont de variation ; persistance `infrastructure/src/registry.rs`, migrations 0002/0003 et suites associées ; interfaces registre, change, historique et périodes de profil, types et tests correspondants. Les calculs intermodules de dettes et métaux sont réservés aux parties 5 et 6. Le raccordement `onChanged` dans App est lu ponctuellement pour vérifier le résultat d’une mutation.

Contrôle exécuté : `cargo test -p patrimoine-domain --offline --test fx_tests --test variation_tests` — **18 tests réussis**, aucun échec ni ignoré. Les autres suites citées sont consultées, pas annoncées comme relancées. Aucun défaut de montant ni de persistance n’a été confirmé dans ce périmètre. Transactions immédiates, révisions attendues, cutoff K, refus de cycles et conservation des annulations sont à préserver.

### REG-01 — Compléter les sources expliquant les contributions

**P2 · Provenance incomplète observée, pas un montant erroné démontré.** [Poids initiaux](../../crates/domain/src/consolidation.rs#L273), [sources de la contribution](../../crates/domain/src/consolidation.rs#L450).

Le choix du périmètre familial utilise les appartenances datées, mais les sources des poids initiaux ne reçoivent que l’identité/révision des personnes. Les politiques des sociétés déterminent aussi la traversée détaillée ou son arrêt sans être toutes ajoutées aux sources. Un lecteur ne retrouve donc pas directement toutes les décisions à l’origine du montant.

**Idée :** ajouter les identités et révisions des appartenances actives et des politiques effectivement utilisées. Vérifier les sources dans les cas direct, indirect et mixte global/détaillé, en conservant le même résultat numérique.

### REG-02 — Préparer les index et chemins de change pour un snapshot donné

**P2 · Complexité observée, optimisation proposée.** [Recherche des poids](../../crates/domain/src/consolidation.rs#L319), [parcours des actifs](../../crates/domain/src/consolidation.rs#L817), [projection des profils](../../crates/domain/src/profiles.rs#L6), [dernier cours](../../crates/domain/src/fx.rs#L85), [recalculs de variation](../../crates/domain/src/variation.rs#L133).

Les participations sont rescannées par société, les propriétés par actif, les estimations par position et les cours par conversion. Plusieurs validations/recalculs sont répétés sur le même snapshot. Ces coûts sont visibles dans le code ; aucune durée propre à chaque boucle n’est établie ici.

**Idée :** indexer les sources par cible et date, préparer les chemins FX par couple/date et réutiliser les structures dans un même snapshot immuable. Toute réutilisation doit rester attachée à D/K et être invalidée après mutation ou restauration. Préserver les priorités direct/inverse/pivots, les inconnus, les sources et les alertes. Mesurer séparément graphe large, nombreuses estimations et dates historiques, sans remplacer l’arithmétique exacte.

### REG-03 — Séparer écriture confirmée et actualisation de l’affichage

**P2 · Chemin d’erreur observé, scénario conditionné à un échec de lecture après écriture.** [Retour après commit](../../crates/infrastructure/src/registry.rs#L589), [mutation UI](../../src/features/registry.tsx#L109), [change](../../src/features/exchange-rates.tsx#L49), [rechargement qui peut rejeter](../../src/App.tsx#L81).

Les sauvegardes rechargent le registre complet après le commit pour retrouver l’objet. L’interface ferme aussi son formulaire après `save*`, puis attend `onChanged` dans le même bloc d’erreur. Un rechargement peut donc échouer alors que la mutation est déjà persistée. Dans App, l’erreur du registre peut masquer le composant : il ne faut pas présumer qu’un message local précis sera toujours visible, mais la confirmation de réussite peut être perdue.

**Idée :** retourner l’objet produit ou relu par identifiant avant commit, distinguer l’état « enregistré » d’un échec de synchronisation et proposer de recharger sans rejouer l’écriture. Couvrir le scénario `save*` réussi puis actualisation rejetée ; ne pas annoncer un rollback qui n’a pas eu lieu.

### REG-04 — Mesurer aussi une longue succession de connaissances

**P2 · Proposition de mesure et d’optimisation.** [Validation de chaque séquence](../../crates/infrastructure/src/registry.rs#L721), [chargement de l’historique de profils](../../crates/infrastructure/src/registry.rs#L69).

La validation historique recharge et contrôle un snapshot à chaque changement, y compris ceux sans modification du registre. Le coût cumule donc des états croissants. Le gros profil de qualification n’a que deux changements : il ne renseigne pas ce cas d’historique dense.

**Idée :** mesurer plusieurs milliers de changements successifs avant de proposer une réutilisation des états ou le ciblage des séquences pertinentes. Conserver la vérification de tous les anciens états qui peut révéler une incohérence historique ; ne pas se limiter au dernier K pour accélérer.

### REG-05 — Rendre les confirmations accessibles au clavier

**P2 · Défaut de parcours clavier observé statiquement.** [Confirmation rendue avant les tableaux](../../src/features/registry.tsx#L164), [défilement seul](../../src/features/registry.tsx#L87), [gestion existante dans EditorFrame](../../src/features/registry-ui.tsx#L43).

L’ouverture d’une confirmation défile vers celle-ci mais laisse le focus sur son bouton déclencheur, situé plus loin dans les tableaux. Tab poursuit alors les actions des tableaux au lieu d’atteindre la nouvelle confirmation.

**Idée :** focaliser le titre à l’ouverture et restaurer le focus au déclencheur à la fermeture, comme pour les formulaires ordinaires. Vérifier ce parcours pour archivage, réactivation et annulation, en plus des contrôles déjà présents pour EditorFrame.

### REG-06 — Identifier une conversion dont la saisie a changé

**P3 · Amélioration de présentation.** [Saisie et résultat FX](../../src/features/exchange-rates.tsx#L93), [invalidation actuelle](../../src/features/exchange-rates.tsx#L42).

Modifier le montant ou les devises conserve la conversion précédente. Celle-ci affiche encore son ancien montant : aucun calcul faux n’est établi, mais la proximité des nouveaux champs peut prêter à confusion.

**Idée :** masquer la conversion dès que sa requête change ou l’étiqueter comme résultat précédent. Montrer également le K effectif du pont historique. Une actualisation du journal ne doit pas être considérée comme recalculant implicitement ce pont ; aucun scénario réel de divergence de K n’est démontré ici.

### REG-07 — Remplacer les annonces de modules futurs déjà livrés

**P3 · Contenu affiché périmé.** [Notice sociétés](../../src/features/registry.tsx#L176), [contrôle d’attribution](../../src/features/registry.tsx#L221), [historique](../../src/features/history.tsx#L61).

Ces textes annoncent encore comme futurs les indicateurs, financements, modules immobilier/banque/métaux ou tableaux de bord pourtant présents.

**Idée :** renvoyer vers les sections existantes et expliquer le périmètre précis du contrôle affiché. Préserver la distinction entre sous-total historique d’actifs et bilan financier complet, sans annoncer que leurs résultats seraient interchangeables.

### REG-08 — Ajouter une frontière persistante explicite à K=0

**P3 · Proposition ciblée de couverture.** [Lecture bornée](../../crates/infrastructure/src/registry.rs#L304), suites `registry_adversarial.rs`, `registry_migrations.rs` et `fx_history.rs`.

Les trois suites consultées exercent des K passés non nuls et futurs ; une lecture persistante explicite `Some(0)` n’y apparaît pas. Cela n’affirme pas une absence de couverture K=0 dans toutes les autres couches.

**Idée :** vérifier le snapshot métier vide, l’absence de fuite de révisions et l’absence de mutation à K=0, notamment après migration ou restauration. Conserver les tests existants de corrections/annulations anciennes et de rollback des compteurs.

**Partie 3 terminée — registre, propriété, consolidation, devises et historique.** Huit pistes supplémentaires consignées ; aucune correction appliquée. Prochaine partie : immobilier et loyers. Point à rapprocher des cessions : le registre seul peut signaler des droits incomplets après clôture de tous les droits d’un actif ; il ne dispose pas des événements immobiliers permettant d’inférer sa vente. Aucun défaut du bilan n’est conclu sur ce seul constat.

## Partie 4 — Immobilier et loyers

Périmètre : `domain/src/property.rs` et `rentals.rs`, `infrastructure/src/properties.rs` et `rentals.rs`, migration 0004, écrans `properties.tsx` et `real-estate.tsx`, contrats et suites domaine/persistance/interface associés.

Contrôles exécutés : **26 tests existants réussis**, sans échec ni ignoré, avec `cargo test -p patrimoine-domain --test rental_tests --test income_attribution_tests --offline`. Un probe temporaire hors dépôt, compilé contre le domaine local puis exécuté, confirme IMMO-01. Les autres tests sont consultés. Les coûts d’acquisition, conversions historiques, estimations et gains sont correctement séparés dans les cas lus. La vraie cession comporte un événement explicite et une clôture atomique des droits ; elle n’invente pas de trésorerie et conserve les anciens K. Les garanties d’idempotence de génération et de correction des remboursements sont à préserver.

### IMMO-01 — Ne pas rendre inconnus les revenus à cause d’un bail sans exposition économique

**P2 · Défaut confirmé par un cas ciblé.** [Contrôle des données locatives manquantes](../../crates/domain/src/rentals.rs#L1699), [invalidation des indicateurs](../../crates/domain/src/rentals.rs#L1737).

Cas reproduit : actif entièrement détenu par un tiers identifié, bail actif sans échéances générées, requête familiale sans filtre de bien. Le poids économique familial est connu et vaut `0/1`, le loyer contractuel annualisé vaut `0.00`, mais le revenu économique et l’impayé deviennent `None`, avec statut partiel et alerte `missing_rent_dues`. Un bail sans exposition dans ce contexte empêche donc de calculer ses totaux.

**Idée :** limiter l’invalidation de chaque indicateur aux segments où son exposition est positive ou inconnue ; conserver zéro quand elle est démontrée nulle. Ne pas filtrer globalement les encaissements : leur bénéficiaire explicite reste indépendant des droits économiques. Couvrir le tiers sans échéances et un cas mixte comportant aussi un revenu familial connu.

### IMMO-02 — Conserver la confirmation après rechargement du parent

**P2 · Fragilité de cycle de vie observée dans l’intégration.** [Succès après onChanged](../../src/features/properties.tsx#L68), [reload qui efface les snapshots](../../src/features/real-estate.tsx#L80), [montage conditionnel de l’enfant](../../src/features/real-estate.tsx#L182).

Le parent déclenche un chargement qui peut démonter PropertyWorkspace, alors que celui-ci affiche son succès seulement après l’actualisation. Avec un vrai chargement asynchrone, ce résultat peut viser l’ancienne instance et disparaître. Les tests isolés de l’enfant utilisent un callback qui ne le démonte pas.

**Idée :** conserver le résultat de mutation dans un parent durable ou préserver l’instance pendant le rechargement du même contexte. Vérifier depuis RealEstatePage qu’une sauvegarde puis une correction de cession donnent une confirmation durable sans seconde mutation. Ne pas conserver des données affichées sous un autre K ; rapprocher ce travail de REG-03.

### IMMO-03 — Bloquer les onglets pendant une opération enfant

**P2 · Condition de navigation observée.** [Activité enfant](../../src/features/real-estate.tsx#L55), [onglets](../../src/features/real-estate.tsx#L177).

Le parent reçoit `childBusy`, mais ses onglets ne sont désactivés que sur `pending || loading`. Un changement d’onglet pendant une sauvegarde ou cession en attente peut démonter l’enfant et masquer le résultat local de cette opération.

**Idée :** inclure l’activité enfant dans le verrou des onglets et conserver le résultat indépendamment de son composant. Avec une promesse de cession en attente, vérifier que les onglets restent inactifs puis se réactivent après résolution ou refus.

### IMMO-04 — Distinguer les biens, unités et baux homonymes

**P2 · Ambiguïté observée dans les options de sélection.** [Libellé du bail](../../src/features/real-estate.tsx#L167), [sélecteurs](../../src/features/real-estate.tsx#L213), [unité](../../src/features/real-estate.tsx#L221).

Deux immeubles peuvent avoir une unité « A · Studio » ; deux baux successifs peuvent aussi concerner le même locataire. Les options n’affichent pas systématiquement le bien et la période, donc les choix peuvent être identiques avant sélection, notamment pour un paiement ou la génération d’échéances.

**Idée :** afficher bien, référence d’unité et dates du bail, avec un identifiant court lorsque nécessaire. Vérifier deux biens aux unités homonymes et deux contrats successifs : chaque choix doit être identifiable et la commande viser l’objet choisi.

### IMMO-05 — Réduire les recherches répétées sans retirer les contrôles

**P3 · Optimisations proposées, sans benchmark propre à ces fonctions.** [Identités immobilières](../../crates/domain/src/property.rs#L179), [génération d’échéances](../../crates/infrastructure/src/rentals.rs#L627), [sélection des créations](../../crates/infrastructure/src/rentals.rs#L683), [cible immuable](../../crates/infrastructure/src/properties.rs#L464).

Le contrôle des identités immobilières parcourt l’ensemble des couples d’identités du registre pour chaque objet immobilier. La génération recherche chaque clé dans toutes les échéances puis utilise des recherches linéaires d’identifiants. Le contrôle de cible immobilière compare aussi les révisions deux à deux.

**Idée :** préparer deux ensembles d’identités/révisions et un index de clés de génération ; comparer après mesure une agrégation par objet et nombre de cibles distinctes à l’autojointure actuelle. Le contrôle locatif fournit déjà un exemple d’agrégation. Préserver les conflits, annulations, clés réservées et cibles immuables, avec les mêmes résultats historiques.

### IMMO-06 — Verrouiller explicitement l’immutabilité de devise d’un bail

**P3 · Proposition de couverture ; piste de bug écartée.** [Validation des devises sur toutes les révisions](../../crates/infrastructure/src/rentals.rs#L92), [validation avant commit](../../crates/infrastructure/src/rentals.rs#L245), [règle documentée](../utilisateur/immobilier-et-loyers.md#L38).

Un snapshot artificiel du domaine peut représenter des conditions successives de devises différentes, mais le store examine toutes les révisions et refuse cette correction avant commit. L’application n’est donc pas démontrée capable de changer un bail EUR en USD sur ce chemin.

**Idée :** ajouter un cas de correction refusée avec échéance EUR existante, puis vérifier que snapshot, historique et K sont inchangés. Documenter le niveau auquel cet invariant historique est garanti plutôt que conclure à tort qu’un cas artificiel est un défaut de commande.

**Partie 4 terminée — immobilier et loyers.** Six pistes supplémentaires consignées, dont un défaut de complétude reproduit ; aucune correction appliquée. Prochaine partie : trésorerie, financements et bilan financier.

## Partie 5 — Trésorerie, financements et bilan financier

Périmètre : domaines `treasury.rs`, `debts.rs`, `finance.rs`, `financial_adapters.rs`, leurs stores, migration 0005, contrats et interfaces trésorerie/postings/dettes/bilan, suites associées. Le contrat historique `LoanPosition` est aussi vérifié : aucun appel de production avec prêts non vides n’est trouvé ; ne pas le confondre avec le bilan actuel.

Contrôles exécutés : **92 tests existants distincts réussis**, sans échec ni ignoré : 39 dans `treasury_tests`/`debt_tests`, 26 dans `financial_tests`/`financial_adapter_tests`, 24 dans `treasury_store`/`debts_store` et 3 dans `finance::history_root_tests`. Commandes `cargo test -p patrimoine-domain --offline --test treasury_tests --test debt_tests`, `cargo test -p patrimoine-domain --offline --test financial_tests --test financial_adapter_tests`, `cargo test -p patrimoine-infrastructure --offline --test treasury_store --test debts_store` et `cargo test -p patrimoine-infrastructure --lib --offline finance::history_root_tests`. Les suites UI sont lues, pas relancées. Un probe temporaire hors dépôt appelle les services persistants pour FIN-01.

À préserver : ouvertures inconnues distinctes de zéro, principal/intérêts/frais séparés, contrôles des capacités et dates historiques, écritures coordonnées atomiques, refus de révisions obsolètes et déclarations de couverture distinctes des sources manquantes. DOC-01 est résolu ci-dessus comme ambiguïté documentaire. La cession immobilière explicite conserve un traitement distinct des droits simplement clos dans le registre ; aucun défaut du bilan n’est conclu à partir du seul avertissement du registre.

### FIN-01 — Vérifier les parties d’un financement représentant une créance locative

**P1 · Défaut confirmé via les services persistants.** [Validation du lien](../../crates/domain/src/finance.rs#L468), [substitution dans le bilan](../../crates/domain/src/finance.rs#L1271), [sauvegarde persistante](../../crates/infrastructure/src/finance.rs#L312).

Le lien `represented_by_debt_id` vérifie l’existence et la devise du contrat ; le bilan rapproche son principal du reliquat à la date de transfert. Il ne vérifie pas que le débiteur est le locataire ni que les créanciers et leurs droits correspondent à la créance remplacée.

Cas reproduit, personne A, D=2025-03-31 : immeuble 200 000 EUR, créance locative de A sur le locataire 1 000 EUR, banque zéro, et **prêt déjà présent en sens inverse**, dû par A au locataire, de 1 000 EUR. À K=19 : actifs 201 000, passifs 1 000, net 200 000, statut complet. La seule correction ajoutant ce prêt comme représentation de la créance est acceptée à K=20 : actifs 200 000, passifs 1 000, net 199 000, toujours complet, sans `rent_loan_representation_mismatch`. Une dette sans rapport peut ainsi masquer une créance réelle.

**Idée :** contrôler identité du débiteur, créanciers, fractions et continuité temporelle à la date de transfert, puis la cohérence du suivi ultérieur. Refuser un lien incohérent avant commit et conserver ancien K/compteurs ; si une ancienne donnée incohérente est lue, rendre son impact explicite sans présenter le net comme complet. Ajouter les cas parties inversées, autre locataire et créanciers partagés. Le comportement des encaissements postérieurs reste un scénario de régression à définir, pas un deuxième défaut prouvé ici.

### FIN-02 — Afficher l’annulation des prévisions d’échéance

**P2 · Défaut de rendu établi par lecture du chemin de code.** [Rendu](../../src/features/debts.tsx#L190), [montants et statut du domaine](../../crates/domain/src/debts.rs#L966), [retard réservé aux prévisions actives](../../crates/domain/src/debts.rs#L993).

L’écran ignore `schedule.status`. Une échéance `cancelled`, non nulle et sans affectation conserve ses montants historiques mais s’affiche « Non soldée » avec un principal « restant prévu ». Le formulaire permet bien de l’annuler. Aucun encours contractuel faux n’est établi ; l’erreur concerne l’interprétation de la prévision.

**Idée :** afficher « Prévision annulée », distinguer ses montants historiques de sommes encore à régler et couvrir le rendu des deux statuts. Les fixtures de l’écran consultées ont des échéanciers vides.

### FIN-03 — Harmoniser la promesse d’archivage et les nouvelles saisies

**P3 · Incohérence de parcours observée.** [Message d’archivage](../../src/features/debts.tsx#L148), [options des formulaires](../../src/features/debts.tsx#L161), [posting](../../src/features/treasury-postings.tsx#L104).

Le message promet de masquer le contrat archivé des nouvelles saisies, mais les listes proposent tous les contrats sans filtrer `archived`.

**Idée :** choisir une convention cohérente pour les nouvelles opérations, tout en gardant les contrats archivés accessibles pour consulter et corriger leur historique. Étendre également IMMO-04 aux baux successifs proposés par les postings, sans créer un constat distinct.

### FIN-04 — Ordonner chronologiquement les relevés et changes exécutés

**P3 · Amélioration de présentation.** [Collections du rapport](../../crates/domain/src/treasury.rs#L974), [changes](../../crates/domain/src/treasury.rs#L1022), [lecture par identifiant](../../crates/infrastructure/src/registry.rs#L86), [rendu](../../src/features/treasury.tsx#L150).

Les collections gardent l’ordre du snapshot, chargé par identifiant, puis sont affichées directement. Des dates peuvent donc être dispersées selon les UUID.

**Idée :** trier par date puis identifiant de façon déterministe, comme l’échéancier de dette. Aucun montant incorrect n’est établi.

Compléments aux pistes existantes : REG-03 touche aussi les sauvegardes ordinaires des trois stores et les écrans financiers ; les postings coordonnés relisent au contraire leur objet avant commit. IMMO-02 concerne aussi l’enfant de TreasuryPage. REG-02/04 sont renforcés par `infrastructure/src/finance.rs:126–150,216–240` : chargements et validations emboîtés répètent les mêmes invariants et le rejeu des K. Proposer une orchestration unique par snapshot, sans supprimer de contrôle historique ; aucune latence propre n’a été mesurée. Le refus de corriger isolément une transaction partagée entre sources protège l’intégrité : un éventuel parcours collectif serait une amélioration, pas un contournement à introduire.

**Partie 5 terminée — trésorerie, financements et bilan financier.** Quatre pistes supplémentaires, dont un défaut de substitution financière reproduit ; aucune correction appliquée. Prochaine partie : métaux précieux et sociétés.

## Partie 6 — Métaux précieux et sociétés

Périmètre : domaines, stores, ports et interfaces métaux/sociétés, migration 0006, helpers `phase7.rs`, intégrations `finance.rs`/`consolidation.rs`, contrats, suites et guide associés.

Contrôles exécutés : **69 tests existants distincts réussis**, sans échec ni ignoré : `metals_tests` 21, `companies_tests` 18, `operations_integration` 14, `metals_store` 9 et `companies_store` 7, avec `cargo test -p patrimoine-domain --offline --test metals_tests --test companies_tests --test operations_integration` et `cargo test -p patrimoine-infrastructure --offline --test metals_store --test companies_store` (suites lancées en groupes distincts ; les relances d’intégration ne sont comptées qu’une fois). Un test UI temporaire hors dépôt reproduit MET-01. Des probes hors dépôt exercent les calculs, puis les services persistants pour MET-02/03 et deux frontières de distribution.

À préserver : conversions physiques exactes, coût homogène proportionnel, stock nul distinct d’une ouverture inconnue, double face bancaire et capacité commune entre modules, transfert des droits atomique et conservation des bornes futures. Les indicateurs de sociétés ne créent pas de positions patrimoniales, ni une déclaration de dividende un encaissement. DOC-06 est tranché ci-dessus.

### MET-01 — Fixer le lot cible pendant un transfert des droits

**P1 · Mauvaise cible de commande reproduite dans l’interface.** [Ouverture](../../src/features/metals.tsx#L140), [sauvegarde](../../src/features/metals.tsx#L148), [sélecteur actif](../../src/features/metals.tsx#L182).

Le brouillon conserve les propriétaires du lot A mais pas son identifiant. Le filtre reste utilisable pendant la saisie. Si l’utilisateur choisit ensuite le lot B, la sauvegarde relit B et lui applique les propriétaires préparés pour A.

Probe UI : ouvrir le transfert de A, saisir Charles comme nouveau propriétaire, sélectionner B, enregistrer. Le payload vise `lot-b`, clôture les droits de B et ouvre pour B ceux de Charles à `1/1`. Ce n’est pas une commande rejetée pour mélange d’actifs : les lignes construites concernent toutes B. Aucun coffre réel d’utilisateur n’a été modifié pendant le probe.

**Idée :** capturer lot et actif dans le brouillon, afficher leur identité dans le formulaire et invalider/fermer ce brouillon au changement de contexte. Vérifier que le backend reçoit toujours la cible annoncée à l’ouverture et que les droits de B restent inchangés. Le test temporaire est exécuté avec `node node_modules/vitest/vitest.mjs run --config /tmp/patrimoine-p6-ui-audit/vite.config.mts` ; il ne constitue pas encore un test de régression versionné.

### MET-02 — Convertir les primes et décotes même sans cotation du métal

**P2 · Montants de composantes faux, reproduits dans les services persistants.** [Devise de repli](../../crates/domain/src/metals.rs#L937), [taux EUR de repli](../../crates/domain/src/metals.rs#L987), [composantes EUR](../../crates/domain/src/metals.rs#L1058), [affichage](../../src/features/metals.tsx#L228).

Sans intrinsèque calculable, le taux EUR est remplacé par 1. Stock de 2 unités, devise de coût USD, coût zéro, aucune cotation métal, prime 10 USD/unité et décote 5 USD/unité : le rapport expose prime 20 USD **et 20 EUR**, décote 10 USD **et 10 EUR**. Ajouter USD/EUR=0,5 ne corrige pas ces valeurs : elles devraient valoir 10 EUR et 5 EUR. Sans FX, ces faux EUR sont déjà présents sans avertissement de change. Les valeurs totales intrinsèque/revente restent inconnues : aucun net total erroné n’est établi par ce cas.

**Idée :** convertir indépendamment chaque composante connue, conserver ses sources FX et rendre ses euros inconnus si le taux nécessaire manque. Couvrir prime monétaire, décote, prime proportionnelle dépendant de l’intrinsèque et zéros explicites.

### MET-03 — Garder calculable un stock confirmé nul malgré une cotation étrangère

**P2 · Défaut de complétude reproduit dans les services persistants.** [Devise du stock nul](../../crates/domain/src/metals.rs#L933), [transmission au bilan](../../crates/domain/src/finance.rs#L2181), [exigence de change](../../crates/domain/src/finance.rs#L933).

Fixture persistante : banque 1 000 EUR, lot ouvert à quantité/coût zéro, inventaire complet, aucune cotation ni FX. Bilan complet, net 1 000 EUR. Ajouter uniquement un cours or 60 USD/g est accepté ; le rapport métal reste complet avec valeur 0 EUR, mais le bilan devient partiel, net inconnu, `missing_exchange_rate`, sous-total connu 1 000 EUR. L’ancien K conserve le résultat complet. La présence de la cotation fait transporter le zéro en USD jusqu’au bilan, qui exige alors inutilement un change.

**Idée :** conserver une valeur exactement nulle sans exiger un taux, avec provenance du zéro et sans convertir une quantité inconnue en zéro. Couvrir stock nul avec/sans cotation étrangère, taux absent et propriété inconnue.

### SOC-01 — Expliquer la règle de chevauchement indépendamment de la devise

**P3 · Contradiction documentaire observée.** [Guide](../utilisateur/metaux-et-societes.md#L78), [règle](../../crates/domain/src/companies.rs#L335), [test](../../crates/domain/tests/companies_tests.rs#L350).

Le guide limite l’interdiction aux périodes de même société, fréquence **et devise**. Le moteur interdit le chevauchement de même société/fréquence quelle que soit la devise, ce que le test exerce explicitement.

**Idée :** distinguer cette unicité de la compatibilité de devise nécessaire pour calculer une tendance. Aucune correction du calcul n’est motivée par cette divergence.

### SOC-02 — Conserver deux frontières de distribution dans les tests

**P2 · Proposition de prévention, résultats actuels corrects.** [Compensation opaque](../../crates/domain/src/finance.rs#L2299), [cas de sortie déjà testé](../../crates/domain/tests/operations_integration.rs#L590).

Probes du domaine : un dividende de 500 entre filiale détaillée et parent global, tous deux sous la même barrière opaque, conserve un bilan complet de 5 000 avec l’ancienne estimation ; leurs mouvements se compensent. Pour une société détaillée détenue à 50 %, banque société 1 000 et membre 100, un dividende de 500 au membre fait passer le net familial de 600 à 850, compte tenu de la part des associés extérieurs.

**Idée :** fixer ces résultats dans des tests ciblés pour éviter qu’une simplification du traitement opaque ou une neutralisation de tout dividende ne casse les périmètres partiels. Ne pas présenter ces cas corrects comme des défauts.

**Partie 6 terminée — métaux précieux et sociétés.** Cinq pistes supplémentaires, dont trois défauts reproduits ; aucune correction appliquée. Prochaine partie : documents, sauvegardes et portabilité.

## Partie 7 — Documents, sauvegardes et portabilité

Reprise le **16 novembre 2026, Europe/Paris**, à partir du checkpoint `7ddc03a` ; aucun nouveau commit distant à intégrer. Périmètre : domaines et ports documentaires/portabilité, stores et chemins documentaires de `lib.rs`/`db.rs`, archives, CSV, XLSX, activation/restauration, interfaces et guides correspondants, script de récupération indépendant, construction du kit et suites associées. Les primitives de fichiers sont lues pour ces parcours ; leur revue générale reste en partie 9.

Contrôles exécutés pendant cette partie, y compris avant l’interruption : **100 tests existants distincts réussis**, sans échec ni ignoré : `documents_store` 12, `csv_portability` 9, `csv::tests` 2, `portability::tests` 2, `recovery_adversarial` 29, `xlsx_export` 4, `xlsx::tests` 4, interface documents/portabilité 22, récupérateur Python 16. Commandes Rust ciblées `cargo test -p patrimoine-infrastructure --offline --test documents_store --test csv_portability --test recovery_adversarial --test xlsx_export` et filtres `--lib csv::tests`, `--lib portability::tests`, `--lib xlsx::tests` ; UI `npm test -- src/features/documents.test.tsx src/features/portability.test.tsx --exclude '.local/**'` ; Python `python3 scripts/test-recover-csv.py`. Les copies de tests de `.local/pop-os22/source` ne sont pas recomptées. Des probes temporaires hors dépôt vérifient métadonnées, copies, conflits, 2 801 chaînes CSV et injections logicielles d’erreurs d’E/S. Aucune panne physique n’est prétendue testée.

À préserver : originaux et anciens K conservés après remplacement/retrait, vérification des octets avant ouverture/export, schéma local approuvé et insertions paramétrées, refus d’archive incohérente, validation métier de la candidate avant activation, historique exact et provenance technique unique du réimport, anciennes générations conservées. XLSX produit du texte inerte et conserve les longs textes sans troncature silencieuse. Le récupérateur externe n’exécute pas de SQL reçu et annonce correctement qu’il ne remplace pas les validations financières Rust. DOC-02 est confirmé côté comportement : bibliothèque à K, opérations intégrales sur le coffre actuel.

### PORT-01 — Exposer l’état réellement activé après une erreur de finalisation

**P1 · Défaut de reprise reproduit par injection logicielle d’E/S.** [Bascule et nettoyage](../../crates/infrastructure/src/lib.rs#L217), [propagation](../../crates/infrastructure/src/lib.rs#L543), [interface](../../src/features/portability.tsx#L78), [rafraîchissement attendu](../../src/App.tsx#L110).

Une erreur EIO injectée lors du retrait de `recovery/activation.json`, après bascule de CURRENT, fait retourner une erreur alors que la nouvelle génération et ses données sont déjà actives. L’interface saute son rafraîchissement et conserve l’ancien résumé d’analyse, registre et K. Le journal résiduel fait échouer une nouvelle activation avec EEXIST jusqu’à réouverture. Une erreur ponctuelle de renommage de CURRENT conserve bien l’ancien coffre par rollback, mais laisse également ce journal bloquant. Dans les probes, la reprise à la réouverture réussit ; aucune perte ni corruption n’est démontrée.

**Idée :** distinguer échec avant bascule, activation effectuée et reprise encore nécessaire. Relire la génération effective, rafraîchir l’interface en conséquence et clôturer le journal ou guider explicitement la réouverture. Préserver les anciennes générations et éviter un rollback implicite après une activation déjà effectuée. Ce cas de remplacement complet dépasse le retour de mutation ordinaire de REG-03.

### PORT-02 — Guider la reprise d’une sauvegarde dont la publication n’est pas durablement confirmée

**P2 · État de sortie reproduit, erreur justifiée.** [Publication](../../crates/infrastructure/src/files.rs#L197), [sauvegarde](../../crates/infrastructure/src/lib.rs#L435), [refus de remplacement](../../crates/infrastructure/src/files.rs#L181).

Une EIO injectée sur la synchronisation du dossier parent laisse un ZIP final complet, lisible et validable, tout en retournant une erreur. Réessayer le même nom est refusé parce que ce fichier existe. L’échec de fsync empêche de garantir la durabilité : il ne faut donc pas annoncer un succès assuré au seul motif que le ZIP est visible.

**Idée :** préciser que la destination peut déjà exister, proposer de la vérifier puis choisir un nouveau nom si nécessaire, sans écraser le fichier ni supprimer automatiquement une sauvegarde potentiellement utilisable. Distinguer disponibilité immédiate et durabilité après panne.

### DOCU-01 — Unifier les métadonnées documentaires entre les chemins d’accès

**P2 · Incohérences reproduites.** [MIME d’import](../../crates/infrastructure/src/lib.rs#L334), [MIME de remplacement](../../crates/infrastructure/src/documents.rs#L255), [date du résumé](../../crates/infrastructure/src/db.rs#L159), [date initiale de la bibliothèque](../../crates/infrastructure/src/documents.rs#L51).

Importer un CSV, DOCX, XLSX, ODT, ODS ou RTF attribue un MIME générique ; remplacer ensuite le document par les mêmes octets lui attribue un MIME précis, avec SHA inchangé. L’ouverture reste fonctionnelle grâce à la liste d’extensions. Une simple correction de description change aussi `added_at` dans le résumé public `list_documents`, alors que la bibliothèque conserve la date de première révision. Aucun mauvais `added_at` affiché par l’interface courante n’est établi : elle utilise le chemin bibliothèque.

**Idée :** partager les règles MIME et définir séparément date d’ajout et date de révision. Vérifier la cohérence des contrats sur import, remplacement identique et correction de description.

### DOCU-02 — Garder les associations retirées identifiables

**P2 · Défaut de présentation et proposition de lisibilité.** [Catalogue des cibles](../../crates/infrastructure/src/documents.rs#L95), [tableau](../../src/features/documents.tsx#L122), [explication promise](../utilisateur/documents-et-portabilite.md#L44).

Une cible métier retirée conserve son association mais peut n’être désignée que par type et UUID ; le tableau omet aussi l’indication `retired`, présente dans les sélecteurs et annoncée par le guide.

**Idée :** afficher le retrait dans le tableau et conserver un libellé historique identifiable à K, avec l’identifiant comme référence complémentaire. Ne pas masquer les anciens liens ni réactiver implicitement la cible.

### DOCU-03 — Gérer la rétention des copies de consultation

**P2 · Proposition de gestion du stockage.** [Création des copies](../../crates/infrastructure/src/documents.rs#L604).

Chaque ouverture crée une copie privée distincte dans `recovery/document-views/<uuid>`. Le nettoyage ne concerne que l’échec de préparation ; deux ouvertures du même original laissent deux copies, encore présentes après fermeture/réouverture dans le probe. Aucun défaut d’intégrité des originaux n’est établi.

**Idée :** définir une rétention limitée des seules copies de consultation, expliquer leur emplacement et prévoir un nettoyage compatible avec les lecteurs encore ouverts. Ne pas purger les blobs ni versions historiques.

### DOCU-04 — Expliquer la reprise d’un formulaire après conflit

**P2 · Amélioration de parcours confirmée par un probe UI.** [Rechargement](../../src/features/documents.tsx#L37), [source du formulaire](../../src/features/documents.tsx#L82), [soumission](../../src/features/documents.tsx#L92).

Après conflit puis rechargement d’une révision r2, le formulaire conserve sa saisie et sa source r1. Une nouvelle soumission renvoie encore r1 et peut être refusée à nouveau. Le contrôle optimiste protège les données ; il ne doit pas être retiré.

**Idée :** indiquer explicitement de fermer puis rouvrir le formulaire depuis la nouvelle source, ou proposer une reprise contrôlée avec comparaison des changements. Conserver les champs pour permettre leur récupération, sans remplacer silencieusement la révision attendue.

### PORT-03 — Traiter les tables CSV progressivement

**P2 · Optimisation proposée, aucun échec mémoire mesuré.** [Lecture complète](../../crates/infrastructure/src/portability.rs#L455), [matérialisation des cellules](../../crates/infrastructure/src/csv.rs#L170), [copies de ligne](../../crates/infrastructure/src/portability.rs#L549).

Le lecteur charge une table en octets puis matérialise toutes ses cellules et copie des lignes. Les limites de 128 Mio/table, deux millions de cellules et 1 Mio/cellule bornent le volume logique ; elles ne mesurent pas les allocations cumulées du parseur.

**Idée :** mesurer puis envisager une lecture progressive conservant grammaire, limites, diagnostics de ligne et transaction de candidate. Préserver les refus et l’exactitude du texte ; ne pas annoncer de pic chiffré ni d’OOM non observé.

Compléments : REG-05 concerne également le focus de la confirmation de retrait d’association ; REG-03 les rafraîchissements documents/portabilité après écriture. La piste `valid_from=null` est écartée : le backend exige une date. La durabilité des ancêtres de nouveaux répertoires documentaires reste à examiner avec les primitives en partie 9 ; aucune panne physique n’est conclue ici.

**Partie 7 terminée — documents, sauvegardes et portabilité.** Sept pistes supplémentaires ; aucune correction appliquée. Prochaine partie : tableaux de bord, recherche et composants d’interface communs.

## Partie 8 — Tableaux de bord, recherche et composants d’interface communs

Périmètre : `domain/src/dashboard.rs`, port et store dashboard, export JSON du rapport, types, interfaces tableau de bord/graphiques/recherche, App et wrappers de navigation, composants communs, styles, point d’entrée HTML/React et guide. Les modules métiers déjà examinés sont relus seulement à leurs raccordements.

Contrôles exécutés : **66 tests existants distincts réussis**, sans échec ni ignoré : 14 unités `dashboard::tests`, 11 `dashboard_store`, 23 UI tableau de bord/graphiques/recherche, 18 App/phase4/composants communs. Commandes `cargo test -p patrimoine-domain --offline --lib dashboard::tests`, `cargo test -p patrimoine-infrastructure --offline --test dashboard_store`, suites frontend ciblées via `npm test -- … --exclude '.local/**'` ; les copies isolées de construction ne sont pas recomptées. Probes temporaires : pays historique et fenêtre société via services persistants, trois scénarios shell et trois recherche/focus via tests UI hors dépôt.

À préserver : éliminations appliquées avant filtrage, fractions exactes et inconnus, K unique pour tous les snapshots, séparation bilan/indicateurs, sections non filtrées exclues explicitement de l’export filtré. Les graphiques ne convertissent en Number que leur géométrie ; étiquettes et tables gardent les textes du moteur. Aucun nouveau défaut d’addition globale n’est établi.

### DASH-01 — Ne pas attribuer un pays courant à une position historique de pays inconnu

**P2 · Affectation historique erronée reproduite via Vault.** [Complément de pays](../../crates/domain/src/dashboard.rs#L398), [source volontairement inconnue](../../crates/domain/src/consolidation.rs#L444).

Actif de 100 EUR, D=2020-06-01, aucune période descriptive : le rapport financier garde `country=None` et explique que le pays historique n’est pas attribué par hypothèse. Au même D/K, le tableau de bord complète pourtant ce None depuis le pays descriptif courant de l’actif : FR. Le filtre FR retient une position de 100 EUR et le regroupement l’affecte à FR. Aucun montant global faux n’est démontré ; filtre et répartition géographique historiques sont erronés.

**Idée :** conserver le pays inconnu tant qu’aucune source datée ne permet son attribution. Aligner l’annotation du tableau de bord sur le moteur source et vérifier les cas actif, résidence d’entité et profil daté corrigé à K ultérieur.

### DASH-02 — Appliquer le plafond de points aux séries effectivement construites

**P2 · Refus inutile reproduit via Vault.** [Dates calculées avant branchement](../../crates/infrastructure/src/dashboard.rs#L184), [vue société](../../crates/infrastructure/src/dashboard.rs#L220).

La vue société ne construit aucun historique financier, mais le plafond de 61 points lui est appliqué avant ce branchement. Une société sans observation sur 2019-01-01→2025-01-01 est refusée avec le pas mensuel ; la même demande annuelle est acceptée avec `history=[]` et zéro observation. Aucun montant faux ni perte n’est établi.

**Idée :** vérifier la capacité seulement pour les séries produites, ou définir et expliquer une limite globale si elle est intentionnelle. Couvrir une longue fenêtre de société sans courbe patrimoniale, indépendamment du plafond familial.

### SEARCH-01 — Ouvrir les opérations d’espèces dans le module correspondant

**P2 · Mauvaise destination reproduite par deux probes UI.** [Routage générique](../../src/features/global-search.tsx#L18), [seuls comptes conservés](../../src/features/global-search.tsx#L46), [exception limitée au compte](../../src/features/global-search.tsx#L63), [filtrage bancaire](../../src/features/treasury.tsx#L67).

Une transaction n’ayant qu’une ligne de compte espèces, ainsi qu’un relevé de caisse, produisent une navigation vers `bank`. Le module bancaire filtre ensuite ces objets et ne les montre pas. Seul le résultat représentant directement le compte espèces dispose actuellement d’une destination adaptée.

**Idée :** conserver les références nécessaires au classement des opérations/relevés et choisir la destination selon leurs comptes. Définir un parcours explicite pour une opération mixte banque/espèces. Le guide promet l’ouverture du module ; une sélection automatique de l’objet serait une amélioration distincte.

### SHELL-01 — Agréger les opérations actives avant de libérer la navigation

**P2 · Chevauchement reproduit par probe UI.** [Callbacks du registre](../../src/features/registry.tsx#L85), [raccordement du contrôle](../../src/features/registry.tsx#L182), [consolidation](../../src/features/registry.tsx#L257), [wrapper App](../../src/App.tsx#L185).

Le calcul de consolidation et une sauvegarde d’actif peuvent attendre simultanément. Ils écrivent dans le même booléen d’activité ; terminer le calcul remet navigation et date D à disposition alors que la sauvegarde est encore en cours. Aucun contournement des validations persistantes n’est établi.

**Idée :** agréger des états identifiés par opération/composant et garder le blocage jusqu’à leur extinction complète. Vérifier les deux ordres de résolution et le démontage d’un enfant sans effacer l’activité d’un autre.

### SHELL-02 — Garder les données masquées jusqu’au dernier chargement K

**P2 · Mauvaise étiquette de connaissances temporaire reproduite par probe UI.** [Sélection et finalisation](../../src/App.tsx#L104), [montage du registre](../../src/App.tsx#L168), [sélection depuis Historique](../../src/features/history.tsx#L46).

Depuis Historique, choisir K4 puis K2, naviguer vers Famille et terminer seulement la réponse K4. Son `finally` libère `knowledgeLoading` alors que K2 attend encore ; le registre courant K5 peut s’afficher sous le contexte K2. La résolution K2 rétablit le bon affichage. Les écritures historiques restent désactivées.

**Idée :** protéger aussi la finalisation par l’identité de requête, et masquer le registre tant que son chargement est actif ou que son K ne correspond pas au contexte. Couvrir les réponses dans les deux ordres et les changements de page.

### DASH-03 — Proportionner éventuellement l’abscisse à la durée écoulée

**P3 · Proposition de lecture graphique, aucun montant faux.** [Historique](../../src/features/dashboard-charts.tsx#L56), [observations société](../../src/features/dashboard-charts.tsx#L157).

Les axes de dates sont catégoriels : deux dates successives occupent le même espacement, même pour une première/dernière période incomplète ou des observations séparées par une lacune. Dates et valeurs exactes restent présentes dans les tables.

**Idée :** utiliser un axe temporel proportionnel ou expliciter l’espacement par observation. Préserver les interruptions de valeurs inconnues et ne pas faire interpréter la pente comme un rendement.

Compléments : la fermeture de recherche refocalise son bouton après que la navigation a focalisé le titre ; un probe confirme que ce second RAF reprend le focus. Étendre REG-05 pour distinguer fermeture simple et navigation vers une page. REG-02/04 concernent aussi les bilans répétés du dashboard (courant, membres, historique, date finale recalculée), sans latence propre mesurée. Un probe confirme aussi qu’un brouillon de paramètres disparaît à la navigation : une conservation locale ou une indication d’abandon reste une proposition de confort. La largeur CSS minimale de 360 px et le contraste du focus restent à qualifier sur les tailles/zooms réellement pris en charge, sans panne desktop supplémentaire affirmée ici.

**Partie 8 terminée — tableaux de bord, recherche et composants d’interface communs.** Six pistes supplémentaires, dont cinq défauts reproduits ; aucune correction appliquée. Prochaine partie : SQLite, migrations, fichiers et interface native Tauri.

## Partie 9 — SQLite, migrations, fichiers et interface native Tauri

Périmètre : `db.rs`, six migrations, fichiers et démarrage/verrou/générations de `lib.rs`, handlers natifs, capacités/CSP, transport `api.ts`, contrats et suites correspondantes. Les flux déjà examinés en partie 7 sont rapprochés des primitives générales, sans dupliquer PORT-01/02.

Contrôles exécutés : **26 tests existants distincts réussis**, sans échec ni ignoré : `registry_migrations` 14, `vault` 4, récupération ciblée CURRENT/journal/verrou/génération manquante 8. Les quatre tests vault lancés par deux sous-revues sont comptés une seule fois. Inventaire statique : **94 invoke, 94 handlers et 94 enregistrements**, arguments et retours nominalement concordants ; ce n’est pas un test IPC natif exécuté. Les six empreintes SQL correspondent au récupérateur autonome. Probes hors dépôt : métadonnées techniques altérées, observation des appels fsync/mkdir/rename sans modification d’E/S, six publications avec puis sans capacité de lien physique, trois échecs de démarrage du binaire ELF existant et un bloc source d’ouverture extrait. Aucun nouveau paquet construit ; aucune panne physique ni essai réel FAT/exFAT prétendu.

À préserver : validation du schéma exact et des checksums, copies candidates pour les migrations, conservation des octets/générations sources, snapshot WAL via l’API SQLite, mutex et spawn_blocking, sélection explicite des chemins externes, absence de permission shell/fs frontend et ouverture sans shell. Aucun contournement d’accès ni défaut de migration confirmé.

### STORE-01 — Compléter la durabilité des nouveaux dossiers documentaires

**P1 · Garantie de durabilité à compléter ; perte non reproduite.** [Création récursive](../../crates/infrastructure/src/files.rs#L68), [import](../../crates/infrastructure/src/lib.rs#L321), [remplacement](../../crates/infrastructure/src/documents.rs#L406).

L’observation des appels confirme que les nouveaux dossiers `documents/`, `documents/sha256/` et leur préfixe sont créés, mais seul le dossier final du blob est explicitement synchronisé avant le succès. SQLite synchronise sa propre base et son dossier ; aucun fsync des deux ancêtres documentaires n’est observé. Le protocole ne garantit donc pas explicitement la persistance de toute la chaîne référencée par la transaction SQLite après une panne. Il ne s’agit pas d’une perte constatée : aucune coupure physique n’a été exercée.

**Idée :** publier les nouveaux répertoires avec synchronisation de leurs parents dans l’ordre nécessaire avant de confirmer la référence en base ; couvrir import, remplacement et réutilisation d’un blob. Qualifier la reprise avec un modèle de panne adapté, en conservant les originaux et sans modifier les migrations livrées.

### STORE-02 — Prendre en charge ou expliquer les supports sans liens physiques

**P2 · Dépendance de système de fichiers confirmée.** [Publication exclusive](../../crates/infrastructure/src/files.rs#L197).

La publication finale impose `hard_link`. Les systèmes FAT/exFAT usuels sur clés USB n’offrent pas cette capacité. Six parcours d’export réussissent normalement ; une injection ciblée EPERM sur link/linkat les fait échouer proprement avant publication, sans fichier final/temporaire ni mutation du coffre. Ce probe ne remplace pas un test sur un vrai support FAT/exFAT ; l’environnement testé utilise overlay.

**Idée :** proposer une publication exclusive adaptée aux supports pris en charge, en préservant le refus d’écraser une destination apparue entre-temps, ou annoncer clairement cette limite. Contournement actuel : exporter sur un disque local compatible, vérifier le fichier puis le copier sur le support. Ce cas diffère de PORT-02, où une destination est déjà publiée.

### STORE-03 — Valider les métadonnées techniques et la correspondance des audits

**P2 · Données techniques incohérentes acceptées dans un coffre synthétique altéré hors API.** [Validation](../../crates/infrastructure/src/db.rs#L276), [FK indépendantes](../../crates/infrastructure/migrations/0001_foundation.sql#L21).

Des timestamps techniques `not-a-timestamp` et un command_id non UUID passent réouverture, sauvegarde et analyse de restauration ; l’historique expose la date invalide. Un audit de révision A redirigé vers objet/changement B, tout en conservant revision_id A, passe aussi ces validations. Les FK individuelles sont satisfaites, sans garantir leur correspondance commune. Les commandes ordinaires ne sont pas démontrées produire ces états et aucun montant faux n’est établi. Les contrôles positifs refusent bien checksum de migration erroné et trigger supplémentaire.

**Idée :** valider formats techniques et cohérence audit/révision/objet/changement au niveau applicatif, avec diagnostics précis, sans réécrire les migrations. Préserver la distinction entre validation interne et authenticité cryptographique d’une archive.

### STORE-04 — Prévoir une échéance ou annulation du snapshot

**P2 · Proposition de résilience ; blocage non reproduit.** [Copie SQLite](../../crates/infrastructure/src/db.rs#L343), [sauvegarde](../../crates/infrastructure/src/archives.rs#L102).

`Backup::run_to_completion` de la dépendance épinglée recommence sur Busy/Locked sans échéance globale. Le timeout SQLite de cinq secondes existe bien, mais ne borne pas cette boucle de nouvelles tentatives. Le mutex limite la concurrence interne ; un verrou persistant d’un outil externe reste une frontière à qualifier.

**Idée :** ajouter une échéance ou une annulation avec nettoyage de la candidate, sans interrompre une publication déjà effectuée ni confondre attente et corruption.

### NATIVE-01 — Présenter les erreurs de démarrage au lancement graphique

**P2 · Proposition étayée par trois parcours du binaire existant.** [Sortie terminal](../../src-tauri/src/main.rs#L927), [ouverture avant interface](../../src-tauri/src/main.rs#L935).

Chemin de coffre relatif, verrou occupé et CURRENT invalide quittent avec code 1 et seul stderr avant la fenêtre principale. Les données sont conservées. Au double-clic, l’utilisateur peut ne pas voir cette explication.

**Idée :** présenter une erreur native lisible avec la cause et l’action adaptée, tout en conservant le diagnostic terminal. Distinguer aussi la vraie contention du verrou d’une autre erreur de `try_lock_exclusive`, actuellement toutes classées « déjà ouvert » (`lib.rs:73`). Ce périmètre commence après l’entrée dans le programme : il ne prétend pas intercepter une incompatibilité du chargeur glibc.

### NATIVE-02 — Rendre les échecs du lecteur de document consultables

**P2 · Proposition de diagnostic ; aucune fausse confirmation d’ouverture effective établie.** [Lancement et attente](../../src-tauri/src/main.rs#L879), [message UI exact](../../src/features/documents.tsx#L69).

Le code de sortie de xdg-open est ignoré. Un probe du bloc source extrait, avec un lanceur quittant en code 3, confirme le retour Ok. L’interface dit seulement que l’ouverture a été demandée, ce qui décrit correctement l’envoi de cette demande.

**Idée :** récupérer le statut d’échec et proposer l’export manuel ou le choix du lecteur, sans bloquer le mutex du coffre pendant la consultation. Rapprocher la rétention de la copie de DOCU-03.

ARCH-01/02 restent les pistes transversales pour contrats dupliqués et codes d’erreur ; l’inventaire courant n’a trouvé aucun raccordement manquant. Les probes n’ont modifié aucun fichier suivi ni coffre d’utilisateur.

**Partie 9 terminée — SQLite, migrations, fichiers et interface native Tauri.** Six pistes supplémentaires ; aucune correction appliquée. Prochaine partie : tests transversaux, outils, CI, dépendances et maintenance.

## Partie 10 — Tests transversaux, outils, CI, dépendances et maintenance

Périmètre achevé le **16 novembre 2026, Europe/Paris** : manifestes/verrous, versions/outils/CI, scripts de préparation et construction, notices et provenance, qualifications desktop/accessibilité/performance/mise à jour AppImage, audits locaux de dépendances, index de livraison et cohérence finale du rapport. Les scripts de récupération étaient clos en partie 7. Le retrait du `.deb` est complété dans `distribution/0.1.1/README.md`, où le reclassement distant avait laissé une annonce périmée.

Contrôles : typecheck réussi ; 7 contrôles OSV réussis et comparaison JS rejouée sur 203 couples dont 46 de production, 37 729 avis et 10 698 déclarations npm : zéro correspondance et zéro plage non résolue. Base inchangée du 12 novembre, pas de recherche d’avis nouveaux au 16. Audit Rust hors ligne sur la base du 12 novembre : 432 dépendances, zéro entrée dans la liste de vulnérabilités et les deux avertissements conservés. Vérifications de syntaxe ciblées (Python AST, bash -n, node --check) réussies. Deux probes CLI confirment MAINT-04 ; un probe synthétique confirme MAINT-06. Les archives des deux distributions passent leurs SHA256SUMS, l’AppImage Pop!_OS reste `53ec6527842cbfeac228489af081f1602e5f2c6deb98f7d11980924beaa45e2b`. Notices et fichiers de provenance contrôlés par empreintes ; les résultats ABI antérieurs sont consultés, pas relancés. Aucune reconstruction ni campagne native complète relancée.

Complément ARCH-01 : inventaire de 94 wrappers/handlers, comparaison de 143 types communs et 1 156 champs, puis cinq probes du transport avec invoke mocké, sans incompatibilité trouvée. Cela ne remplace pas un test IPC natif. La CI inclut déjà le récupérateur Python et les tests Rust/UI ; son contrôle OSV courant est un autotest de l’algorithme, pas une comparaison à des avis récents.

### MAINT-01 — Exclure les copies de construction de la découverte des tests

**P2 · Découverte dupliquée confirmée.** [Configuration Vitest](../../vite.config.ts#L10), [commande de contrôle](../../scripts/check.sh#L5).

`vitest list --filesOnly` découvre 38 fichiers : 19 suites suivies et leurs 19 copies sous `.local/pop-os22/source`. `.gitignore` n’est pas une exclusion du runner. Le cas documents/portabilité a effectivement exécuté 44 tests au lieu de 22 sans exclusion explicite. Ce n’est pas une panne du CI sur clone propre.

**Idée :** borner les suites au répertoire source ou exclure les copies et répertoires d’outils. Préserver les suites présentes, et distinguer tests uniques, relances et copies dans les rapports.

### MAINT-02 — Harmoniser les métadonnées avec la licence ajoutée au dépôt

**P2 · Contradiction confirmée après l’ajout distant de GPLv3.** [Licence du dépôt](../../LICENSE), [workspace](../../Cargo.toml#L10), [générateur](../../scripts/collect-third-party-licences.py#L314), [guide](../technique/licences-tierces.md#L3).

README et LICENSE annoncent GPLv3, tandis que Cargo, le générateur, les notices et leur manifeste conservent `UNLICENSED`. Les trois crates bibliothèques n’héritent pas non plus de la licence du workspace. Le packaging embarque les notices existantes.

**Idée :** aligner métadonnées, génération et explications des constructions futures sur le choix de licence publié. Conserver les licences des tiers et les preuves historiques telles qu’elles étaient ; ne pas prétendre que les binaires déjà construits ont été reconstruits ou que leurs métadonnées ont changé.

### MAINT-03 — Organiser le renouvellement des bases d’avis et le suivi des avertissements

**P2 · Proposition de maintenance, pas découverte d’une nouvelle vulnérabilité.** [Revue de sécurité datée](../technique/revue-securite.md#L20), [comparateur](../../scripts/audit-js-offline.mjs).

Les preuves utilisent des bases publiques du 12 novembre 2026. La relance Rust hors ligne conserve zéro entrée dans la liste de vulnérabilités, **avec deux avertissements** : proc-macro-error non maintenu et GLib 0.18.5 affecté par un défaut de sûreté mémoire. Rejouer la même base ne recherche pas les avis publiés depuis. La chaîne GTK/Tauri impose des contraintes : une montée isolée de GLib n’est pas validée ici.

**Idée :** définir un renouvellement des bases et une revue des deux avertissements lors des nouvelles constructions/changements GTK/Tauri. Conserver commit/date/couverture des bases et le périmètre de l’analyse d’atteignabilité. Aucune mise à jour de verrou ni envoi d’inventaire externe n’est effectué pendant la revue.

### MAINT-04 — Faire dépendre la qualification de la sortie réelle du processus

**P2 · Faux succès du superviseur de performance reproduit.** [Lecture du résultat](../../scripts/qualify-performance.py#L190), [session](../../scripts/qualify-performance.py#L236), [fin de session](../../scripts/qualify-performance.py#L263), [succès global](../../scripts/qualify-performance.py#L307).

Un faux driver imprime un JSON `operation=open, status=passed, open_seconds=0.01`, puis quitte en code 7. Le script de qualification quitte pourtant en code 0 dans les deux modes. En mode isolé, il conserve exit_code=7 sans le prendre en compte ; en session, il le remplace par 0. `provisional_target_met` vaut aussi true. Ce probe prouve un défaut de l’outil, **sans établir que les anciennes campagnes ont rencontré ce scénario**.

**Idée :** conserver séparément le résultat métier et l’état réel du processus, exiger une fin normale, superviser la clôture de session et distinguer « encore actif » de « terminé avec succès ». Couvrir erreur après JSON valide, signal, timeout et JSON invalide.

### MAINT-05 — Contrôler les notices avant chaque construction générique

**P2 · Proposition de garde des constructions futures ; archives actuelles non déclarées défectueuses.** [Construction générique](../../scripts/build-linux.sh#L20), [rapport du collecteur](../../scripts/collect-third-party-licences.py#L311), [garde Ubuntu existante](../../distribution/0.1.1-popos22.04/construction/inside-build.sh#L13).

Le chemin générique embarque les ressources de notices existantes sans vérifier leur fraîcheur par rapport aux verrous/AppDir. Le collecteur produit des listes de notices manquantes et bibliothèques non identifiées, mais peut terminer en succès avec ces listes non vides. La recette Ubuntu possède déjà des assertions puis reconstruit après la collecte finale.

**Idée :** vérifier empreintes des verrous, collecter les notices de l’AppDir final et faire échouer explicitement la construction si des éléments requis restent inconnus/manquants. Préserver les preuves des constructions déjà qualifiées.

### MAINT-06 — Garder l’origine du paquet lors du choix de sa notice native

**P2 · Cas synthétique reproduit dans le collecteur, impact sur les archives existantes non établi.** [Candidat sysroot/hôte](../../scripts/collect-third-party-licences.py#L239), [notice choisie séparément](../../scripts/collect-third-party-licences.py#L269).

Un candidat sysroot existant mais ancien masque un candidat hôte dont le binaire correspond pourtant au fichier embarqué. Autre branche reproduite : un binaire et sa version sont correctement identifiés dans la base hôte, mais le copyright est choisi prioritairement dans le sysroot ancien. Les empreintes cohérentes des notices actuelles ne prouvent ni ne démontrent une occurrence de ce cas dans les paquets distribués.

**Idée :** essayer les candidats jusqu’à correspondance, conserver l’origine/version retenue et choisir la notice de ce même paquet/environnement. Couvrir coexistence de sysroot et hôte divergents, sans mélanger leurs métadonnées.

Compléments de maintenance : les trois bibliothèques Rust pourraient hériter explicitement de `rust-version` et de la licence du workspace ; les versions courantes concordent et la toolchain racine est épinglée. Le setup teste GTK/WebKit avant d’installer aussi patchelf/Xvfb/xauth/D-Bus : vérifier ces prérequis séparément renforcerait le cas où GTK est déjà présent ; tous sont disponibles ici, aucune panne actuelle n’est affirmée. La qualification desktop pourrait proposer des scénarios ciblés pour le diagnostic tout en conservant une campagne complète. Les limites a11y sont annoncées, les résultats axe incomplets restent conservés ; aucune certification intégrale implicite n’est déduite. Les dépassements de performance documentés restent distincts d’un oracle métier réussi.

**Partie 10 terminée — tests transversaux, outils, CI, dépendances et maintenance.** Six pistes supplémentaires ; aucune correction appliquée. **Les dix parties sont maintenant examinées.**

## Priorités pour la phase de correction

| Ordre | Référence | Résultat attendu | Preuve actuelle |
|---|---|---|---|
| 1 | FIN-01 | Une créance locative ne disparaît que si le financement représente la même créance | Défaut via services persistants |
| 2 | MET-01 | Le transfert vise toujours le lot annoncé à l’ouverture du formulaire | Mauvais payload reproduit dans l’UI |
| 3 | PORT-01 | L’interface indique le coffre réellement actif et permet sa reprise après erreur | Injections EIO, anciennes générations conservées |
| 4 | STORE-01 | Les nouveaux dossiers documentaires sont durablement publiés avant la référence SQLite | Appels observés ; aucune perte physique reproduite |

Traiter ensuite les défauts P2 de calcul/complétude, d’historique et d’affichage (notamment MET-02/03, DASH-01, SHELL-01/02, FIN-02 et SEARCH-01), puis les parcours documentaires et les garanties d’outillage. Les propositions et hypothèses gardent leur qualification propre : une optimisation non mesurée n’est pas un bug de montant.

À chaque reprise, lire ce fichier et l’identifiant concerné avant de modifier quoi que ce soit. Intégrer le fichier de correctifs personnels annoncé lorsqu’il sera fourni, conserver les décisions utiles, puis transformer les reproductions retenues en tests de régression versionnés au moment de corriger. Les sources, dépendances verrouillées et migrations applicatives n’ont pas été corrigées pendant la revue. L’unique exception exécutée est le retrait demandé du `.deb` avec ses annonces et sa génération courante ; l’AppImage publiée reste inchangée.
