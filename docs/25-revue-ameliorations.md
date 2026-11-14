# Revue des améliorations possibles

Revue commencée le **14 novembre 2026, Europe/Paris**, à la demande de l’utilisateur. Référence de lecture : commit `af8c88c319245454ec25c1966f0715198185f952`.

Ce fichier contient des observations et des propositions. Il ne met en œuvre aucune correction. Seul ce document est ajouté ou mis à jour pendant la revue ; les sources, tests, migrations, dépendances et documents existants sont conservés.

## Point de reprise

- **Dernière partie terminée : 2 — Architecture applicative, contrats et primitives de calcul.**
- **Partie suivante : 3 — Registre, propriété, consolidation, devises et historique.**
- La partie suivante commence seulement après la clôture de la partie en cours.
- « Terminée » signifie que le périmètre de revue et ses observations sont consignés, pas que les améliorations sont réalisées.

## Ordre de la revue

| Partie | Périmètre | État |
|---|---|---|
| 1 | Documentation, cahier des charges et explications | Terminée |
| 2 | Architecture applicative, contrats et primitives de calcul | Terminée |
| 3 | Registre, propriété, consolidation, devises et historique | À examiner |
| 4 | Immobilier et loyers | À examiner |
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

**P1 · Point à vérifier, pas un bug du moteur établi.** [Règles initiales](03-regles-metier.md#L71), [éliminations](03-regles-metier.md#L148), [décision de financement](decisions.md#L90).

Les règles initiales annoncent un net familial égal à la somme des nets individuels et une élimination identique sur les deux faces. La décision ultérieure retient un appariement nominal et une réduction de la créance proportionnelle à sa valeur comptable. Pour un prêt familial nominal de 100 000 EUR déprécié à 80 000 EUR, les deux conventions ne donnent pas nécessairement le même résultat consolidé.

**Idée :** ajouter cet exemple chiffré à la décision, indiquer explicitement les réductions de chaque face et les conditions de l’égalité des nets. Vérifier la cohérence du contrat courant et des tests pendant la partie 5 ; ne pas transformer une règle historique en assertion générale sans préciser ses exceptions.

### DOC-02 — Harmoniser les explications sur les documents à une ancienne connaissance K

**P2 · Contradiction observée dans les guides.** [Devises/historique](11-devises-et-historique.md#L32), [immobilier](13-immobilier-et-loyers.md#L82), [financements](15-comptes-et-financements.md#L185), [guide documentaire](19-documents-et-portabilite.md#L14).

Les trois premiers textes annoncent que les documents concernent toujours le coffre actuel ; le guide documentaire explique que leur consultation suit K et que les écritures sont bloquées à K fixe. Ces pages restent accessibles depuis le parcours utilisateur courant.

**Idée :** distinguer dans chaque guide la bibliothèque consultée à K des paramètres et exports intégraux du coffre actuel. Ajouter un renvoi vers le guide documentaire. Critère : les mêmes notions D/K donnent la même réponse dans tous les guides ; le comportement réel sera examiné en partie 7.

### DOC-03 — Rendre les renvois documentaires utilisables depuis GitHub

**P2 · Limitation de navigation observée.** [Guide Linux, lien du kit](23-installation-et-recuperation-linux.md#L126), [rapport courant](22-validation-phase10.md#L7), [explication du README](../README.md#L15), [téléchargements versionnés](../distribution/README.md#L45).

Les 77 renvois vers `.local/` ou `/workspace/.tooling/` fonctionnent dans le cloud mais pas dans un clone ordinaire. Le kit possède déjà une destination versionnée sous `distribution/0.1.1/`. Le ZIP documentaire fournit aussi des copies adaptées des preuves : ce constat ne signifie pas que les preuves finales sont perdues.

**Idée :** remplacer le lien de téléchargement du kit par sa destination versionnée ; créer un index stable des preuves et renvoyer les pages GitHub vers cet index ou le ZIP correspondant. Distinguer les pièces publiées, les anciennes pièces conservées seulement dans l’environnement et les commandes permettant de reproduire les contrôles. Vérifier les liens à partir des seuls fichiers versionnés, en complément du contrôle local.

### DOC-04 — Relier les exigences au périmètre effectivement reçu

**P2 · Proposition de suivi.** [Besoin de variation expliquée](cahier-des-charges.md#L890), [critères initiaux](05-validation-et-plan.md#L130), [limite du pont livré](decisions.md#L130), [avancement](avancement.md#L142).

Les limites du pont de variation sont déclarées : il conserve une portée de registre et ne décompose pas tout le bilan intermodules en flux, valorisation, change et périmètre. Le plan initial seul ne permet toutefois pas de suivre ce reliquat parmi les phases marquées implémentées.

**Idée :** maintenir une matrice courte « exigence → livré / partiel / reste à faire → décision → preuve ». Y intégrer le pont intermodules et les autres conventions de livraison, sans réécrire le cahier des charges d’origine ni présenter les limites déclarées comme des défauts cachés.

### DOC-05 — Ajouter une correspondance entre conception et schéma livré

**P2 · Proposition de maintenance.** [Catalogue conceptuel](02-modele-relationnel.md#L5), [plan initial de migrations](02-modele-relationnel.md#L248), [décisions du schéma](decisions.md#L48), [portabilité au schéma 6](decisions.md#L120).

Le catalogue historique emploie notamment des sous-types et une migration documentaire `0007` qui ne correspondent pas directement au schéma livré. Son caractère prévisionnel est indiqué, mais retrouver la correspondance impose de reconstituer le journal des décisions.

**Idée :** ajouter un tableau « concept initial → table/commande livrée → migration → décision ». Identifier localement les dispositions remplacées : seuils d’ancienneté proposés comme paramétrables, mode strict et filiation automatique des lots, par exemple. Préserver les migrations livrées à l’identique.

### DOC-06 — Unifier la borne de pureté des métaux

**P2 · Contradiction documentaire observée.** [Modèle proposé](02-modele-relationnel.md#L147), [règles proposées](03-regles-metier.md#L270).

Le premier texte impose une pureté dans `]0,1]`, le second dans `[0,1]`.

**Idée :** choisir une borne unique et la distinguer de la quantité restante, qui peut être nulle après une vente complète. La règle effectivement appliquée sera contrôlée dans la partie 6 avant de proposer une correction du code.

### DOC-07 — Distinguer les programmes couverts par chaque campagne

**P2 · Proposition de traçabilité.** [Parcours du paquet](22-validation-phase10.md#L40), [accessibilité](22-validation-phase10.md#L59), [manifeste de construction](../distribution/0.1.1/build-manifest.json).

Le parcours natif et la mise à jour sont bien rattachés à l’AppImage finale par SHA-256. Les contrôles d’accessibilité utilisent un ELF de développement et les performances plusieurs binaires de service ; leurs identités sont précisées dans les preuves, mais dispersées. Le manifeste de construction contient les empreintes des dépendances verrouillées, sans commit ni arbre source.

**Idée :** ajouter une matrice « campagne → date → type/empreinte du programme → sources connues → portée ». Pour les constructions futures, enregistrer commit, état modifié et empreinte des sources. Pour les anciennes, signaler les métadonnées non enregistrées sans les inventer. Le ZIP des sources correspond à `2c2fe966` ; les commits suivants jusqu’à la base de revue ajoutent README et distribution, sans divergence applicative démontrée.

### DOC-08 — Inventorier les anciennes preuves et les critères de réception

**P3 · Proposition de lisibilité des archives.** [Campagne phase 2](06-validation-phase2.md#L99), [phase 3](08-validation-phase3.md#L95), [critères de phase 2](06-validation-phase2.md#L9).

Les anciens dossiers desktop cités en phases 2 et 3 ne figurent pas dans le ZIP distribué ; les preuves finales et celles des phases 4 à 10 sont présentes. Le lien entre identifiants de critères et observations reste souvent en prose.

**Idée :** inventorier les campagnes selon « livrée / conservée seulement dans l’environnement / non retrouvée » et relier chaque critère aux preuves existantes et limites. Séparer contrôle statique, refus adversarial réellement exécuté et panne physique non exercée. Ne pas recréer un résultat ancien à partir d’un test actuel.

**Partie 1 terminée — documentation, cahier des charges et explications.** Huit pistes consignées ; aucune correction appliquée. Prochaine partie : architecture applicative, contrats et primitives de calcul.

## Partie 2 — Architecture applicative, contrats et primitives de calcul

Périmètre : les dix fichiers de contrats de `crates/application/src/`, les primitives `exact.rs`, `operations.rs`, `profiles.rs`, les contrats communs du domaine, les types de transport TypeScript et leur appelant `src/services/api.ts`. Les validations communes sont lues ; la validation temporelle détaillée du registre appartient à la partie 3.

Contrôles ciblés effectués : **8 tests existants réussis**, sans échec ni test ignoré, via `cargo test -p patrimoine-domain --lib --offline exact::tests`, `cargo test -p patrimoine-domain --test fx_tests --offline precision` et `cargo test -p patrimoine-domain --test attribution --offline french_form_normalization`.

À conserver : sources bornées avant conversion en grands entiers, normalisation distincte du stockage canonique, calcul rationnel et arrondi final au pair, précision TND de trois décimales et refus d’une précision implicite pour une devise inconnue. Aucun défaut numérique ni incompatibilité fonctionnelle entre les DTO comparés n’a été établi. Le fait que la couche application expose surtout des ports ne justifie pas à lui seul une refonte.

### ARCH-01 — Contrôler la correspondance des contrats Rust et TypeScript

**P2 · Proposition de prévention des régressions.** [Documents Rust](../crates/domain/src/documents.rs#L6), [documents TS](../src/services/documents-types.ts#L1), [portabilité Rust](../crates/domain/src/portability.rs#L6), [portabilité TS](../src/services/portability-types.ts#L1), [appels typés](../src/services/api.ts#L79).

Les champs comparés concordent actuellement, mais les structures sont recopiées et les types de retour d’`invoke<T>` ne valident pas eux-mêmes les réponses. Un changement Rust peut donc laisser le frontend compiler avec un contrat périmé.

**Idée :** générer les types de transport depuis Rust ou contrôler les schémas sérialisés en CI. Vérifier champs, propriétés obligatoires, nullabilité et valeurs finies, sans déplacer les calculs financiers dans TypeScript. Distinguer également les champs facultatifs d’une ancienne forme acceptée des propriétés normalement émises par le DTO courant : `serde(default)` ne signifie pas une omission en sortie.

### ARCH-02 — Exploiter des codes d’erreur et statuts stables

**P2 · Proposition de maintenance et d’explication.** [AppError](../crates/domain/src/lib.rs#L44), [présentation d’erreur](../src/services/api.ts#L68), [statuts documentaires](../src/services/documents-types.ts#L3).

Les codes d’erreur et plusieurs valeurs finies sont des chaînes libres. Le présentateur commun retient surtout le message, ce qui ne structure pas les actions propres à un conflit de révision, une validation ou une erreur d’accès au support. Cela ne signifie pas une absence de validation backend des valeurs actuelles.

**Idée :** définir des enums sérialisées conservant les valeurs existantes et partager leur contrat avec TypeScript. Associer aux codes des explications et actions adaptées, tout en conservant les messages français. Vérifier les branches inconnues et la compatibilité des anciens codes plutôt que l’orthographe d’un message complet.

### ARCH-03 — Indexer une fois les capacités des lignes de trésorerie

**P2 · Complexité observée, optimisation proposée.** [Agrégation](../crates/domain/src/operations.rs#L13), [recherches répétées](../crates/domain/src/operations.rs#L49).

Après agrégation par `(transaction_revision_id, line_id)`, chaque clé provoque une recherche linéaire dans les transactions puis leurs lignes. Avec U clés utilisées et T transactions, le coût peut atteindre `O(U × T)`, hors recherche des lignes. Aucune latence propre à cette fonction n’a été mesurée ici.

**Idée :** construire un index des capacités par clé avant de vérifier les sommes. Garder les mêmes références exactes, résultats, refus et ordre déterministe. Comparer les cas de capacité partagée existants et mesurer un jeu comprenant plusieurs milliers de lignes ; ne pas retirer les validations pour obtenir un gain.

### ARCH-04 — Compléter quelques frontières des normalisations

**P3 · Proposition de couverture, aucun comportement incorrect observé.** [Normalisation des montants](../crates/domain/src/exact.rs#L32), [zéro négatif](../crates/domain/src/exact.rs#L50), [normalisation des taux](../crates/domain/src/exact.rs#L126).

Les tests couvrent déjà de nombreuses bornes et valeurs non canoniques.

**Idée :** compléter ultérieurement les cas `-0` et `-0,000 → 0`, l’idempotence de la normalisation et les longueurs exactes de 64/65 octets. Ces cas visent les frontières du contrat ; ils ne nécessitent pas une nouvelle représentation monétaire.

**Partie 2 terminée — architecture applicative, contrats et primitives de calcul.** Quatre pistes supplémentaires consignées ; aucune correction appliquée. Prochaine partie : registre, propriété, consolidation, devises et historique.
