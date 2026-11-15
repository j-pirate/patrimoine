# Validation — phase 5 : immobilier et loyers

Cette campagne porte sur les fiches immobilières, unités techniques, acquisitions/cessions, baux, conditions datées, échéances, paiements, affectations et dépenses. Toutes les données utilisées sont synthétiques ; les coffres de test sont isolés. Les résultats ne seront attribués qu’aux commandes effectivement exécutées.

## Critères chiffrés

| Scénario | Résultat attendu |
|---|---|
| Acquisition 100 000 et frais 10 000 ; estimation 150 000 | Coût 110 000, gain latent 40 000 ; aucune estimation déduite du coût si elle manque |
| Coût 100 000 TND converti à 0,30 ; estimation 120 000 TND au cours 0,40 | Gain natif 20 000 TND ; différence EUR 48 000 − 30 000 = 18 000 EUR |
| Un bien et trois unités | Une seule valeur consolidée ; unités techniques sans deuxième actif |
| Cession au 1er juillet | Clôture atomique des droits à cette date, anciennes fiches/estimations conservées, absence de stock courant |
| Revenus indirects à travers une société globale | Droits économiques exacts pour l’analyse des revenus ; aucune distribution de trésorerie implicite |
| Baux successifs avec vacance | Aucune échéance hors contrat, fins exclusives, occupation et recouvrement distincts |
| Génération répétée d’échéances | Identités stables, pas de doublon ni mutation inutile ; désaccord sur une ancienne échéance signalé |
| Échéance 1 000, paiements 400 et 300 | Reçu 700, restant 300, limites des allocations exactes |
| Un paiement 1 500 sur deux échéances 1 000 et 500 | Paiement compté une seule fois, trop-affectation refusée sans écriture partielle |
| Échéance du 5 mars, réception 600 en mars puis 400 le 20 avril | À fin mars : restant 400 ; à fin avril : zéro, avec retard historique conservé |
| Loyer mensuel 1 000 ; exploitation annuelle 2 000 ; valeur 200 000 | Annualisé 12 000, NOI 10 000, brut 6 %, net 5 %, selon les bases explicitement saisies |
| Dépôt reçu 2 000, contrat pur | Revenu nul, trésorerie +2 000 et obligation +2 000 ; intégration des comptes en phase 6 |
| Sauvegarde et restauration | Même registre, K, baux, sources de termes, allocations, originaux et calculs avant/après |

## Campagnes

Les tests du domaine vérifient les invariants et les calculs exacts ; ceux du stockage vérifient les transactions SQLite, conflits, historique et migrations ; ceux de l’interface contrôlent les formulaires et la lecture seule à K. Le parcours desktop vérifie l’intégration Tauri/WebKitGTK et les dialogues natifs, dans un conteneur sans réseau.

## Vérifications exécutées

Le 5 novembre 2026, `source scripts/cloud-env.sh` puis `CARGO_BUILD_JOBS=3 bash scripts/check.sh` : **réussi, code de sortie 0**.

| Campagne | Résultat observé |
|---|---|
| TypeScript et Vite | Vérification des types et build réussis |
| Vitest | **63 tests réussis**, 7 fichiers, aucun ignoré : les 40 tests précédents, 9 immobilier et 14 locations |
| Domaine Rust | **87 tests réussis** : 5 exactitude, 19 attribution, 11 FX, 5 revenus économiques, 4 profils, 15 immobilier, 21 locations, 7 variation |
| Infrastructure Rust | **103 tests réussis** : 18 FX/histoire, 9 immobilier, 29 récupération adverse, 17 registre, 10 migrations, 16 locations, 4 coffre |
| Workspace Rust | **190 tests réussis**, aucun échec ni test ignoré ; les autres cibles et doctests n’ajoutent pas de cas |
| rustfmt et Clippy | Formatage conforme ; toutes cibles et fonctionnalités, `-D warnings`, réussis |
| Desktop Tauri | Binaire Linux avec interface embarquée `custom-protocol` construit avec succès |

Les tests purs couvrent les coûts et estimations indépendants, FX aux dates respectives, absence d’hypothèse sur les frais inconnus, cession/ancienne connaissance, unités sans double valorisation, prorata calendaire et bissextile, paiement N↔N, remboursement et capacité nette d’avance à chaque date, termes/échéances manquants, dépôts sans revenu, droits courus et destinataire cash distincts, société propriétaire et pays daté, sources et cotations FX, occupation dans le périmètre et annualisation sur les seuls jours observés.

Les tests SQLite vérifient les transactions et refus sans trace, la génération en un seul changement et sa répétition sans mutation, les révisions obsolètes, clés conservées après annulation, références de conditions exactes et non futures, immobilité des cibles, correction d’un remboursement avec remplacement atomique de ses affectations et correction d’une cession avec réparation des droits. L’ouverture et la restauration d’un vrai schéma 3 sont testées vers le schéma 4 ; les archives/générations sources restent inchangées et aucune fiche immobilière n’est inventée. Les régressions des schémas 1/2 sont conservées.

Les tests UI vérifient les entrées décimales en texte, dépendances entre formulaires, contrôles de confirmation, conflits qui conservent les saisies, absence d’anciens snapshots après échec d’un chargement K, lecture seule à K=0, périmètre familial conservé après filtrage d’un bien, choix explicite du bien entier et remboursement créé/corrigé avec ses reprises. Ils utilisent des réponses IPC contrôlées ; le parcours WebKitGTK constitue la campagne d’intégration séparée.

## Corrections issues de revue

La revue a confirmé les cas suivants, corrigés puis vérifiés sans désactiver d’assertion :

- Une cession devenait irréparable si seule sa création était autorisée. Sa correction retrouve désormais les droits liés, contrôle qu’ils n’ont pas été modifiés de manière incompatible et les répare dans le même changement.
- Un remboursement isolé d’un montant déjà affecté pouvait laisser une créance présentée réglée alors que le cash était rendu. Les contraintes de capacité nette et la commande atomique créent/corrigent paiement et reprises ensemble.
- Un filtre de bien faisait passer silencieusement de la part familiale à la totalité du bien. La totalité exige désormais un choix explicite.
- L’occupation ignorait le périmètre individuel. Une personne sans aucun droit pouvait recevoir un taux d’occupation de 100 %. Les jours-unités suivent maintenant les segments de droits/appartenance et les sorties de suivi ; les droits indéterminés laissent le pourcentage inconnu.
- Le rendement net divisait un NOI tronqué à D par une durée comprenant des jours futurs. Avec 1 000 EUR observés au 31 janvier et valeur 200 000 EUR, une fenêtre annuelle donnait 0,50 % au lieu de 5,89 %. Fenêtre annuelle et janvier donnent désormais toutes deux 5,89 % au même D ; aucune durée observée laisse l’indicateur non calculable.

Un premier lancement de `scripts/check.sh` s’est arrêté sur le formatage des nouveaux fichiers du pont Tauri et de tests ; après `cargo fmt --all`, la commande complète a été relancée et a réussi.

## Parcours graphique Linux réel

`bash scripts/test-desktop-cloud.sh` : **réussi, sortie 0**, sur le binaire final. Le [rapport JSON](../../.local/desktop-smoke.tXhT17zX/desktop-result.json) contient **31 étapes réussies**, aucune échouée ni non exécutée, et `native_dialogs_requested: true`. L’application s’exécute sous WebKitGTK/Xvfb dans Debian 13, avec utilisateur non privilégié, réseau désactivé et coffre synthétique neuf. Les contrôles de phases 3/4 sont conservés avant le nouveau scénario immobilier.

Le parcours vérifie notamment :

- Fiche immobilière créée depuis l’interface, rattachée à l’actif existant, surface française `125,50` stockée `125.5`.
- Trois unités sans deuxième valeur patrimoniale ; coût `310 000 EUR`, estimation `400 000 EUR`, gain latent `90 000 EUR`, dette inconnue.
- Douze échéances mensuelles de 2023, puis génération répétée sans nouvelle échéance ni changement de séquence.
- Paiements de `400` et `300` en mars : `700 EUR` reçus, reste `300 EUR` sur l’échéance de mars. Un complément de `300` le 20 avril solde cette échéance en conservant son retard et sa date de règlement.
- Sur l’année 2023 : exigible `12 000 EUR`, reçu `1 000 EUR`, exploitation `2 000 EUR`, NOI `10 000 EUR`, brut `3 %`, net `2,5 %` sur une valeur de `400 000 EUR`. Le cash-flow après financement reste absent. Occupation contractuelle : `365 / 1 095` jours-unités.
- Vente complète au 1er juillet 2024, puis correction depuis l’interface vers le 1er août et prix `161 000 EUR`, avec confirmation : droits réparés, ancien état toujours consultable à son K. Le bien cédé est absent du stock courant et aucun produit de vente bancaire n’est inventé.
- Indicateurs calculés depuis leur formulaire réel, avec le choix explicite du bien entier ; [capture de l’interface](../../.local/desktop-smoke.tXhT17zX/rental-indicators.png).
- Dialogues GTK d’import/export documentaire, sauvegarde, analyse et annulation sans mutation, confirmation avec sauvegarde préalable, puis restitution des identités, droits, taux, profils, trois snapshots et indicateurs locatifs après activation d’une nouvelle génération.

Le premier essai du nouveau pilote s’est arrêté après création de la fiche : le nom d’argument Python `kind` entrait en collision avec le champ métier de l’événement. Le paramètre a été renommé, la syntaxe contrôlée, puis le parcours complet a réussi sans modification des assertions ni des calculs applicatifs. Les saisies de texte utilisent le clavier WebDriver ; les dates natives utilisent les événements DOM `input/change`, avec vérification après deux images. Aucun état React ni résultat financier n’est injecté pour faire passer le scénario.

## Inspection indépendante des sauvegardes

Une lecture des deux ZIP avec les bibliothèques standard Python `zipfile`, `hashlib`, `sqlite3`, `fractions` et `datetime`, sans appeler le backend applicatif, a produit [ce résultat indépendant](../../.local/desktop-smoke.tXhT17zX/independent-backup-inspection.json).

Les deux archives ont le schéma 4 ; la principale est à K=59, la préalable à K=60. Les quatre fichiers manifestés par archive ont leurs tailles et empreintes exactes. Les comptages de toutes les tables concordent avec le manifeste, `integrity_check = ok` et aucune erreur de clés étrangères n’est détectée. Montants, taux, frais et surfaces sont stockés en `text` lorsqu’ils sont renseignés ; aucune somme en flottants SQLite n’a été utilisée.

La relecture des faits courants reconstitue indépendamment `12 000` de dû, `1 000` de paiements, `2 000` d’exploitation, donc `10 000` de NOI, ainsi que les rendements `3 %` et `5/2 %`, et l’impayé de mars `1 000 − 700 = 300`. Elle retrouve les trois unités et leur unique bail annuel, l’ancienne cession au 1er juillet, la correction au 1er août et le droit clôturé au 1er août.

Les empreintes des scripts `0001` à `0003` correspondent aussi à celles enregistrées dans la sauvegarde synthétique de phase 4 présente dans l’environnement publié : ces migrations antérieures n’ont pas été réécrites. Les quatre scripts correspondent au registre de migrations des nouvelles archives. Le dictionnaire autonome décrit bien les biens et locations.

## Environnement restauré

Les outils restaurés ont été vérifiés : Node 24.22.1, npm 11.9.0, Rust 1.99.1, GTK 3.24.49 et WebKitGTK 2.54.0. Le moteur Docker local et le pilote Tauri étaient disponibles. Les fichiers, outils et commandes existants ont été utilisés ; aucun script d’installation n’a été réappliqué, aucun brouillon cloud modifié et aucune publication effectuée pendant ce lot.

## Limites du lot

La phase 6 apporte les comptes, écritures de banque/caisse et financements persistés. Un paiement locatif non rapproché reste une source explicitement déclarée, sans écriture bancaire fabriquée. Un revenu attribuable à travers une société ne constitue pas une distribution reçue par un associé. Les dépôts ne sont pas ajoutés aux loyers ; leur obligation de restitution et leur intégration patrimoniale complète relèvent de la phase 6. Aucun rendement global, bilan net exhaustif ni paquet AppImage n’est qualifié par cette campagne.

Les impayés sont ici un calcul analytique proportionnel aux droits historiques de la période courue. Les affectations n’identifient pas encore un créancier juridique distinct du bénéficiaire cash ; une répartition préférentielle ou cession individuelle de créance n’est pas simulée. Ces impayés ne sont pas encore ajoutés au total global des actifs, qui conserve son avertissement de couverture partielle jusqu’à l’intégration des comptes et passifs en phase 6.
