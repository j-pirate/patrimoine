# Validation, risques et plan d’implémentation

Ce document définit des **tests à implémenter** et des résultats attendus. Aucun test applicatif n’a été exécuté en phase 1. Tous les jeux de données ci-dessous sont synthétiques.

## 1. Stratégie de validation

1. Tests unitaires Rust du domaine sans UI : résultats chiffrés, invariants et cas inconnus.
2. Tests d’intégration avec fichiers SQLite temporaires réels : migrations, contraintes, transactions, états datés et chemins documentaires.
3. Tests de portabilité dans des répertoires vierges séparés : sauvegarde SQLite **et** réimport CSV indépendants, comparaisons canoniques et recalculs.
4. Tests de panne déterministes aux points de synchronisation : manque de disque, processus interrompu, erreurs d’E/S, import corrompu et migrations interrompues.
5. Tests UI ciblés : formulaires, contexte familial/date, accessibilité, messages français ; E2E desktop pour les parcours transversaux.
6. Compilation et paquets Linux ; scénario fonctionnel avec accès réseau coupé et récupération sans application.

Un test financier vérifie le montant attendu, le périmètre, les sources, les exclusions et le statut de complétude. Une exécution sans assertions métier, avec zéro test ou tests ignorés ne valide pas une phase. Les données réelles et les secrets ne sont jamais nécessaires dans les fixtures. CI doit signaler séparément réussite, échec, ignorance et non-exécution.

## 2. Tests prioritaires et oracles

### P0 — avant toute confiance dans les montants

| ID | Scénario | Résultat attendu |
|---|---|---|
| M01 | A 50 %, B 30 %, tiers 20 % d’un bien de 400 000 EUR | A = 200 000 ; B = 120 000 ; famille = 320 000 ; vue juridique = 400 000 |
| M02 | A possède X à 60 %, X possède Y à 50 %, Y possède un bien de 400 000 | A = 120 000, soit 30 % ; pas de valeur X/Y ajoutée aux sous-jacents |
| M03 | A→X 50 %, A→Y 20 %, X→Z 40 %, Y→Z 50 % ; Z→bien 400 000 | Droit A sur Z = 0,50×0,40 + 0,20×0,50 = 0,30 ; A = 120 000 |
| M04 | A/B/tiers détiennent 40/35/25 % de X ; X possède un bien 400 000 | Famille = 300 000 ; tiers = 100 000 exclu |
| M05 | Une chaîne cyclique est valide aujourd’hui mais cyclique dans un intervalle passé corrigé | Toute l’écriture est refusée ; pas seulement contrôle au jour courant |
| M06 | Propriété 70 % connue, reste non défini ; puis saisie 110 % ou intervalles chevauchants | Premier cas partiel explicitement ; autres refusés atomiquement ; aucun 30 % inventé |
| M07 | A et B possèdent chacun 50 % d’un actif 400 000 ; seul A a une dette externe de 100 000 | A brut 200 000/dette 100 000/net 100 000 ; B net 200 000 ; famille net 300 000 |
| M08 | A prête 10 000 à B, tous deux dans la famille ; observer le contrat seul | Individuel A créance +10 000, B dette −10 000 ; famille créance et dette neutralisées de 10 000 ; net 0 |
| M09 | Créancier société à 75 % familiale, débiteur société à 40 %, contrat 10 000 | Avant élimination : créance 7 500, dette 4 000 ; neutralisation 4 000 des deux ; créance externe résiduelle 3 500 ; net inchangé 3 500 |
| M10 | X détaillée possède immeuble 400 000, banque 30 000, dette externe 100 000 ; famille 75 % | Brut 322 500 ; dette 75 000 ; net 247 500 ; indicateur de trésorerie de X n’est pas rajouté |
| M11 | X opaque, valeur des capitaux propres 200 000, famille 75 % ; immeuble/dette sous-jacents déjà décrits | Participation retenue 150 000 ; sous-jacents exclus sur ces chemins, pas de dette incluse déduite à nouveau |
| M12 | Prêt impliquant X opaque sans couverture explicite ; ou X détaillée déficitaire à responsabilité non qualifiée | Résultat incomplet/part concernée non calculable ; aucune élimination ou dette personnelle supposée |
| M13 | Date demandée 31/01, seul taux CHF daté 01/02 | Conversion impossible au 31/01 ; EUR non disponible, pas zéro ni taux futur |
| M14 | 100 CHF ; 1 CHF = 1,05 EUR | 105,00 EUR ; taux, sens, date et source consultables ; inversion exacte jusqu’à l’arrondi final |
| M15 | Sources simultanées USD→EUR et AED→USD ; chemin USD autorisé comme pivot secondaire | Résultat produit exact des deux ; deux dates visibles ; chemin déterministe ; sans chemin autorisé, résultat impossible |
| M16 | Somme 0,10 EUR + 0,20 EUR et répétition de fractions à plusieurs niveaux | 0,30 exact ; somme exacte avant arrondi, pas d’écarts binaires cumulés |
| M17 | Arrondi EUR à 2 décimales, moitié au pair | 1,005→1,00 ; 1,015→1,02 ; −1,005→−1,00 ; afficher écarts de ventilation si nécessaire |
| M18 | TND à 3 décimales, valeur 10,001 TND ; poids/cours à grande précision | Aucune perte dans le stockage ni l’export ; arrondi EUR uniquement à la sortie concernée |
| M19 | Garantie 50 000 EUR sans appel | Engagement visible hors dettes certaines ; appel ultérieur documenté séparément |
| M20 | Personne quitte le périmètre le 01/07 | Incluse le 30/06, exclue le 01/07 ; aucune suppression de ses données ; variation de périmètre identifiée |
| M21 | Trois membres possèdent chacun une part sur trois d’un bien à 1 EUR | Droits exacts 1/3, somme 1 ; total familial 1,00 EUR ; ventilation affichée déterministe 0,34/0,33/0,33, ajustement de présentation signalé |

### P0 — intégrité, historique et récupération

| ID | Scénario | Résultat attendu |
|---|---|---|
| D01 | Décimal non canonique, monnaie inexistante, FK orpheline, pourcentage négatif | Refus explicite avant commit ; état et audit inchangés |
| D02 | Acquérir puis céder un actif ; corriger rétroactivement sa valeur | Actif conservé, absence après cession ; passé corrigé et passé tel que connu tous deux reproductibles |
| D03 | Révision concurrente obsolète ou double validation d’une commande | Conflit/idempotence contrôlé ; pas de transaction ni paiement dupliqué |
| D04 | Paiement et écriture de banque puis panne entre leurs écritures | Transaction entière présente ou absente ; aucun demi-paiement |
| D05 | Sauvegarder une base en WAL avec écritures documentaires concurrentes | Snapshot DB + blobs cohérents ; écritures hors snapshot exclues proprement |
| D06 | Restaurer sauvegarde SQLite riche dans une génération vierge | Même ensemble d’UUID, valeurs décimales, dates, FK, versions, liens et empreintes ; mêmes indicateurs recalculés |
| D07 | Export CSV puis import dans une autre base vierge | Même comparaison métier que D06, sans exiger l’identité binaire des fichiers SQLite |
| D08 | Document partagé par deux objets ; désassocier un lien | Un seul blob, second lien conservé ; sauvegarde complète contient original |
| D09 | Archive tronquée, hash faux, fichier manquant, version future, ID dupliqué ou FK absente | Rejet avant activation ; base en service inchangée |
| D10 | ZIP avec `../`, chemin absolu, lien symbolique, doublon normalisé ou bombe compressée | Rejet contrôlé, aucune écriture hors staging, quota mémoire/disque respecté |
| D11 | Panne avant/après remplacement du pointeur de génération et aux points fsync | Au redémarrage, ancienne ou nouvelle génération valide ; aucun mélange DB/documents |
| D12 | Migration échoue ou disque se remplit | État précédent récupérable, sauvegarde pré-migration vérifiée, pas de réinitialisation silencieuse |
| D13 | Export de texte `=HYPERLINK(...)`, `+…`, `@…`, chiffres >15 chiffres, NULL et chaîne vide | CSV exact et réimportable ; XLSX texte littéral et montants exacts textuels ; NULL distinct du vide |
| D14 | Ouvrir le ZIP, la DB et les documents sans application | `PRAGMA integrity_check` = `ok`, `foreign_key_check` vide, consultation SQL/CSV et empreintes reproductibles avec outils standards |

La fixture riche D06/D07 contient trois membres, un tiers, deux sociétés imbriquées et chemins multiples, un bien commun, un bail avec échéance réglée partiellement, dépenses, comptes EUR/TND/CHF/AED, une caisse, une dette externe, un prêt interne, un lot d’or/argent, des cours et taux datés, des changements de propriété/périmètre et au moins un document lié à plusieurs objets. Inclure des corrections rétroactives conservées et une valeur inconnue : l’état d’incomplétude doit survivre aussi. Tester aussi une base ne contenant aucun document ; en revanche, retirer un original référencé d’une archive CSV doit faire échouer son import. La V1 ne prévoit pas de variante CSV intégrale privée de ses pièces référencées.

### P1 — modules opérationnels

| ID | Scénario | Résultat attendu |
|---|---|---|
| O01 | Échéance loyer 1 000, deux paiements 400 puis 300 ; dont paiement tardif | Encaissement 700 ; restant 300 ; retard et date de règlement traçables ; aucune double entrée banque |
| O02 | Un paiement 1 500 affecté à deux échéances 1 000 et 500 | Paiement total 1 500 ; pas de duplication ; trop-affectation refusée |
| O03 | Dépôt de garantie 2 000 | Trésorerie reçue +2 000 et dette de restitution +2 000 ; revenu locatif nul |
| O04 | Loyer mensuel 1 000 ; exploitation 2 000/an ; valeur bien 200 000 ; financement annuel 4 000 | Contractuel annualisé 12 000 ; si même montant exigible/encaissé : NOI 10 000 ; rendement brut 6 %, net avant financement 5 % ; cash-flow 6 000 |
| O05 | Bail terminé, bien vacant, nouvel occupant puis vente | Aucun loyer fictif durant vacance ; historique maintenu ; revenu attribuable selon droits aux dates des flux |
| O06 | Banque ouverture 100, entrée 30, sortie 20, transfert interne 50 | Solde source 60 ; destination +50 ; transfert familial net nul et aucun revenu |
| O07 | Conversion : débit 100 EUR, crédit 330 TND, frais séparés 2 EUR | Débit total 102 EUR, crédit 330 TND ; taux exécuté 3,3 TND/EUR sur principal ; frais de 2 distincts, sans déduction double |
| O08 | Rapprochement calculé 110, relevé 100 | Écart −10 signalé ; aucune correction implicite, mouvement justifié si l’utilisateur corrige |
| O09 | Principal initial 100 000, remboursement principal 1 000 + intérêts 200 + frais 10 | Capital restant 99 000 ; débit trésorerie 1 210 ; 210 de charges, pas 1 210 retranchés du principal |
| O10 | 2 pièces de 31,1034768 g à pureté 0,9 ; cours 60 EUR/g fin | Poids fin 55,98625824 g ; intrinsèque 3 359,1754944 EUR → 3 359,18 à l’affichage ; prime séparée |
| O11 | Vendre la moitié du lot O10 | Quantité et poids fin divisés par deux ; produit de vente lié à trésorerie une seule fois ; prix de revient alloué explicitement au lot vendu |
| O12 | 1 once troy / 1 kg | 31,1034768 g / 1 000 g exactement, pas once avoirdupois |
| O13 | Dividende reçu d’une société familiale | Trésorerie des deux entités reliée ; pas de bénéfice supposé reçu avant distribution, pas de gain familial à hauteur interne |
| O14 | Don, cession de parts et transfert de métal entre deux membres | Recomposition individuelle, pas de gain familial inventé ; frais/cession à tiers séparés |
| O15 | Modifier montant local sans taux ni données au début d’une période | Variation/performance non calculable ou partielle ; pas de courbe complète inventée |

### P2 — interface, distribution et performances

- Contexte membre/date transmis à toutes les vues, erreurs et exports de rapports. Export intégral explicitement non filtré.
- Navigation française, clavier, confirmations destructrices, formulaires avec unités et devises, états vides avec action réelle.
- Requêtes lentes/export avec progression et annulation sûre ; aucun bouton factice ni graphique de démonstration en base réelle.
- AppImage lancée sur une machine Linux de test sans outils de développement ; redémarrage hors ligne ; documents dans XDG conservés après remplacement du paquet.
- Mise à jour d’un ancien schéma avec pré-sauvegarde, retour à l’ancienne génération en cas d’erreur, blocage d’un ancien binaire sur schéma trop récent.
- Mesurer sur matériel déclaré : 100 personnes, 100 sociétés en DAG, 10 000 actifs, 100 000 mouvements, 10 000 documents ; relever temps et mémoire avant de fixer un engagement. Cible provisoire : consultation courante <2 s et interface réactive pendant export. Limiter les archives par taille décompressée, nombre et taille de fichiers ; limites visibles et documentées, non destructrices.

## 3. Risques et réponses

| Risque | Prévention / détection | Décision requise et moment |
|---|---|---|
| Double compte sociétés/sous-jacents ou trésorerie/indicateurs | Modes détaillé/opaque, couverture et lignes sources | Qualification des valorisations avant leur usage |
| Droits économiques atypiques, démembrement, cycles | Aucun calcul approximatif ; état non pris en charge ; rejet cycles temporels | Formes réellement nécessaires avant phase 3 |
| Dette personnelle assimilée à quote-part d’actif | Débiteur et créancier explicites, garanties séparées | Qualification de chaque financement à la saisie |
| Société détaillée déficitaire à responsabilité limitée | Exposition analytique distincte du patrimoine opposable ; total incomplet sans règle explicite | Responsabilité/engagements ou valorisation opaque documentés avant total complet |
| Prêts à travers une société opaque | Contrôle de couverture/réconciliation, pas d’élimination spéculative | Documenter inclusion dans sa valeur de capitaux propres |
| Pourcentages, évaluations et cours absents ou anciens | Propagation d’anomalies ; sous-total valorisable affiché comme tel | Seuils d’ancienneté paramétrables sans masquer les dates |
| Données historiques insuffisantes | Intervalle connu de suivi et absence explicite ; aucun solde rétro-inventé | Importer les pièces manquantes ou accepter périmètre partiel |
| Correction rétroactive et rapports incohérents | Révisions conservées, temps de connaissance et version moteur | Définir le contexte de chaque rapport |
| Explosion du nombre de chemins / nombres immenses | Propagation topologique sans énumération exhaustive ; bornes de saisie et coût | Profilage phase 3, limites déclarées |
| Corruption, disque plein, rupture d’alimentation | Transactions, sauvegarde API, staging/fsync/activation génération | Tests de panne phases 2 et 8 |
| ZIP hostile ou document à contenu actif | Quotas, chemins stricts, pas de rendu actif embarqué ; ouverture externe explicite | Politique d’import et avertissement local |
| Fuite de données ou vol du poste | Aucune télémétrie ; permissions privées ; logs minimisés ; OS chiffré si choisi | Support de sauvegarde sécurisé par l’utilisateur |
| CSV exact interprété comme formule par tableur | Archive brute de récupération ; XLSX texte pour consultation ; guide import texte | Ne pas modifier les données de réimport pour les « nettoyer » |
| Dépendances WebKit/AppImage ou pilote E2E | Choisir distribution minimale ; test desktop réel et build reproductible | Cible Linux à fixer avant livraison |
| Illusion de produit terminé | Statut par module, gates de validation, pas de graphiques avant moteur | Revue des critères de sortie à chaque phase |

## 4. Plan d’implémentation en tâches exécutables

Chaque tâche doit produire un changement relisible, ses migrations éventuelles et des preuves de validation. Les numéros ci-dessous sont des unités de travail, pas des promesses d’exécution dans ce lot. Ne pas ouvrir la phase 9 avant validation des phases métier concernées.

| Tâche | Dépend de | Travail et livrable | Critère de sortie |
|---|---|---|---|
| P2.1 | Phase 1 | Choisir distribution, épingler Rust/Node ; initialiser Tauri 2, React TS et workspace crates | `cargo test`, typecheck, build frontend et lancement Linux réussis |
| P2.2 | P2.1 | Installer couche SQLite et migrations de base, registre schéma/checksums, FK/WAL, verrou mono-instance, chemins XDG | Créer/rouvrir DB ; migration idempotente et échec sans perte |
| P2.3 | P2.2 | Documents minimaux et première sauvegarde DB+blobs ; inspection de restauration, prébackup et activation de génération | Cycle sauvegarde/restauration et panne contrôlée D05/D09/D11 |
| P2.4 | P2.1–3 | Coque française, navigation, paramètres et section sauvegarde | Actions réelles et écrans non implémentés signalés ; démarrage hors ligne |
| P3.1 | P2.2 | Personnes, tiers, membres, périmètre daté et sociétés | CRUD historisé, versions et FK testées |
| P3.2 | P3.1 | Droits juridiques/éco/votes ; participations/propriétés à intervalles | Cycles, chevauchements et dépassements refusés, inconnus conservés |
| P3.3 | P3.2 | Cœur rationnel et graphe topologique ; lignes d’attribution et modes détaillé/opaque | M01–M07, M10–M12 et invariants réussis avec évaluations EUR synthétiques |
| P3.4 | P3.3 | Définir positions créance/dette et éliminations dans domaine pur | M08/M09/M19 ; module de saisie des dettes encore prévu P6.3 |
| P4.1 | P3.3 | Référentiels monnaies, taux manuels, conventions et chemins FX | M13–M18 ; aucun taux futur ou par défaut |
| P4.2 | P4.1 | Reconstruction économique/temps de connaissance, corrections et audit | M20/D02 ; moteur des périodes sans données partielles cachées |
| P4.3 | P4.2 | Calcul de variation expliqué flux/valorisation/change/périmètre | Pont réconcilié, interaction attribuée selon ordre documenté, pas de performance inventée |
| P5.1 | P3.2/P4 | Biens, unités, acquisition/cession et valorisations datées | Coûts et valeurs distincts, conservation après vente |
| P5.2 | P5.1 | Locataires, baux, conditions/révisions et échéances idempotentes | O05 ; contrats successifs, génération non dupliquée et vacance corrects |
| P5.3 | P5.2 | Paiements/affectations, dépenses, dépôts et projections des loyers | O01/O02/O04 ; O03 testé dans le domaine pur ; intégration trésorerie et obligation de dépôt finalisée en P6.1/P6.3 |
| P6.1 | P4/P5.3 | Comptes/caisses, soldes d’ouverture, écritures, transferts et rapprochements | O06/O08 ; transactions atomiques et paiement source unique |
| P6.2 | P6.1 | Conversions exécutées avec frais et transferts patrimoniaux | O07/O14 ; montants débité/crédité sourcés, flux internes isolés |
| P6.3 | P3.4/P6.1 | Contrats de financement, échéances, remboursements et garanties ; dépôts locatifs | M07–M09/M19 et O03/O09 ; capital et intérêts séparés |
| P7.1 | P4/P6.1 | Lots métaux, unités exactes, cotations et mouvements partiels | O10–O12 ; stock et prix de revient réconciliés |
| P7.2 | P3/P6 | Indicateurs sociétés, périodes, modes datés et dividendes | M10/M11/O13 ; aucun indicateur compté comme position supplémentaire |
| P8.1 | P2.3/P5–7 | Gestionnaire documentaire complet, liens multiples, import et export d’originaux | D08/D10 et permissions, aucune dépendance à chemin source |
| P8.2 | P8.1 | CSV versionné de toutes tables, dictionnaire et réimport vierge | D07/D09/D13 : comparaison canonique, FK et calculs |
| P8.3 | P8.2 | XLSX complet, feuilles/relations/unités et contrôle capacité | Ouverture LibreOffice ; textes/precision conservés ; D13 |
| P8.4 | P8.1–3 | Étendre sauvegarde à tous modules ; restauration complète et recovery CLI/documenté | D05–D14 avec fixture riche, deux voies round-trip indépendantes |
| P9.1 | P3–8 validées | Vues famille/membre/société, alertes et recherche | Mêmes sources et contexte ; totaux réconciliés, incomplets visibles |
| P9.2 | P9.1 | Graphiques historiques et répartitions, rapports filtrés | Aucun chiffre de démonstration ; pont de variation et couverture affichés |
| P10.1 | P9 | Revue intégrité, sécurité, performances et accessibilité ; guides utilisateur/récupération | Suites complètes, hors ligne et inspection autonome réussies |
| P10.2 | P10.1 | Construire AppImage, tester installation/mise à jour, .deb si retenu | Démarrage machine propre ; données préservées et version précédente récupérable |

### Migrations à préparer progressivement

Le [plan de migrations du modèle relationnel](../technique/modele-relationnel.md#plan-de-migrations-et-reprise) est la référence unique pour la numérotation prévisionnelle. Le catalogue cible ne demande pas de créer prématurément toutes les tables. Le socle historique commun existe dès la première table métier. Une dépendance FK nécessaire est créée dans la même migration ou avant ; les tables de paiement de P5 acceptent un lien de trésorerie facultatif jusqu’au module P6, jamais une fausse écriture bancaire.

## 5. Questions réellement bloquantes

**Aucune pour terminer la phase 1 ou amorcer le socle.** Les conventions proposées sont documentées et permettent les tests.

Les points à confirmer avant d’utiliser les fonctions concernées sont : droits juridiques atypiques effectivement détenus ; responsabilité des associés et sociétés déficitaires ; définition/périmètre des valorisations globales ; traitement des financements traversant une société opaque. Tant que la qualification manque, la position reste explicitement non calculable. La distribution Linux minimale et le matériel cible bloquent la qualification du paquet final, pas le moteur métier.
