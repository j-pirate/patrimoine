# Validation — phase 6 : comptes, financements et bilan financier

Cette campagne utilise uniquement des données synthétiques et des coffres isolés. Elle couvre le journal des comptes/caisses, les rapprochements, financements, avances/dépôts, droits sur les créances locatives et leur bilan familial ou individuel. Les rapports incomplets conservent leurs sous-totaux connus sans les présenter comme un net exhaustif. Le [guide](../utilisateur/comptes-et-financements.md) décrit les formulaires disponibles.

## Vérification complète

Le 11 novembre 2026 (Europe/Paris), `source scripts/cloud-env.sh` puis `CARGO_BUILD_JOBS=3 bash scripts/check.sh` : **réussi, sortie 0**. Journal local : [check-phase6.log](/workspace/.tooling/check-phase6.log).

| Campagne | Résultat observé |
|---|---|
| TypeScript et Vite | Types et build réussis |
| Vitest | **97 tests**, 11 fichiers, aucun échec ni ignoré |
| Domaine Rust | **152 tests** : 87 précédents, 21 trésorerie, 18 financement, 8 adaptations immobilier/loyers, 18 bilan financier |
| Infrastructure Rust | **129 tests**, notamment 11 trésorerie, 13 financement et 12 migrations |
| Workspace Rust | **281 tests réussis**, aucun échec ni ignoré ; application, binaire et doctests sans cas supplémentaires |
| rustfmt / Clippy | Formatage conforme, toutes cibles et fonctionnalités, `-D warnings` réussi |
| Tauri Linux | Binaire `custom-protocol` avec interface embarquée construit |

Les tests UI utilisent des réponses IPC contrôlées ; ils vérifient les saisies décimales textuelles, dépendances, erreurs conservant la saisie, sélection des composantes et révisions, chargement historique et lecture seule. Le parcours natif ci-dessous vérifie séparément le pont réel vers Rust et SQLite.

## Oracles et invariants métier

| Cas | Contrôle attendu et vérifié |
|---|---|
| Ouverture absente / explicitement nulle | Solde inconnu / zéro connu ; aucune estimation transformée en ouverture |
| Ouverture 100, entrée 30, sortie 20 ; relevé 100 | Journal 110, écart −10, aucune écriture automatique |
| Transfert de 50 entre comptes | Deux jambes opposées ; aucun revenu ni duplication du patrimoine |
| Conversion 100 EUR → 330 TND, frais 2 EUR | Taux exécuté exact 33/10 ; frais distincts ; cours de stock indépendant |
| Banque négative / caisse négative | Découvert en passif / état de caisse connu négatif refusé |
| Principal 100 000, remboursement 1 000, intérêts 200, frais 10 | Principal 99 000 ; débit bancaire 1 210 ; trois composantes liées une seule fois |
| Échéancier et garantie non appelée | Prévision sans mouvement de principal ; garantie hors dette certaine |
| M07/M08/M09 du cahier des charges | Neutralisations internes intégrales ou proportionnelles aux expositions réellement détenues |
| Société globale et dette couverte | Pas de sous-jacents ni dette ajoutés une deuxième fois ; couverture absente conservée inconnue |
| Créance nominale 7 500, valeur 6 750 ; dette visible 4 000 | Élimination 3 600, résidu créance 3 150 et dette 400, net 2 750 |
| Dépôt +2 000, compensation −300, restitution −500 | Obligation restante 1 200 ; compensation sans nouveau cash ni loyer reçu |
| Avance et échéance future | Obligation du détenteur et droit réciproque du locataire ; famille interne neutralisée |
| Créance de 1 000, dépôt compensé 300 puis cession | Droits cédés sur le reliquat 700, sans recréer 1 000 |
| Dépréciation/reprise de loyer | Plafonds par créancier et composante à chaque date ; dette locataire inchangée |
| Créance déjà représentée par un financement | Cohérence au transfert ; pas de deuxième créance ajoutée |
| Paiement, dépense payée, tirage ou remboursement sans cash lié | Net incomplet ; banque inchangée par hypothèse, même après déclaration d’inventaire |
| Flux locatif complet 12 000 − exploitation 2 000 − service de dette 4 000 | Cash-flow après financement 6 000 ; NOI et principal restent distincts |

Les cas de bilan sont construits indépendamment des helpers de production, avec résultats rationnels explicites. Les tests conservent les tiers, participations partielles, sociétés opaques, changements de créanciers et frontières temporelles ; une table vide ne prouve pas l’absence de passif. Une ouverture historique ne réclame pas un flux bancaire nouveau. Les débits réels exigent en revanche les liens appropriés, par face et composante.

## Persistance et historique

Les tests SQLite couvrent les mutations en un seul changement, rollback intégral, anciennes connaissances K et révisions courantes. Loyers, dépenses payées, dépôts et remboursements corrigés conservent une seule origine bancaire. Réutiliser une autre opération, créer un second cash, corriger isolément une source liée ou utiliser une séquence/révision périmée sont refusés sans mutation partielle. Une ligne partagée ne peut pas être corrigée en laissant les autres liens obsolètes.

Les références exactes des sources, transactions et lignes sont conservées et plafonnées. Dates, devises, signes, détenteurs et composantes sont contrôlés. Les snapshots du registre, immobilier, locations, trésorerie, financements et créances d’un rapport sont lus dans une même transaction SQLite.

Les fixtures utilisent de vrais schémas historiques 1–4, puis contrôlent l’ouverture et les archives vers le schéma 5. Les fichiers SQL antérieurs ne sont pas modifiés. La copie candidate passe les contrôles d’intégrité et de clés étrangères avant activation ; les anciennes générations et archives restent conservées. Une restauration d’un dépôt corrigé conserve ses versions, liens et ancien K.

## Parcours Linux natif

`bash scripts/test-desktop-cloud.sh` : **réussi, sortie 0**, sur le binaire final. Le [rapport JSON](../../.local/desktop-smoke.yLQnPQrW/desktop-result.json) contient **37 étapes réussies**, aucune échouée ni non exécutée et `native_dialogs_requested: true`. Debian 13, WebKitGTK/Xvfb, utilisateur non privilégié, conteneur sans réseau et coffre neuf ; les parcours antérieurs du registre, devises, historique et immobilier restent exécutés.

Les nouveaux contrôles emploient les commandes Tauri réelles, les formulaires React et SQLite :

- Compte bancaire créé depuis l’interface, absence de solde tant que l’ouverture manque, puis ouverture confirmée de 100 EUR depuis le formulaire.
- Deux autres comptes explicitement ouverts à zéro ; entrée 30 et sortie 20, relevé 100 contre journal 110, écart −10 sans mutation ; transfert 50 ; conversion 100 EUR en 330 TND et frais 2 EUR. Soldes : banque A −42 EUR, banque B 50 EUR, caisse 330 TND ; taux exécuté 33/10, valorisation de la caisse 102,30 EUR au cours indépendant de 0,31.
- Prêt externe ouvert à 100 000 EUR, échéance prévue, remboursement réel de principal 1 000, intérêts 200 et frais 10 : trois liens sur un seul débit de 1 210 EUR, principal 99 000 EUR. Une garantie non appelée de 50 000 reste contingente. Un autre contrat de 10 000 entre membres est neutralisé des deux côtés dans la famille.
- Dépôt de 2 000 reçu depuis le formulaire et lié à la banque B ; compensation de 300 sur l’échéance de janvier sans nouvelle transaction ; restitution de 500 avec débit. Solde banque B 340 EUR, obligation 1 200 EUR et impayé de janvier 700 EUR ; loyer reçu annuel inchangé à 1 000 EUR. À l’ancien K, le solde bancaire reste −1 160 EUR.
- Déclaration de couverture depuis l’interface, calcul du bilan et contrôle du découvert en passif. Malgré la déclaration, le net reste absent parce que des anciennes sources locatives du scénario n’ont pas été rapprochées. Les sous-totaux connus sont 351 452,30 EUR d’actifs et 100 242 EUR de passifs ; leur différence 251 210,30 EUR reste explicitement partielle.
- Dialogues GTK d’import/export documentaire, sauvegarde complète, analyse/annulation sans mutation, confirmation avec sauvegarde préalable et activation d’une nouvelle génération. Les six snapshots métier, identités, révisions, liens, anciennes connaissances et soldes sont reproduits. Le [rapport financier avant/après](../../.local/desktop-smoke.yLQnPQrW/financial-restore-comparison.json) est entièrement identique à K=91, y compris les positions et leurs traces de conversion. Les [oracles du scénario](../../.local/desktop-smoke.yLQnPQrW/financial-oracles.json) sont conservés.

Un premier essai du pilote comparait les positions financières calculées avant l’import documentaire à celles restaurées après cet import. L’import augmente K et les traces FX conservent cette séquence : les montants et sources métier étaient conservés, mais la comparaison utilisait deux connaissances différentes. Le pilote capture désormais le rapport après l’import, immédiatement avant sauvegarde, puis vérifie sa reproduction intégrale à K égal. Le second parcours complet réussit ; aucune assertion financière n’a été retirée et aucun calcul applicatif n’a été modifié pour ce motif.

Les saisies textuelles utilisent le clavier WebDriver avec vérification après rendu. Les champs date natifs emploient `input/change` et deux images de stabilisation. Les fixtures complémentaires sont créées par IPC réel ; aucun état React ni résultat financier simulé n’est injecté. Les captures et archives restent des artefacts locaux ignorés par Git.

## Inspection indépendante des archives

Une lecture indépendante des deux ZIP avec les bibliothèques standard Python `zipfile`, `hashlib`, `sqlite3` et `fractions.Fraction`, sans appeler le backend, produit [ce rapport](../../.local/desktop-smoke.yLQnPQrW/independent-backup-inspection.json) : **réussi**.

La sauvegarde principale est à K=91, la préalable à K=92. Les quatre fichiers manifestés par archive ont les tailles et SHA-256 annoncés. Les bases sont au schéma 5, avec 54 tables, 106 objets et 111 révisions ; `integrity_check` répond `ok` et `foreign_key_check` ne retourne aucune ligne. Les migrations SQL 1–4 correspondent exactement à celles des deux archives de phase 5 ; les cinq migrations enregistrées correspondent aux fichiers courants.

Le recalcul rationnel du journal retrouve −42 EUR sur A, 340 EUR sur B et 330 TND en caisse, ainsi que le taux exécuté 33/10 et les frais 2 EUR. Les sources de financement donnent principal 99 000, intérêts 200 et frais 10 ; le prêt interne reste 10 000. Les dépôts donnent 2 000 − 300 − 500 = 1 200 et l’échéance de janvier 1 000 − 300 = 700. Les trois seuls paiements locatifs totalisent toujours 1 000 : le dépôt n’a pas créé de faux paiement.

Les cinq liens de trésorerie pointent les bonnes révisions de source, journal et ligne, avec détenteur, date, devise, capacités et changements atomiques contrôlés. Toutes les données métier et lignes enfants sont identiques entre les deux archives ; seules les mutations attendues des réglages, changements et audit diffèrent. Ces vérifications complètent le round-trip applicatif sans prétendre simuler tous les matériels, systèmes de fichiers ou pannes physiques.

## Limites de cette phase

Une couverture est une déclaration de l’utilisateur, pas une découverte automatique de comptes ou dettes externes. Aucun connecteur bancaire ni cours en ligne n’est ajouté. Les droits inconnus et couvertures ambiguës restent inconnus ; en particulier, la dette d’un découvert joint n’est pas répartie automatiquement selon la propriété des liquidités. Les montants connus sont calculés exactement, mais ils ne certifient pas l’exhaustivité du patrimoine réel.

Les prêts conservent leurs intérêts et frais réellement saisis ; ils ne fabriquent pas de tableau d’amortissement actuariel depuis un taux descriptif. L’application ne déduit pas un mouvement bancaire d’une acquisition ou cession immobilière historique. Le rapport de variation des actifs de phase 4 garde son périmètre ; il ne devient pas une mesure de performance du nouveau bilan financier.

Métaux et indicateurs de sociétés restent en phase 7, exports CSV/XLSX et associations documentaires complètes en phase 8, tableaux de bord en phase 9, qualification AppImage et livraison Linux en phase 10. Aucun nouvel environnement n’a été publié pendant ce lot. Les rapports de validation des phases précédentes restent des preuves de leurs versions respectives.
