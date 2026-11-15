# Revue des améliorations possibles

Revue commencée le **14 novembre 2026, Europe/Paris**, à la demande de l’utilisateur. Référence de lecture : commit `af8c88c319245454ec25c1966f0715198185f952`.

Ce fichier contient des observations et des propositions. Il ne met en œuvre aucune correction. Seul ce document est ajouté ou mis à jour pendant la revue ; les sources, tests, migrations, dépendances et documents existants sont conservés.

## Point de reprise

- **Dernière partie terminée : 4 — Immobilier et loyers.**
- **Partie suivante : 5 — Trésorerie, financements et bilan financier.**
- La partie suivante commence seulement après la clôture de la partie en cours.
- « Terminée » signifie que le périmètre de revue et ses observations sont consignés, pas que les améliorations sont réalisées.

## Ordre de la revue

| Partie | Périmètre | État |
|---|---|---|
| 1 | Documentation, cahier des charges et explications | Terminée |
| 2 | Architecture applicative, contrats et primitives de calcul | Terminée |
| 3 | Registre, propriété, consolidation, devises et historique | Terminée |
| 4 | Immobilier et loyers | Terminée |
| 5 | Trésorerie, financements et bilan financier | À examiner |
| 6 | Métaux précieux et sociétés | À examiner |
| 7 | Documents, sauvegardes et portabilité | À examiner |
| 8 | Tableaux de bord, recherche et composants d’interface communs | À examiner |
| 9 | SQLite, migrations, fichiers et interface native Tauri | À examiner |
| 10 | Tests transversaux, outils, CI, dépendances et maintenance | À examiner |

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

**P1 · Point à vérifier, pas un bug du moteur établi.** [Règles initiales](../technique/regles-metier.md#L71), [éliminations](../technique/regles-metier.md#L148), [décision de financement](../technique/decisions-architecture.md#L90).

Les règles initiales annoncent un net familial égal à la somme des nets individuels et une élimination identique sur les deux faces. La décision ultérieure retient un appariement nominal et une réduction de la créance proportionnelle à sa valeur comptable. Pour un prêt familial nominal de 100 000 EUR déprécié à 80 000 EUR, les deux conventions ne donnent pas nécessairement le même résultat consolidé.

**Idée :** ajouter cet exemple chiffré à la décision, indiquer explicitement les réductions de chaque face et les conditions de l’égalité des nets. Vérifier la cohérence du contrat courant et des tests pendant la partie 5 ; ne pas transformer une règle historique en assertion générale sans préciser ses exceptions.

### DOC-02 — Harmoniser les explications sur les documents à une ancienne connaissance K

**P2 · Contradiction observée dans les guides.** [Devises/historique](../utilisateur/devises-et-historique.md#L32), [immobilier](../utilisateur/immobilier-et-loyers.md#L82), [financements](../utilisateur/comptes-et-financements.md#L185), [guide documentaire](../utilisateur/documents-et-portabilite.md#L14).

Les trois premiers textes annoncent que les documents concernent toujours le coffre actuel ; le guide documentaire explique que leur consultation suit K et que les écritures sont bloquées à K fixe. Ces pages restent accessibles depuis le parcours utilisateur courant.

**Idée :** distinguer dans chaque guide la bibliothèque consultée à K des paramètres et exports intégraux du coffre actuel. Ajouter un renvoi vers le guide documentaire. Critère : les mêmes notions D/K donnent la même réponse dans tous les guides ; le comportement réel sera examiné en partie 7.

### DOC-03 — Rendre les renvois documentaires utilisables depuis GitHub

**P2 · Limitation de navigation observée.** [Guide Linux, lien du kit](../utilisateur/installation-linux.md#L126), [rapport courant](../historique/validation-phase-10.md#L7), [explication du README](../../README.md#L15), [téléchargements versionnés](../../distribution/README.md#L45).

Les 77 renvois vers `.local/` ou `/workspace/.tooling/` fonctionnent dans le cloud mais pas dans un clone ordinaire. Le kit possède déjà une destination versionnée sous `distribution/0.1.1/`. Le ZIP documentaire fournit aussi des copies adaptées des preuves : ce constat ne signifie pas que les preuves finales sont perdues.

**Idée :** remplacer le lien de téléchargement du kit par sa destination versionnée ; créer un index stable des preuves et renvoyer les pages GitHub vers cet index ou le ZIP correspondant. Distinguer les pièces publiées, les anciennes pièces conservées seulement dans l’environnement et les commandes permettant de reproduire les contrôles. Vérifier les liens à partir des seuls fichiers versionnés, en complément du contrôle local.

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

**Idée :** choisir une borne unique et la distinguer de la quantité restante, qui peut être nulle après une vente complète. La règle effectivement appliquée sera contrôlée dans la partie 6 avant de proposer une correction du code.

### DOC-07 — Distinguer les programmes couverts par chaque campagne

**P2 · Proposition de traçabilité.** [Parcours du paquet](../historique/validation-phase-10.md#L40), [accessibilité](../historique/validation-phase-10.md#L59), [manifeste de construction](../../distribution/0.1.1/build-manifest.json).

Le parcours natif et la mise à jour sont bien rattachés à l’AppImage finale par SHA-256. Les contrôles d’accessibilité utilisent un ELF de développement et les performances plusieurs binaires de service ; leurs identités sont précisées dans les preuves, mais dispersées. Le manifeste de construction contient les empreintes des dépendances verrouillées, sans commit ni arbre source.

**Idée :** ajouter une matrice « campagne → date → type/empreinte du programme → sources connues → portée ». Pour les constructions futures, enregistrer commit, état modifié et empreinte des sources. Pour les anciennes, signaler les métadonnées non enregistrées sans les inventer. Le ZIP des sources correspond à `2c2fe966` ; les commits suivants jusqu’à la base de revue ajoutent README et distribution, sans divergence applicative démontrée.

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
