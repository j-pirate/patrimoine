# Avancement du projet

Mis à jour le 13 novembre 2026, Europe/Paris.

## Conception livrée — phase 1

- Phase 1 : analyse du besoin, architecture et frontières techniques.
- Organisation cible du dépôt et registre des décisions.
- Modèle relationnel détaillé proposé : tables, contraintes, index et historique.
- Algorithmes et règles proposés : propriété, consolidation, change, calculs et qualité.
- Conception des documents, sauvegardes, exports, imports et restauration.
- Scénarios chiffrés, risques, critères d’acceptation et tâches des phases 2 à 10.
- Cahier des charges version 3.0 conservé dans le dépôt.

Pour cette phase de conception, « livré » signifie **documenté et revu**, pas exécuté en production. Les phases implémentées sont détaillées ci-dessous. Les valeurs chiffrées des documents sont des exemples de tests synthétiques, pas des données financières utilisateur.

## Socle implémenté — phase 2

- Workspace Rust : contrats de domaine, service applicatif et adaptateurs indépendants de Tauri.
- Application Linux Tauri 2 et interface React/TypeScript entièrement en français.
- Navigation des treize sections ; modules futurs signalés explicitement, sans montants ni graphiques fictifs.
- Paramètres persistants et contrôlés : nom de l’espace, devise préférée, consolidation EUR fixe.
- SQLite standard, migration `0001`, vérification du schéma et checksum, clés étrangères, WAL, synchronisation durable et verrou d’instance.
- Coffre privé hors installation, générations et pointeur `CURRENT` contrôlé.
- Import de documents originaux, empreintes SHA-256, déduplication et export d’une copie.
- Sauvegarde ZIP avec SQLite Online Backup API, originaux et manifeste contrôlé.
- Analyse séparée de restauration, résumé, confirmation et proposition de sauvegarde préalable ; activation en nouvelle génération et conservation de la précédente.
- Contrôles des archives, schémas futurs, pièces manquantes, conflits après analyse et états de reprise.
- Scripts reproductibles, dépendances verrouillées et workflow CI Linux.

À la sortie du socle, les associations métier, le classement documentaire et les exports CSV/XLSX complets restaient à développer ; la phase 8 les complète. Le format ZIP reste en version 1, au schéma courant 6, avec mise à niveau sur copie des anciennes archives SQLite des schémas 1 à 5.

## Registre implémenté — phase 3

- Personnes, tiers et sociétés : création, correction motivée, archivage et réactivation de profils.
- Périmètre familial par intervalles datés, fin exclusive et périodes successives.
- Participations et propriétés datées, avec droits juridiques, économiques et de vote distincts.
- Fractions exactes, données inconnues conservées, normalisation des fractions et montants à la saisie.
- Refus atomique des cycles, chevauchements et sommes supérieures à 100 % à toutes les dates affectées.
- Actifs génériques, estimations datées en EUR et méthodes détaillée/globale des sociétés.
- Moteur Rust d’attribution familiale/individuelle : détentions directes/indirectes, chemins multiples et absence de double comptabilisation.
- Contexte global membre/date et contrôle des contributions avec sources, dates et alertes ; aucune courbe fictive ni affirmation de patrimoine net exhaustif.
- Révisions et audit conservés ; conflits de révision refusés ; reconstitution backend par borne de connaissance.
- Migration `0002` sur nouvelle génération ; anciennes bases et sauvegardes du schéma 1 conservées et migrables.
- Sauvegardes du registre avec ses anciennes versions ; résultats d’attribution reproduits après restauration.
- Contrat du domaine pur pour prêts/garanties et éliminations proportionnelles testé ; la phase 3 ne comportait pas encore leur saisie ni leur persistance, ajoutées en phase 6.

La phase 4 étend ce registre aux estimations multidevises et à la datation civile des profils. Voir le [guide du registre](../utilisateur/famille-et-propriete.md) et son [complément devises et historique](../utilisateur/devises-et-historique.md).

## Devises et historique implémentés — phase 4

- Taux manuels EUR/TND/CHF/USD/AED : saisie française, corrections motivées, annulation et conservation des versions.
- Conversions exactes directe, inverse et croisée ; priorité déterministe, aucun cours futur ni parité supposée.
- Traces des taux, sens, dates, sources et révisions ; alertes de cours ancien ou contradictoire ; montant inconnu distinct de zéro.
- Estimations multidevises et conversion des stocks au cours applicable à la date du rapport, avec montant d’origine conservé.
- Deux axes indépendants D/K : date économique et séquence de connaissance globale, consultation ancienne en lecture seule et journal des corrections.
- Périodes descriptives des personnes, sociétés et actifs, référant une révision exacte ; absence de pays historique supposé en cas de période manquante.
- Pont exact du sous-total d’actifs : périmètre/méthodes, droits, estimations, change ; références manquantes et résidu explicites.
- Migration `0003` sur copie, coffres et archives des schémas 1/2 migrables sans écraser leur source ; sauvegarde incluant tous les nouveaux historiques.

Ce pont ne calcule pas de rendement et ne déduit aucun flux ni coût d’acquisition. Dans le périmètre de la phase 4, caisses, dettes persistées, loyers et bilan financier restaient absents ; ils ont été ajoutés dans les phases 5 et 6. Le pont reste un contrôle du sous-total d’actifs. Les tableaux de bord n’étaient pas encore livrés à cette sortie ; leur mise en œuvre de phase 9 est décrite ci-dessous.

## Immobilier et loyers implémentés — phase 5

- Fiches immobilières et unités techniques rattachées aux actifs existants, sans double valorisation ; surfaces et périodes contrôlées.
- Acquisition/ouverture du suivi distincte de l’estimation ; coût historique, effet du change et gain latent exacts, données absentes conservées inconnues.
- Cession complète et corrections de date atomiques avec clôture/réouverture contrôlée des droits ; historique et anciennes connaissances conservés.
- Baux successifs et conditions datées, fréquences mensuelle/trimestrielle/annuelle, prorata civil et génération d’échéances idempotente.
- Paiements et affectations plusieurs-à-plusieurs, avances explicites, remboursements et corrections atomiques ; limites de montants et devises contrôlées à chaque date affectée.
- Dépenses d’exploitation séparées des travaux en capital, intérêts, principal et frais de financement.
- Indicateurs sourcés : exigible, reçu, impayé/retards historiques, revenu couru, NOI, rendements avant financement et occupation contractuelle ; filtres bien, pays daté et société.
- Attribution exacte des revenus pendant la période courue, y compris à travers des sociétés ; sommes reçues attribuées au bénéficiaire déclaré, sans distribution fictive.
- Lecture historique D/K des trois ensembles registre/immobilier/locations ; anciennes séquences non modifiables.
- Migration `0004` sur copie et sauvegarde/restauration des conditions historiques, clés d’échéance, paiements et révisions.

À la sortie de la phase 5, les impayés étaient un calcul analytique proportionnel aux droits historiques, sans créancier explicite ni intégration au bilan global. Comptes, rapprochements, financements, dépôts effectivement reçus et cash-flow après financement restaient hors de cette phase. La phase 6 ci-dessous complète ces limites ; le revenu couru analytique reste distinct du droit porté sur une créance. Les dépôts ne sont jamais des loyers.

## Comptes et financements implémentés — phase 6

- Comptes bancaires et caisses rattachés aux actifs du registre, titulaire juridique distinct des droits économiques ; ouverture unique explicitement confirmée, y compris à zéro.
- Journal exact, transferts entre comptes et conversions exécutées à deux jambes ; frais séparés du principal et taux réellement exécuté traçable.
- Soldes à D/K, découvert présenté en passif, caisse connue négative refusée ; ouverture ou cours manquant conservé inconnu.
- Relevés avec référence/document, comparaison au journal et écarts explicites, sans correction automatique du solde.
- Liens vers les révisions exactes de la source, de l’opération et de sa ligne ; choix d’une ligne existante au lieu d’un second encaissement/décaissement.
- Saisies atomiques des paiements locatifs, affectations, dépenses payées, dépôts et mouvements de financement avec leur trésorerie ; conflits de séquence ou de révision refusés sans écriture partielle.
- Dépôts reçus/restitués distincts des revenus et charges ; application explicite à une échéance sans flux bancaire ; avances non affectées visibles comme obligations.
- Contrat de prêt unique à deux faces, parties et répartitions économiques propres, indépendantes des copropriétaires du bien ; principal, intérêts et frais séparés.
- Échéances prévisionnelles et affectations de remboursements réels, dépréciations de créance distinctes d’une remise juridique de dette, archivage sans extinction du principal.
- Liens analytiques aux biens et collatéraux distincts des emprunteurs ; garanties conditionnelles hors dettes certaines ; couverture explicite des créances/dettes déjà comprises dans une valeur globale de société.
- Créanciers économiques explicites par échéance locative, périodes et fractions connues ou inconnues ; cession de créance non déduite d’une vente du bien ; dépréciations et représentations par contrat documentées.
- Bilan financier familial/individuel distinct du contrôle historique des attributions : actifs, banques, créances, passifs et éliminations réciproques avec sources et cours.
- Déclarations de couverture de l’espace familial requises pour un net complet ; tables vides et sources manquantes ne deviennent pas des montants nuls. Sous-totaux connus distincts du patrimoine net complet.
- Compléments aux contrôles immobilier/locatif : dette affectée, net avant frais/fiscalité de cession et cash-flow après financement seulement lorsque couverture et liens suffisent.
- Migration `0005` vers le schéma 5 sur copie, sans ouverture ni mouvement bancaire inventé pour les anciennes sources ; sauvegardes incluant les nouveaux faits et toutes leurs révisions.

À la sortie de la phase 6, le registre et son pont de variation conservaient leur périmètre historique d’actifs ; le nouveau bilan financier traitait les passifs et créances. Une déclaration de couverture ne remplace aucune donnée manquante : dettes, droits et flux nécessaires doivent être réellement connus pour le périmètre et la date consultés. La phase 7 ci-dessous ajoute les lots et indicateurs ; les exports bureautiques complets restaient à cette sortie la phase 8, et les tableaux de bord la phase 9.

## Métaux et sociétés implémentés — phase 7

- Lots physiques d’or/argent rattachés aux actifs existants, poids et pureté fixes, unités g/kg/once troy exactes ; titulaire initial distinct des droits courants.
- Ouverture explicite du stock, achats, ventes partielles, corrections d’inventaire et déplacements du lot entier ; quantité historique non négative et prix de revient proportionnel par lot homogène.
- Cours du métal fin et primes/décotes datés ; intrinsèque, estimation de revente, coût, gains réalisés/latents et source retenue au bilan distincts. Absence de prime différente d’un zéro confirmé.
- Transfert atomique des droits juridiques/économiques datés, sans changer le stock du lot ni créer un gain familial ; lecture des anciens droits à K.
- Achats/ventes liés aux révisions exactes du journal ; source et cash créés ou corrigés ensemble, sans deuxième encaissement ni substitution silencieuse de leur origine.
- Valorisation dérivée des métaux dans le contrôle d’attribution et le bilan, remplaçant l’estimation générique du même actif ; droits familiaux/individuels et barrières des sociétés globales conservés.
- Observations de gestion mensuelles, trimestrielles ou annuelles ; champs inconnus conservés, périodes compatibles comparées et fréquences séparées, sans ajouter des indicateurs au patrimoine.
- Contrôle de société avec profils, participations, actifs, comptes, financements, périodes terminées et tendances sourcées ; stocks entiers des comptes distincts des droits économiques, financements déjà attribués à leur part contractuelle.
- Déclarations de dividendes distinctes des paiements ; deux côtés réellement suivis reliés au journal, paiements et corrections atomiques, raisons explicites pour les sources historiques incomplètes.
- Migration `0006` vers le schéma 6 sur copie ; sauvegardes incluant les nouveaux faits, enfants, révisions et anciennes connaissances.

Le lot suivi est à un seul lieu à chaque date ; la scission physique avec filiation automatique de lots n’est pas une commande livrée. Les transferts de propriété portent sur les fractions datées du même stock identifié. Le pont de variation historique du registre garde son périmètre de phase 4 et signale l’exclusion des valeurs dérivées de métaux ; il n’est pas présenté comme une performance complète du nouveau bilan. Les tableaux de contrôle de sociétés ne constituent pas une comptabilité générale ni les tableaux de bord graphiques de phase 9.

## Documents et portabilité implémentés — phase 8

- Bibliothèque documentaire avec descriptions, recherche, classement, identifiants, empreintes et consultation à K ; filtres d’association selon la date économique D.
- Associations multiples aux objets métier existants, périodes datées, corrections motivées et retrait d’un seul lien sans perdre les autres ni l’original.
- Remplacement de pièce par nouvelle version conservant identité, anciens octets et associations ; archivage, retrait confirmé du classement et rétablissement sans purge physique implicite.
- Ouverture explicite d’une copie vérifiée par le lecteur système et export de la version choisie ; déduplication physique des contenus sans fusion des documents logiques.
- Snapshot cohérent commun aux exports intégraux, indépendants des filtres membre/date/K ; tables, révisions, sources, réglages, audit et originaux conservés.
- Archive CSV UTF-8/RFC 4180 versionnée, toutes les tables même vides, clés et relations explicites, décimaux exacts et marqueurs distinguant NULL de la chaîne vide.
- Réimport CSV dans une candidate vierge avec schéma livré, validations des archives, tables, relations, règles métier et originaux avant confirmation ; aucune fusion ni SQL reçu de l’archive exécuté.
- Provenance technique du réimport ajoutée dans la candidate sans modifier les identités ni les anciens K ; activation et sauvegarde préalable par le même mécanisme que la restauration SQLite.
- Classeur XLSX de consultation : tables et historiques complets, feuilles de dictionnaire et d’informations, données utilisateur textuelles inertes, limites de feuilles gérées et textes longs récupérables par fragments.
- Schéma SQLite 6 conservé, sans modification des migrations 1–6 ; anciennes sauvegardes SQLite mises à niveau sur copie, premier format CSV réimportable dédié au schéma 6.

Le CSV et le ZIP SQLite incluent les originaux ; le XLSX contient leurs métadonnées et associations. La suppression documentaire est logique, et l’import CSV remplace l’espace entier après analyse et confirmation. Le suivi fiscal, la comptabilité générale et les tableaux de bord graphiques ne sont pas ajoutés par cette phase.

## Tableaux de bord et rapports implémentés — phase 9

- Tableau de bord familial ou individuel alimenté par le bilan financier existant, avec contexte D/K, couvertures, sources et montants inconnus conservés.
- Vue de pilotage de société reprenant ses comptes, financements, participations, observations et distributions ; aucun résultat de gestion ne devient une banque ni un patrimoine net supposé.
- Une lecture SQLite cohérente résout K pour le rapport, tous ses membres et tous ses points historiques ; la consultation ne crée aucune révision.
- Répartitions par type de position, pays et devise ; positions résiduelles après éliminations internes, garanties conditionnelles séparées et calculs rationnels dans Rust.
- Recherche et filtres des positions par pays, devise, catégorie et propriétaire associé ; ce dernier sélectionne des positions entières, sans recalculer une quote-part juridique.
- Recherche globale des objets métier et documents connus à K, avec descriptions datées à D, éléments retirés signalés et renvoi au module ; périmètre du coffre entier explicitement distinct des filtres du tableau de bord.
- Comparaison des bilans individuels ; leurs actifs et passifs bruts ne sont pas additionnés pour remplacer le bilan familial après neutralisations.
- Historique mensuel ou annuel à K fixe, borné à 61 points, avec données de début et de fin ; les lacunes de net complet restent distinctes des sous-totaux connus.
- Variation entre les bornes calculée exactement, sans la qualifier de rendement ; pont historique du registre présenté avec sa portée partielle et ses propres sources.
- Graphiques Recharts embarqués localement, montants exacts disponibles dans leurs tables et libellés ; aucun chiffre de démonstration ni calcul financier JavaScript.
- Alertes et références des moteurs métier conservées ; rapport JSON filtré exportable par dialogue natif, distinct des archives complètes de récupération.
- Schéma SQLite 6 et migrations 1–6 inchangés ; sauvegardes, CSV intégral et XLSX de portabilité conservent leur périmètre indépendant des filtres du tableau de bord.

Les indicateurs de gestion d’une société et les revenus locatifs gardent leur contexte et leurs conventions propres. Un sous-total connu ou un résultat de période ne certifie pas un net complet. La décomposition exhaustive de la variation du bilan en flux et performance n’est pas déduite du pont historique limité du registre. La qualification de livraison Linux reste en phase 10.

## Qualification et livraison Linux — phase 10

- Construction AppImage en mode release pour la cible déclarée Debian 13 x86_64, avec version de paquet, empreinte SHA-256, manifeste de construction et notices tierces.
- Programme séparé des données XDG ; procédure de sauvegarde préalable, remplacement du paquet et retour à une version compatible documentée.
- Scénario de qualification exécuté sur des candidats locaux 0.1.0 puis 0.1.1, sans présenter le premier comme une ancienne diffusion publique ; seuls les essais exécutés et leurs résultats de lancement et de préservation sont annoncés dans le rapport actuel.
- Revue des capacités IPC, des chemins documentaires, de la récupération et des dépendances ; portée et limites explicites dans la revue de sécurité.
- Contrôles d’accessibilité automatisés et parcours clavier sur les scénarios déclarés ; aucune certification générale déduite de ces seuls contrôles.
- Mesures de temps et de mémoire sur fixtures synthétiques, avec matériel et profondeur historique déclarés ; la cible provisoire de deux secondes n’est pas transformée en garantie sans mesure.
- Budgets de portabilité adaptés au jeu documentaire prévu : 11 000 entrées ZIP et manifeste de 4 Mio, base SQLite canonique de 256 Mio, autres fichiers de 128 Mio et total décompressé de 512 Mio ; les contrôles de chemin, empreinte et intégrité restent appliqués.
- Guide utilisateur d’installation, mise à jour et récupération SQLite/CSV autonome, en complément des guides métier des phases précédentes.

Les [preuves de phase 10](../historique/validation-phase-10.md), le [guide Linux](../utilisateur/installation-linux.md) et la [revue de sécurité](../technique/revue-securite.md) donnent le périmètre de cette qualification locale. Les archives, formats et schéma restent ceux déjà livrés ; aucune publication distante n’est effectuée.

Les deux voies de récupération du grand coffre préservent les 773 124 lignes et 10 000 originaux. Ce profil reste toutefois lent : la cible provisoire de deux secondes n’est pas atteinte et le tableau de bord dépasse 120 secondes, soit 14 opérations abouties sur 15. Les deux avertissements de dépendances amont restent ouverts et documentés.

## État des phases

| Phase | État | Condition de sortie |
|---|---|---|
| 2 — Socle | Implémenté, voir preuves de validation | Tauri Linux exécutable, SQLite et migrations, sauvegarde/restauration minimale testée |
| 3 — Famille et propriété | Implémentée, voir preuves de validation | CRUD validés et moteur de consolidation testé avant tableaux de bord |
| 4 — Devises/historique | Implémentée, voir preuves de validation | Précision, D/K, profils datés et restauration vérifiés |
| 5 — Immobilier/loyers | Implémentée, voir preuves de validation | Biens, baux, paiements partiels, corrections atomiques et indicateurs vérifiés |
| 6 — Liquidités/dettes | Implémentée, voir preuves de validation | Soldes sourcés, transferts, financements, sources atomiques et neutralisations contrôlés ; couverture explicite pour un net complet |
| 7 — Métaux/sociétés | Implémentée, voir preuves de validation | Stocks, coûts, cotations, droits, cash et indicateurs sourcés sans doubles comptes |
| 8 — Portabilité complète | Implémentée, voir preuves de validation | Documents, CSV réimportable, XLSX et deux voies de récupération contrôlées |
| 9 — Tableaux de bord | Implémentée, voir preuves de validation | Vues famille/membre/société, répartitions, historique, alertes et rapports fondés sur les moteurs réels |
| 10 — Livraison Linux | Qualification locale, voir résultats et limites | Paquet, récupération, remplacement du programme, mesures et revues documentés |

## Vérification de ce lot

Phase 1 : contrôles documentaires, couverture du cahier des charges, cohérence des conventions et références. Les scénarios des modules à venir restent planifiés ; ceux du moteur de phase 3 ont désormais été exécutés.

Phase 2 : les résultats de compilation, tests Rust/UI et parcours desktop sont consignés séparément dans [le rapport de validation](../historique/validation-phase-2.md). Les limites de la simulation des pannes y sont explicites ; le passage des tests du socle ne vaut pas validation financière ni qualification AppImage.

Phase 3 : les preuves de calcul, de persistance, de migration et de parcours desktop sont consignées dans [le rapport du registre](../historique/validation-phase-3.md). Elles portent sur cette version et ne valent pas validation des modules de devises, de location ou de dettes ajoutés ensuite.

Phase 4 : les campagnes de calcul pur, stockage, migration, interface et parcours Linux sont consignées dans [le rapport devises et historique](../historique/validation-phase-4.md). Les résultats des phases précédentes restent des preuves de leurs versions respectives.

Phase 5 : les commandes et résultats de cette version sont consignés dans [la validation immobilier et loyers](../historique/validation-phase-5.md), avec [le guide d’utilisation](../utilisateur/immobilier-et-loyers.md) et les limites de couverture de cette phase.

Phase 6 : les commandes, résultats et limites de cette version figurent dans [la validation comptes et financements](../historique/validation-phase-6.md). Le [guide comptes et financements](../utilisateur/comptes-et-financements.md) décrit les parcours et les conditions de couverture du bilan.

Phase 7 : les commandes, résultats et limites de cette version figurent dans [la validation métaux et sociétés](../historique/validation-phase-7.md). Le [guide métaux et sociétés](../utilisateur/metaux-et-societes.md) décrit les parcours, conventions de coût, droits et distributions. Les chiffres des campagnes restent dans le rapport de validation.

Phase 8 : les commandes, résultats et limites de cette version figurent dans [la validation documents et portabilité](../historique/validation-phase-8.md). Le [guide](../utilisateur/documents-et-portabilite.md) distingue la bibliothèque historique, le classement, les originaux, les archives récupérables et le XLSX de consultation.

Phase 9 : les commandes, résultats et limites figurent dans [la validation des tableaux de bord](../historique/validation-phase-9.md). Le [guide](../utilisateur/tableaux-de-bord.md) précise les filtres, les répartitions après éliminations, les historiques à K et le rapport de consultation.

Phase 10 : les commandes, résultats et limites actuels figurent dans [la qualification Linux](../historique/validation-phase-10.md). Le [guide d’installation et de récupération](../utilisateur/installation-linux.md) décrit l’utilisation du paquet, les données XDG, les mises à jour manuelles et les procédures de secours. La [revue de sécurité](../technique/revue-securite.md) expose les contrôles des capacités et dépendances.

## Suite du projet

Conserver les scénarios de non-régression, les procédures de récupération et les notices lors des mises à jour. Traiter les retours d’usage et qualifier explicitement les postes, volumes ou fonctionnalités supplémentaires avant d’élargir les engagements. Les limites métier et de qualification décrites dans les guides restent applicables ; aucune nouvelle phase fonctionnelle n’est présumée après le plan initial.
