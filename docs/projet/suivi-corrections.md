# Suivi des corrections

Autorisation du 16 novembre 2026 : appliquer tous les changements du [rapport](revue-ameliorations.md), sans demander de confirmation. Les observations historiques sont conservées ; ce suivi indique les réalisations et leurs validations.

## Point de reprise

Les changements sont intégrés dans la version 0.1.2. La validation de la version et la publication de l’AppImage Pop!_OS 22.04 sont suivies dans le [rapport de validation](validation-corrections-0.1.2.md). En cas d’échec de publication, les fichiers concernés et leurs chemins dans le dépôt seront fournis pour une publication manuelle.

| Référence | Changement | État | Validation |
|---|---|---|---|
| DOC-01 | Clarifier le net familial lorsqu’une créance interne est dépréciée | Intégré | Exemple de prêt déprécié et règle harmonisée dans les décisions et règles métier. |
| DOC-02 | Harmoniser les explications sur les documents à une ancienne connaissance K | Intégré | Guides alignés sur la bibliothèque à K et les opérations intégrales sur le coffre actuel. |
| DOC-03 | Rendre les renvois documentaires utilisables depuis GitHub | Intégré | Index de preuves versionné et vérification des chemins depuis les pages GitHub. |
| DOC-04 | Relier les exigences au périmètre effectivement reçu | Intégré | Matrice des exigences, de la réception et des reliquats. |
| DOC-05 | Ajouter une correspondance entre conception et schéma livré | Intégré | Correspondance entre concepts, tables, migrations et décisions. |
| DOC-06 | Unifier la borne de pureté des métaux | Intégré | Borne `]0,1]` dans les règles et guides. |
| DOC-07 | Distinguer les programmes couverts par chaque campagne | Intégré | Matrice des campagnes, empreintes des sources et provenance automatique des futures constructions. |
| DOC-08 | Inventorier les anciennes preuves et les critères de réception | Intégré | Index des pièces disponibles et critères de réception. 70 membres du ZIP contrôlés. |
| ARCH-01 | Contrôler la correspondance des contrats Rust et TypeScript | Intégré | Comparaison AST Rust/TypeScript intégrée à la CI et tests de dérive. |
| ARCH-02 | Exploiter des codes d’erreur et statuts stables | Intégré | Codes et actions typés, statut documentaire sérialisé et tests API. |
| ARCH-03 | Indexer une fois les capacités des lignes de trésorerie | Intégré | Index des capacités par révision et ligne, validé par les suites de trésorerie et d’opérations. |
| ARCH-04 | Compléter quelques frontières des normalisations | Intégré | Tests des normalisations, de l’idempotence et des bornes de longueur. |
| REG-01 | Compléter les sources expliquant les contributions | Intégré | Sources des appartenances et méthodes retenues, test de contributions. |
| REG-02 | Préparer les index et chemins de change pour un snapshot donné | Intégré | Index de droits, valorisations et profils. Chemins FX mis en cache dans le contexte immuable du calcul. |
| REG-03 | Séparer écriture confirmée et actualisation de l’affichage | Intégré | Résultat calculé avant le commit. Confirmation d’écriture distincte des erreurs de relecture, avec tests UI. |
| REG-04 | Mesurer aussi une longue succession de connaissances | Intégré | Rejeu limité aux K qui modifient les sources et mesure reproductible sur 3 000 changements mixtes. |
| REG-05 | Rendre les confirmations accessibles au clavier | Intégré | Focus des confirmations et restitution au déclencheur, avec tests clavier. |
| REG-06 | Identifier une conversion dont la saisie a changé | Intégré | Conversion invalidée dès que sa saisie change. K effectif affiché dans le pont. |
| REG-07 | Remplacer les annonces de modules futurs déjà livrés | Intégré | Références aux modules disponibles dans les formulaires et l’historique. |
| REG-08 | Ajouter une frontière persistante explicite à K=0 | Intégré | Test du registre et des archives à K=0 après corrections. |
| IMMO-01 | Ne pas rendre inconnus les revenus à cause d’un bail sans exposition économique | Intégré | Exposition nulle distinguée des droits inconnus dans les indicateurs locatifs. |
| IMMO-02 | Conserver la confirmation après rechargement du parent | Intégré | Confirmation conservée après relecture du parent, test d’intégration interface. |
| IMMO-03 | Bloquer les onglets pendant une opération enfant | Intégré | Onglets bloqués pendant une sauvegarde enfant, test d’intégration interface. |
| IMMO-04 | Distinguer les biens, unités et baux homonymes | Intégré | Libellés distincts par bien, unité et période, tests du module et des opérations bancaires. |
| IMMO-05 | Réduire les recherches répétées sans retirer les contrôles | Intégré | Index des clés de génération et des identités, régression historique dans les suites de stockage. |
| IMMO-06 | Verrouiller explicitement l’immutabilité de devise d’un bail | Intégré | Correction de devise d’un bail avec échéance refusée et absence de mutation vérifiée. |
| FIN-01 | Vérifier les parties d’un financement représentant une créance locative | Intégré | Validation des parties et de la capacité. Régression de migration d’un ancien coffre et bilan incomplet avec alerte. |
| FIN-02 | Afficher l’annulation des prévisions d’échéance | Intégré | Prévisions annulées présentées avec leur état et leurs montants historiques. |
| FIN-03 | Harmoniser la promesse d’archivage et les nouvelles saisies | Intégré | Contrats archivés disponibles pour consultation et correction, exclus des nouvelles saisies. |
| FIN-04 | Ordonner chronologiquement les relevés et changes exécutés | Intégré | Tri des relevés et changes exécutés par date et identifiant. |
| MET-01 | Fixer le lot cible pendant un transfert des droits | Intégré | Cible figée dans le transfert et brouillon fermé au changement de lot, test de l’opération. |
| MET-02 | Convertir les primes et décotes même sans cotation du métal | Intégré | Conversion des primes et décotes indépendamment du cours de métal. |
| MET-03 | Garder calculable un stock confirmé nul malgré une cotation étrangère | Intégré | Stock nul valorisable sans taux FX, régression dans le bilan intermodules. |
| SOC-01 | Expliquer la règle de chevauchement indépendamment de la devise | Intégré | Règle de chevauchement harmonisée, quelle que soit la devise. |
| SOC-02 | Conserver deux frontières de distribution dans les tests | Intégré | Tests de distribution dans une même barrière opaque et de détention à 50 %. |
| PORT-01 | Exposer l’état réellement activé après une erreur de finalisation | Intégré | Génération active et finalisation du journal affichées séparément, tests d’erreurs injectées. |
| PORT-02 | Guider la reprise d’une sauvegarde dont la publication n’est pas durablement confirmée | Intégré | Publication visible mais durabilité incertaine signalée avec destination et action. |
| DOCU-01 | Unifier les métadonnées documentaires entre les chemins d’accès | Intégré | MIME commun aux chemins d’import et remplacement, date d’ajout vérifiée. |
| DOCU-02 | Garder les associations retirées identifiables | Intégré | Nom historique et état retiré conservés, test du libellé à un ancien K. |
| DOCU-03 | Gérer la rétention des copies de consultation | Intégré | Purge au bout de sept jours pour les copies de consultation, test des frontières. |
| DOCU-04 | Expliquer la reprise d’un formulaire après conflit | Intégré | Comparaison et adoption volontaire de la révision actuelle, saisie conservée. |
| PORT-03 | Traiter les tables CSV progressivement | Intégré | Lecture CSV par ligne, limites strictes et rollback en cas d’erreur tardive. |
| DASH-01 | Ne pas attribuer un pays courant à une position historique de pays inconnu | Intégré | Pays historique inconnu tant que le profil daté manque, régression à deux K. |
| DASH-02 | Appliquer le plafond de points aux séries effectivement construites | Intégré | Plafond appliqué aux séries effectivement construites, fenêtre longue de société testée. |
| SEARCH-01 | Ouvrir les opérations d’espèces dans le module correspondant | Intégré | Routage des sources d’espèces et contrôle du focus de destination. |
| SHELL-01 | Agréger les opérations actives avant de libérer la navigation | Intégré | Agrégation des opérations actives, test dans les deux ordres de résolution. |
| SHELL-02 | Garder les données masquées jusqu’au dernier chargement K | Intégré | Réponses historiques obsolètes ignorées, test dans les deux ordres de résolution. |
| DASH-03 | Proportionner éventuellement l’abscisse à la durée écoulée | Intégré | Abscisse proportionnelle au temps, les valeurs exactes du moteur restent affichées. |
| STORE-01 | Compléter la durabilité des nouveaux dossiers documentaires | Intégré | Synchronisation de la chaîne des répertoires documentaires, tests d’erreurs injectées. |
| STORE-02 | Prendre en charge ou expliquer les supports sans liens physiques | Intégré | Publication exclusive compatible avec les systèmes qui prennent en charge `RENAME_NOREPLACE`, tests de concurrence. |
| STORE-03 | Valider les métadonnées techniques et la correspondance des audits | Intégré | Contrôle des dates techniques, identifiants et relations d’audit avant activation. |
| STORE-04 | Prévoir une échéance ou annulation du snapshot | Intégré | Échéance de copie SQLite et nettoyage des candidats, test avec verrou externe. |
| NATIVE-01 | Présenter les erreurs de démarrage au lancement graphique | Intégré | Diagnostic graphique avant ouverture et distinction du verrou occupé. |
| NATIVE-02 | Rendre les échecs du lecteur de document consultables | Intégré | Remontée des erreurs de lancement du lecteur, avec accès à l’export manuel. |
| MAINT-01 | Exclure les copies de construction de la découverte des tests | Intégré | Découverte Vitest limitée aux sources, copies locales exclues. |
| MAINT-02 | Harmoniser les métadonnées avec la licence ajoutée au dépôt | Intégré | GPLv3 et version 0.1.2 alignées dans les métadonnées et notices. |
| MAINT-03 | Organiser le renouvellement des bases d’avis et le suivi des avertissements | Intégré | Contrôle hebdomadaire et comparaison locale de bases publiques actualisées le 20 novembre. |
| MAINT-04 | Faire dépendre la qualification de la sortie réelle du processus | Intégré | Résultat du superviseur lié au code de sortie, tests des deux modes. |
| MAINT-05 | Contrôler les notices avant chaque construction générique | Intégré | Contrôle des notices dans les deux passes de construction et dans l’AppDir final. |
| MAINT-06 | Garder l’origine du paquet lors du choix de sa notice native | Intégré | Notices choisies dans le même environnement que le binaire et sa version. |
