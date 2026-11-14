# CAHIER DES CHARGES COMPLET — APPLICATION DE GESTION PATRIMONIALE FAMILIALE INTERNATIONALE

**Version 3.0 — Projet de développement avec Codex**

## 1. TA MISSION

Tu es un architecte logiciel et développeur senior spécialisé dans :

- Les applications desktop Linux.
- Les bases de données relationnelles.
- Les systèmes financiers et patrimoniaux.
- Les interfaces React et TypeScript.
- Rust, Tauri et SQLite.
- La gestion des structures juridiques complexes.
- La sécurité, la pérennité et la portabilité des données.

Ta mission est de concevoir puis développer une application locale de gestion patrimoniale familiale et internationale.

L'application doit permettre de gérer le patrimoine de plusieurs membres d'une même famille, individuellement et collectivement, à travers différents pays, devises et structures juridiques.

Le logiciel doit être réellement utilisable au quotidien, et non constituer une simple démonstration.

Il ne s'agit pas de créer un logiciel de comptabilité générale, un ERP, un logiciel fiscal ou un système bancaire.

L'objectif est de disposer d'un **centre de pilotage patrimonial familial**, permettant de connaître :

- Le patrimoine de chaque membre de la famille.
- Le patrimoine familial consolidé.
- Les actifs détenus directement.
- Les actifs détenus indirectement par des sociétés.
- Les biens détenus conjointement.
- Les dettes.
- Les revenus locatifs.
- Les liquidités disponibles.
- Les quantités et valeurs de métaux précieux.
- L'évolution historique du patrimoine.
- La situation économique simplifiée des sociétés familiales.

L'interface doit être entièrement en français.

L'utilisateur principal administre l'application depuis un ordinateur Linux.

---

## 2. EXIGENCES NON NÉGOCIABLES

Les priorités du projet sont, dans cet ordre :

1. Intégrité et sécurité des données.
2. Possibilité de récupérer intégralement les données sans dépendre de l'application.
3. Exactitude des calculs patrimoniaux.
4. Gestion correcte des propriétés et participations juridiques.
5. Consolidation familiale sans double comptabilisation.
6. Historisation des actifs, flux et valorisations.
7. Simplicité d'utilisation.
8. Fonctionnement entièrement local et hors ligne.
9. Maintenabilité et évolutivité du code.
10. Ergonomie et qualité visuelle.

Les données doivent appartenir exclusivement à l'utilisateur.

Ne pas imposer de compte externe, d'abonnement, de service cloud ou de serveur distant.

Ne jamais rendre l'accès aux données dépendant d'un service tiers.

**La sauvegarde, l'export et la restauration sont des fonctionnalités fondamentales, et non des options à développer uniquement à la fin du projet.**

---

## 3. ENVIRONNEMENT TECHNIQUE

L'application est destinée à Linux.

Architecture privilégiée :

- Tauri 2 pour l'application desktop.
- React et TypeScript pour l'interface.
- Rust pour la logique métier, les calculs financiers et les opérations sensibles.
- SQLite pour la persistance locale.
- Migrations de base de données versionnées.
- Bibliothèque de composants UI cohérente.
- Bibliothèque de graphiques React maintenue.
- Exports CSV, XLSX et ZIP.
- Format de sauvegarde SQLite standard.

Produire prioritairement un AppImage Linux.

Prévoir éventuellement un paquet .deb.

L'application doit fonctionner sans connexion Internet.

Ne pas nécessiter de serveur HTTP distant pour l'utilisation courante.

Utiliser des services métier clairement séparés de l'interface.

Éviter les requêtes SQL métier directement dans les composants React.

Prévoir une arborescence de projet documentée et maintenable.

Stocker les données utilisateur dans les emplacements appropriés de Linux, indépendamment du répertoire d'installation.

---

## 4. PÉRIMÈTRE GÉOGRAPHIQUE ET MONÉTAIRE

Le patrimoine concerne principalement quatre juridictions :

| Pays | Code | Devise principale |
|---|---|---|
| France | FR | EUR |
| Tunisie | TN | TND |
| Suisse | CH | CHF |
| Émirats arabes unis | AE | AED |

Dubaï doit être représenté comme une localisation au sein des Émirats arabes unis, et non comme un pays indépendant.

Les actifs et sociétés peuvent se trouver dans des juridictions différentes.

Exemples :

- Une personne résidant en France possède un appartement en Tunisie.
- Une société suisse possède un appartement à Dubaï.
- Une société tunisienne possède un compte bancaire en EUR.
- Une société française détient une participation dans une société émiratie.

Prévoir la possibilité d'ajouter d'autres pays sans refondre le logiciel.

### Devise de consolidation

La devise de référence de l'application est **EUR**.

Tous les tableaux de bord consolidés doivent être calculés en EUR.

Néanmoins, conserver systématiquement les valeurs dans leur devise d'origine.

Lorsqu'une valorisation est affichée, permettre de consulter :

- Montant d'origine.
- Devise d'origine.
- Montant converti en EUR.
- Taux de conversion utilisé.
- Date du taux.

Prendre en charge prioritairement EUR, TND, CHF et AED, puis USD et d'autres devises éventuelles.

---

## 5. GESTION DES MEMBRES DE LA FAMILLE

Ce module est central et doit être prévu dès la conception de la base de données.

L'application doit gérer plusieurs membres de la même famille.

Chaque membre possède son propre patrimoine, mais peut également détenir des actifs conjointement avec d'autres membres.

### Fiche d'un membre

Prévoir les champs suivants :

- Identifiant unique et stable.
- Prénom et nom.
- Lien familial facultatif.
- Date de naissance facultative.
- Pays de résidence facultatif.
- Devise d'affichage préférée.
- Notes.
- Documents associés.
- Statut actif ou archivé.
- Dates d'appartenance au périmètre de consolidation familiale.

Le nombre de membres ne doit pas être limité artificiellement.

Une personne extérieure à la famille doit pouvoir être enregistrée comme copropriétaire ou associé sans être incluse dans le patrimoine familial.

### Tableau de bord familial

Créer une vue « Patrimoine familial » qui présente :

- Patrimoine brut familial.
- Dettes attribuables à la famille.
- Patrimoine net familial.
- Historique de l'évolution patrimoniale.
- Répartition du patrimoine par membre.
- Répartition géographique.
- Répartition par classe d'actifs.
- Répartition par devise.
- Liquidités familiales.
- Immobilier familial.
- Loyers.
- Métaux précieux.
- Participations dans les sociétés.

Les actifs détenus conjointement ne doivent pas être comptabilisés plusieurs fois.

### Tableau de bord individuel

Chaque membre dispose de son propre espace.

Il doit pouvoir consulter :

- Son patrimoine brut.
- Ses dettes.
- Son patrimoine net.
- Ses actifs immobiliers.
- Ses loyers attribuables.
- Ses comptes bancaires.
- Ses espèces.
- Ses métaux précieux.
- Ses participations dans les sociétés.
- Ses documents.
- L'évolution historique de sa situation.

La vue individuelle doit intégrer les détentions directes et indirectes.

### Navigation

Prévoir un sélecteur global de contexte permettant de basculer entre :

- Patrimoine familial.
- Membre A.
- Membre B.
- Membre C.
- Autres membres.

La sélection doit automatiquement adapter les listes, graphiques et rapports.

### Architecture

Ne pas créer une base de données indépendante pour chaque membre.

Utiliser une base relationnelle commune, avec des propriétaires identifiés, afin de gérer correctement les biens partagés.

Les espaces individuels sont des vues patrimoniales. En V1, ils ne nécessitent pas obligatoirement des comptes de connexion différents.

Ne pas présenter ces vues comme une garantie de confidentialité entre membres : l'administrateur local peut consulter toutes les données.

Préparer l'architecture pour de futurs droits d'accès, sans imposer un système multi-utilisateur complexe dès la V1.

---

## 6. PROPRIÉTÉ JURIDIQUE ET PARTICIPATIONS

Il s'agit de l'une des parties les plus importantes de l'application.

Créer un modèle permettant de représenter correctement les structures de détention.

### Types de propriétaires

- Personnes physiques membres de la famille.
- Personnes physiques extérieures.
- Sociétés.
- Autres entités juridiques si nécessaire.

### Types de propriété

Un actif peut appartenir :

- À une personne physique à 100 %.
- À plusieurs personnes physiques.
- À une société.
- À plusieurs sociétés.
- À une combinaison de personnes physiques et de sociétés.

Une société peut posséder d'autres sociétés.

Une société peut se trouver dans un pays et posséder des actifs dans plusieurs autres pays.

### Exemple 1 — Copropriété familiale

Un appartement vaut 400 000 €.

Il appartient :

- À 50 % au membre A.
- À 30 % au membre B.
- À 20 % à une personne extérieure.

Résultats attendus :

- Vue membre A : 200 000 €.
- Vue membre B : 120 000 €.
- Vue famille : 320 000 €.
- Vue juridique de l'appartement : 400 000 €.

### Exemple 2 — Détention indirecte

- Membre A possède 60 % de Société X.
- Société X possède 50 % de Société Y.
- Société Y possède un appartement à 100 %.

La participation économique indirecte du membre A dans cet appartement est de 30 %.

### Exemple 3 — Société familiale

Une société est détenue par :

- Membre A : 40 %.
- Membre B : 35 %.
- Associé extérieur : 25 %.

La participation économique familiale est de 75 %.

### Règles impératives

Distinguer :

1. Le propriétaire juridique.
2. Les droits économiques.
3. Les droits de vote lorsque pertinents.
4. La valeur brute.
5. Les dettes associées.
6. La valeur nette économiquement attribuable.

Calculer les participations indirectes à travers plusieurs niveaux de sociétés.

Pour les chemins de participation multiples, cumuler correctement les droits économiques.

Ne pas imposer l'hypothèse simplificatrice selon laquelle toutes les dettes se répartissent automatiquement de la même manière que les droits de propriété.

Prendre en charge la distinction entre dette de l'entité, dette personnelle et garantie donnée par un tiers.

Éviter les doubles comptabilisations.

Une société et les actifs qu'elle détient ne doivent pas être comptés simultanément comme deux actifs indépendants dans le patrimoine consolidé.

Interdire ou gérer explicitement les cycles de détention. Pour la V1, rejeter les structures cycliques non prises en charge.

Historiser les participations avec leurs dates de validité.

Les contrôles doivent garantir la cohérence des pourcentages.

Ne pas considérer qu'un pourcentage de détention manquant est égal à zéro ou à 100 %.

### Valorisation des sociétés

Distinguer les sociétés dont les actifs et passifs sont détaillés dans l'application de celles pour lesquelles seule une valorisation globale est connue.

Ne jamais additionner une valorisation globale des parts sociales à des actifs sous-jacents déjà inclus dans la consolidation.

Lorsque les données de consolidation sont incomplètes, afficher le périmètre incomplet ou utiliser une valorisation globale explicitement identifiée selon une règle documentée.

---

## 7. MODULE IMMOBILIER

Créer un module immobilier complet.

### Fiche de bien

Champs nécessaires :

- Identifiant.
- Nom ou désignation.
- Pays.
- Ville.
- Adresse.
- Type de bien.
- Surface.
- Date d'acquisition.
- Prix d'achat.
- Frais d'acquisition.
- Devise d'acquisition.
- Propriétaire ou propriétaires.
- Pourcentages de détention.
- Statut.
- Valeur actuelle estimée.
- Historique des estimations.
- Financements.
- Notes.
- Documents.

Types de biens :

- Appartement.
- Maison.
- Immeuble.
- Terrain.
- Local commercial.
- Bureau.
- Garage.
- Autre.

Statuts :

- Occupé par le propriétaire.
- Loué.
- Vacant.
- En travaux.
- En vente.
- Vendu.

Ne pas supprimer l'historique d'un bien vendu.

Permettre de gérer plusieurs lots ou unités locatives au sein d'un même immeuble.

### Valorisation

Prévoir plusieurs estimations datées.

Conserver l'historique des valorisations plutôt que d'écraser la valeur précédente.

Distinguer :

- Coût d'acquisition.
- Coût total d'acquisition.
- Valeur estimée.
- Plus-value latente.
- Dette restante.
- Valeur nette avant frais et fiscalité de cession.

Documenter les hypothèses de calcul.

---

## 8. GESTION DES LOYERS ET DES LOCATIONS

Un bien immobilier peut être associé à plusieurs contrats successifs.

Chaque contrat comporte :

- Locataire.
- Bien ou unité concernée.
- Date de début.
- Date de fin.
- Loyer contractuel.
- Charges contractuelles.
- Devise.
- Fréquence.
- Dates d'échéance.
- Révisions de loyer.
- Dépôt de garantie.
- Notes et documents.

### Suivi des paiements

Gérer :

- Loyers attendus.
- Loyers exigibles.
- Loyers encaissés.
- Paiements partiels.
- Retards.
- Impayés.
- Régularisations.
- Vacance locative.
- Charges et dépenses.
- Travaux.

Un paiement peut couvrir plusieurs échéances et une échéance peut être réglée par plusieurs paiements.

Permettre d'associer un encaissement à un mouvement bancaire ou de caisse.

Ne pas compter une seconde fois un loyer déjà enregistré en trésorerie.

Le dépôt de garantie doit être traité séparément des revenus locatifs.

### Indicateurs

Calculer :

- Loyers contractuels annualisés.
- Loyers exigibles sur la période.
- Loyers encaissés.
- Impayés.
- Charges d'exploitation.
- Revenus nets d'exploitation.
- Rendement locatif brut.
- Rendement net avant financement.
- Cash-flow après financement.
- Taux d'occupation.
- Plus-value latente.

Permettre les vues par bien, pays, société propriétaire, membre de la famille et période.

Distinguer clairement le revenu généré économiquement par un bien de la somme effectivement reçue par un membre de la famille.

Un bénéfice conservé dans une société n'est pas automatiquement une distribution de liquidités à ses actionnaires.

---

## 9. MODULE COMPTES BANCAIRES

Permettre d'enregistrer plusieurs comptes bancaires dans plusieurs devises.

Chaque compte possède :

- Identifiant.
- Banque.
- Pays.
- Titulaire juridique.
- Devise.
- Solde d'ouverture.
- Date d'ouverture du suivi.
- Historique des transactions.
- Solde calculé.
- Documents et notes.

### Transactions

Prévoir :

- Entrée d'argent.
- Sortie d'argent.
- Transfert entre comptes.
- Conversion de devise.
- Paiement de loyer.
- Paiement de charges.
- Remboursement de dette.
- Acquisition ou vente d'actif.
- Distribution de dividendes.
- Autre opération.

Un transfert entre deux comptes appartenant au même périmètre économique ne doit pas être présenté comme un revenu.

Une conversion de devise doit enregistrer :

- Montant débité.
- Devise débitée.
- Montant crédité.
- Devise créditée.
- Taux effectif.
- Frais.
- Date.

Permettre le rapprochement d'un solde calculé avec un relevé bancaire.

Ne pas intégrer de connexion bancaire automatique dans la V1.

---

## 10. MODULE ESPÈCES PHYSIQUES

Les espèces doivent être gérées séparément des comptes bancaires.

Chaque caisse possède :

- Devise.
- Propriétaire.
- Pays.
- Lieu de conservation.
- Solde initial.
- Mouvements.
- Solde actuel.

Exemples :

- Espèces en EUR en France.
- Espèces en TND en Tunisie.
- Espèces en CHF en Suisse.
- Espèces en AED à Dubaï.

Permettre :

- Ajouts.
- Retraits.
- Transferts.
- Conversions.
- Corrections justifiées.

Conserver les historiques.

Afficher le total en devise d'origine et en EUR.

---

## 11. MODULE TAUX DE CHANGE

### Fonctionnement de la V1

Les taux de change sont saisis manuellement.

Aucune connexion à un fournisseur externe n'est obligatoire.

Créer une interface simple pour mettre à jour les taux des différentes devises.

Pour chaque taux, enregistrer :

- Devise de départ.
- Devise d'arrivée.
- Valeur.
- Date d'application.
- Date de saisie.
- Source éventuelle.
- Notes.

Définir une convention de cotation claire, par exemple le nombre d'EUR correspondant à une unité de devise étrangère.

Gérer correctement les inversions et conversions croisées lorsque les données disponibles le permettent.

### Historique

Conserver tous les cours historiques.

Pour afficher le patrimoine à une date passée, utiliser le cours applicable à cette date, selon une politique explicitement définie.

Une politique de dernier taux connu antérieur ou égal à la date demandée peut être utilisée, avec une indication visible de l'ancienneté du cours.

Ne pas utiliser de cours futurs pour valoriser silencieusement une situation historique.

En l'absence de taux, signaler la valorisation impossible.

Ne jamais inventer une valeur.

Prévoir plus tard la possibilité d'une récupération automatique des taux, sans l'implémenter obligatoirement en V1.

---

## 12. MODULE OR ET ARGENT MÉTAL

Créer un module de gestion des métaux précieux physiques.

Métaux concernés :

- Or.
- Argent.

### Fiche de lot

Chaque lot comprend :

- Identifiant.
- Métal.
- Type ou forme.
- Désignation.
- Quantité.
- Poids.
- Unité de poids.
- Pureté.
- Poids de métal fin.
- Date d'acquisition.
- Prix d'acquisition.
- Devise.
- Propriétaire juridique.
- Pays.
- Lieu de stockage.
- Notes.
- Documents.

Gérer notamment :

- Lingots.
- Lingotins.
- Pièces.
- Barres.
- Autres formes.

### Valorisation

Permettre la saisie manuelle de cours historiques des métaux.

Calculer la valeur selon :

- Quantité de métal fin.
- Cours applicable.
- Devise du cours.
- Prime éventuelle.

Utiliser les conversions exactes entre grammes, kilogrammes et onces troy.

Distinguer :

- Prix de revient.
- Valeur intrinsèque du métal.
- Prime éventuelle.
- Valeur estimée de revente.

### Mouvements

Gérer :

- Achats.
- Ventes.
- Ventes partielles.
- Transferts entre propriétaires.
- Transferts entre lieux.
- Corrections d'inventaire.

Conserver l'historique des lots et des mouvements.

Un transfert interne ne doit pas apparaître comme un gain patrimonial consolidé.

---

## 13. MODULE SOCIÉTÉS

Le logiciel doit offrir une vue simplifiée de la santé économique des sociétés.

Il ne doit pas devenir un logiciel de comptabilité générale.

### Fiche société

Prévoir :

- Nom.
- Pays.
- Activité.
- Devise.
- Propriétaires et participations.
- Autres sociétés détenues.
- Actifs détenus.
- Dettes connues.
- Comptes bancaires associés.
- Documents et notes.

### Indicateurs financiers

Permettre de saisir périodiquement :

- Chiffre d'affaires.
- Charges d'exploitation.
- Résultat d'exploitation.
- Résultat net si connu.
- Trésorerie indicative.
- Dettes.
- Créances importantes.
- Dividendes distribués.
- Estimation de valeur globale, si pertinente.
- Notes de gestion.

Fréquences possibles :

- Mensuelle.
- Trimestrielle.
- Annuelle.

### Tableau de bord société

Afficher :

- Évolution du chiffre d'affaires.
- Évolution des résultats.
- Trésorerie.
- Endettement.
- Actifs détenus.
- Participations.
- Indicateurs de tendance.

Ne pas comptabiliser deux fois une trésorerie déjà représentée par les comptes bancaires de la société.

Distinguer les indicateurs de pilotage des valeurs directement retenues dans la consolidation patrimoniale.

Ne pas développer en V1 :

- Comptabilité générale complète.
- Plan comptable.
- Gestion complète de la TVA.
- Facturation.
- Paie.
- Déclarations fiscales.

---

## 14. MODULE DETTES ET FINANCEMENTS

Créer un module transversal de gestion des passifs.

Prendre en charge :

- Emprunts immobiliers.
- Prêts entre membres de la famille.
- Prêts entre sociétés.
- Prêts entre société et personne physique.
- Autres dettes patrimoniales.

Chaque financement doit comporter :

- Créancier.
- Débiteur.
- Date.
- Devise.
- Montant initial.
- Taux d'intérêt éventuel.
- Échéancier éventuel.
- Remboursements.
- Capital restant dû.
- Documents.
- Notes.

Séparer :

- Principal remboursé.
- Intérêts.
- Frais.

### Consolidation familiale

Un prêt entre deux membres de la famille peut être enregistré comme :

- Créance pour le premier.
- Dette pour le second.

Dans la vue familiale consolidée, ces positions réciproques doivent être neutralisées pour éviter une distorsion du patrimoine familial.

La même logique s'applique aux opérations internes entre sociétés consolidées, selon le périmètre retenu.

Les garanties et cautions doivent être suivies séparément des dettes certaines afin d'éviter leur comptabilisation systématique comme des passifs existants.

---

## 15. TRANSFERTS ENTRE MEMBRES DE LA FAMILLE

Prévoir la gestion des transferts patrimoniaux entre membres.

Types d'opérations :

- Don.
- Transfert de liquidités.
- Cession d'actif.
- Cession de parts sociales.
- Transfert de métal précieux.
- Prêt familial.
- Remboursement de prêt.

Chaque opération doit comporter :

- Émetteur.
- Bénéficiaire.
- Actif ou montant concerné.
- Date.
- Nature de l'opération.
- Valeur.
- Devise.
- Documents et notes.

Un transfert entre membres de la famille ne représente pas nécessairement une modification du patrimoine global familial.

Le logiciel doit distinguer :

- Variation du patrimoine individuel.
- Variation du patrimoine familial.
- Variation de valorisation.

Ne pas calculer automatiquement la fiscalité d'une donation ou succession.

---

## 16. HISTORIQUE PATRIMONIAL

Cette fonctionnalité est obligatoire dès la V1.

L'utilisateur doit pouvoir sélectionner une date et consulter la situation du patrimoine à cette date.

### Historiser

- Les valorisations immobilières.
- Les positions en métaux.
- Les cours des métaux.
- Les taux de change.
- Les transactions bancaires.
- Les liquidités en espèces.
- Les dettes.
- Les remboursements.
- Les contrats locatifs.
- Les loyers.
- Les participations dans les sociétés.
- Les transferts entre membres.
- Les acquisitions et cessions.
- L'appartenance au périmètre familial.

### Règles de calcul

Le patrimoine historique doit être reconstruit à partir des enregistrements datés.

Distinguer la date d'effet économique et la date de saisie ou correction.

Ne pas se contenter de mémoriser uniquement le dernier solde ou la dernière estimation.

Conserver la traçabilité des corrections rétroactives.

Définir les règles de valorisation à une date donnée.

Ne pas inventer une situation historique complète lorsqu'il manque des données nécessaires.

### Graphiques

Prévoir :

- Patrimoine net familial dans le temps.
- Patrimoine individuel dans le temps.
- Évolution de l'immobilier.
- Évolution des liquidités.
- Évolution des métaux précieux.
- Évolution par pays.
- Évolution par devise.

Permettre des vues mensuelles et annuelles.

Ne pas présenter un apport de capital comme une performance d'investissement.

Identifier les variations provenant des flux, des changements de valorisation, des taux de change et des modifications de périmètre.

---

## 17. TABLEAU DE BORD GÉNÉRAL

Créer un tableau de bord visuellement clair, professionnel et synthétique.

### Indicateurs principaux

- Patrimoine brut.
- Total des dettes.
- Patrimoine net.
- Variation sur la période.
- Immobilier.
- Liquidités bancaires.
- Espèces.
- Or.
- Argent métal.
- Participations dans les sociétés.
- Revenus locatifs.
- Impayés.

Tous les montants consolidés doivent être exprimés en EUR.

### Visualisations

Prévoir :

- Courbe d'évolution du patrimoine.
- Répartition par type d'actifs.
- Répartition par pays.
- Répartition par membre.
- Répartition par devise.
- Vue des principaux actifs.
- Alertes importantes.

### Filtres

L'utilisateur doit pouvoir filtrer par :

- Famille ou membre.
- Date.
- Période.
- Pays.
- Devise.
- Entité propriétaire.
- Type d'actif.

### Qualité des données

Afficher des alertes en cas de :

- Taux de change manquant.
- Valorisation ancienne.
- Dette dont le solde est inconnu.
- Incohérence de pourcentages.
- Paiement locatif en retard.
- Actif sans propriétaire défini.
- Données insuffisantes pour une consolidation complète.

Ne pas présenter un total apparemment exact si des actifs importants ne peuvent pas être valorisés.

---

## 18. GESTION DES DOCUMENTS

Créer un gestionnaire documentaire local.

Permettre d'attacher des fichiers aux :

- Membres de la famille.
- Sociétés.
- Biens immobiliers.
- Baux.
- Transactions.
- Dettes.
- Lots de métaux précieux.
- Autres actifs.

Formats courants acceptés :

- PDF.
- JPG.
- PNG.
- DOCX.
- ODT.
- XLSX.
- ODS.
- Autres fichiers courants.

### Fonctions

- Ajouter un fichier.
- Consulter sa description.
- Ouvrir le fichier.
- Exporter une copie.
- Associer un fichier à plusieurs objets.
- Supprimer une association.
- Supprimer un document avec confirmation.

Conserver les métadonnées :

- Identifiant.
- Nom original.
- Type.
- Taille.
- Date d'ajout.
- Empreinte de contrôle.
- Objet(s) associé(s).

Les documents doivent être copiés ou importés dans un espace contrôlé par l'application afin que leur disponibilité ne dépende pas du maintien d'un chemin externe.

Conserver les fichiers dans leurs formats originaux.

Éviter les doublons physiques lorsque cela est pertinent.

Une sauvegarde complète doit pouvoir récupérer tous les documents.

---

## 19. EXPORT, SAUVEGARDE ET RESTAURATION

**C'EST UNE EXIGENCE ABSOLUMENT PRIORITAIRE.**

L'application ne doit jamais enfermer les données dans un format propriétaire ou difficile à récupérer.

Créer une section principale intitulée « Sauvegarde et export ».

### 19.1 Export XLSX

Permettre de générer un classeur Excel complet, lisible dans LibreOffice Calc.

Prévoir des feuilles distinctes pour :

- Famille et membres.
- Entités juridiques.
- Participations.
- Propriétés des actifs.
- Immobilier.
- Valorisation immobilière.
- Locataires.
- Baux.
- Échéances locatives.
- Paiements.
- Dépenses.
- Comptes bancaires.
- Caisses.
- Transactions.
- Métaux précieux.
- Cours des métaux.
- Dettes.
- Remboursements.
- Taux de change.
- Indicateurs des sociétés.
- Métadonnées des documents.
- Associations documentaires.
- Autres données métier.

Conserver les identifiants nécessaires à la compréhension des relations.

Documenter les unités, les devises et les dates.

Le XLSX est prioritairement un format de consultation et d'exploitation bureautique. L'archive CSV structurée reste le format principal de réimportation tabulaire.

### 19.2 Export CSV intégral

Générer une archive ZIP contenant les tables métier en fichiers CSV standard.

Utiliser :

- UTF-8.
- Dates ISO 8601.
- Montants décimaux sans perte de précision.
- Identifiants stables.
- Références explicites entre tables.
- En-têtes de colonnes documentés.

Inclure un manifeste contenant :

- Version du schéma.
- Version de l'export.
- Date de génération.
- Liste des fichiers.
- Dictionnaire des données.
- Description des relations.
- Convention des montants et devises.

L'export doit pouvoir être réimporté dans une base vierge.

Les relations entre membres, sociétés, actifs et propriétaires doivent être conservées.

### 19.3 Sauvegarde complète

Permettre de produire en un clic une archive ZIP contenant :

- Une sauvegarde cohérente de la base SQLite.
- Tous les documents stockés dans l'application.
- Un manifeste.
- La version du schéma.
- La version de l'application.
- Les sommes de contrôle permettant de vérifier les fichiers.

Ne pas simplement copier une base SQLite ouverte sans employer un mécanisme de sauvegarde adapté.

S'assurer que les fichiers documentaires et la base correspondent à un état cohérent.

### 19.4 Restauration intégrale

Permettre de sélectionner une sauvegarde et de restaurer son contenu.

Avant remplacement :

1. Vérifier le format.
2. Vérifier l'intégrité.
3. Vérifier la compatibilité de version.
4. Présenter un résumé du contenu.
5. Proposer une sauvegarde de l'état actuel.
6. Demander confirmation.

Une restauration interrompue ne doit pas détruire les données existantes.

Prévoir un mécanisme de restauration sécurisé, utilisant une zone temporaire de validation avant activation.

### 19.5 Réimportation CSV

Prévoir une fonctionnalité distincte pour réimporter l'archive CSV structurée.

Gérer :

- Validation des fichiers.
- Cohérence des identifiants.
- Relations entre tables.
- Compatibilité des versions.
- Contrôle des doublons.
- Valeurs manquantes.
- Erreurs de format.

Documenter clairement les différences entre réimportation structurée et restauration complète SQLite.

### 19.6 Récupération indépendante

L'utilisateur doit pouvoir récupérer ses données sans disposer de l'application.

La documentation doit expliquer comment accéder :

- Aux fichiers SQLite.
- Aux fichiers CSV.
- Aux classeurs XLSX.
- Aux documents originaux.

Ne pas utiliser de mécanisme propriétaire indispensable à la récupération.

Si un chiffrement optionnel est introduit, fournir un format standard, documenter son déchiffrement et ne pas enfermer les données derrière un service externe.

### 19.7 Tests obligatoires de sauvegarde

Créer une base de test contenant :

- Plusieurs membres.
- Des sociétés familiales.
- Des participations indirectes.
- Des biens immobiliers.
- Des baux.
- Des paiements.
- Des comptes multidevises.
- Des dettes.
- Des métaux précieux.
- Des documents.
- Des taux historiques.

Exporter toutes les données.

Créer un environnement vierge.

Restaurer les données.

Comparer :

- Nombre d'enregistrements.
- Identifiants.
- Relations.
- Montants.
- Devises.
- Pourcentages.
- Historique.
- Indicateurs calculés.
- Nombre et empreintes des documents.

La restauration doit reconstituer des résultats équivalents à ceux de la base d'origine.

Tester séparément la sauvegarde SQLite et la réimportation CSV.

---

## 20. SÉCURITÉ ET CONFIDENTIALITÉ

L'application gère des informations patrimoniales sensibles.

Ne pas envoyer de données à des services externes sans demande explicite.

Éviter les services de télémétrie.

Ne pas intégrer de mots de passe ou secrets dans le code source.

Prévoir :

- Validation stricte des données.
- Transactions atomiques.
- Contraintes de clés étrangères.
- Journal d'erreurs.
- Historique des modifications importantes.
- Confirmations avant suppression.
- Sauvegardes avant migrations risquées.
- Protection contre les injections SQL.
- Contrôle des fichiers importés.
- Prévention des extractions d'archives hors du dossier autorisé.

Ne pas développer un algorithme de chiffrement maison.

Si un mécanisme de protection est mis en œuvre, utiliser des bibliothèques éprouvées.

L'application étant locale et administrée par une personne principale, ne pas développer dès la V1 une architecture complexe d'authentification multi-utilisateur.

---

## 21. PRÉCISION DES CALCULS FINANCIERS

Les calculs monétaires doivent utiliser des représentations décimales exactes.

Ne pas employer les nombres flottants binaires ordinaires pour calculer les soldes financiers sensibles.

Définir explicitement :

- Les unités.
- Les conventions de signe.
- Les règles d'arrondi.
- La précision monétaire.
- Les conversions de devises.
- Les dates applicables.
- Les règles de consolidation.
- Le traitement des valeurs manquantes.

La base SQLite doit stocker les valeurs monétaires sous une forme compatible avec la précision retenue, par exemple des représentations décimales textuelles validées et manipulées côté Rust.

Les quantités de métaux doivent également conserver une précision suffisante.

Les calculs métier sensibles doivent disposer de tests automatisés comportant des valeurs attendues.

---

## 22. STRUCTURE DE L'INTERFACE

Créer une interface desktop professionnelle, moderne et entièrement en français.

Organisation proposée :

**Navigation principale :**

1. Tableau de bord.
2. Famille.
3. Patrimoine.
4. Immobilier et loyers.
5. Comptes bancaires.
6. Espèces et devises.
7. Or et argent.
8. Sociétés.
9. Dettes et financements.
10. Documents.
11. Rapports.
12. Sauvegarde et export.
13. Paramètres.

### Ergonomie

Prévoir :

- Navigation latérale.
- Sélecteur de contexte familial ou individuel.
- Recherche globale.
- Tableaux triables.
- Filtres.
- Formulaires cohérents.
- Graphiques interactifs.
- Pages de détail des actifs.
- Historique des opérations.
- Sélecteur de date.
- Affichage multidevise.
- Messages d'erreur explicites.
- Confirmations des opérations destructrices.

Les écrans vides doivent proposer une action utile pour créer les premières données.

Ne pas laisser de boutons fictifs.

Éviter les graphiques reposant sur des données de démonstration lorsque les modules sont connectés à la base réelle.

Privilégier une interface sobre, similaire à un outil professionnel de gestion financière.

---

## 23. MODÈLE DE DONNÉES

Avant d'écrire le code applicatif, proposer un modèle relationnel détaillé.

Il doit couvrir au minimum les concepts suivants :

### Entités et famille

- Personnes physiques.
- Membres de la famille.
- Appartenance au périmètre familial.
- Sociétés.
- Entités juridiques.
- Participations datées.

### Actifs

- Actifs génériques.
- Relations de propriété.
- Historique de valorisation.
- Documents associés.

### Immobilier

- Biens.
- Unités locatives.
- Locataires.
- Baux.
- Échéances.
- Paiements.
- Dépenses.
- Historique de valorisation.

### Trésorerie

- Comptes bancaires.
- Caisses physiques.
- Transactions.
- Transferts.
- Conversions.

### Métaux

- Lots.
- Mouvements.
- Cotations historiques.

### Sociétés

- Indicateurs périodiques.
- Participations.
- Distributions.

### Financements

- Dettes.
- Créances.
- Échéances.
- Remboursements.
- Garanties éventuelles.

### Données communes

- Devises.
- Taux de change.
- Documents.
- Associations documentaires.
- Historique des modifications.
- Métadonnées de sauvegarde.

Ne pas créer artificiellement une table indépendante pour chaque membre.

Ne pas utiliser un simple champ texte « propriétaire » pour représenter des structures de détention complexes.

Prévoir les contraintes, index, clés étrangères et migrations.

Éviter de dupliquer des données pouvant être dérivées de manière fiable.

Les soldes calculés et les données de synthèse doivent être cohérents avec les opérations sources.

---

## 24. TESTS MÉTIER

Créer des tests automatisés couvrant notamment :

### Famille

- Une famille de trois membres.
- Un membre possédant seul un actif.
- Un bien commun à deux membres.
- Un bien partagé avec une personne extérieure.
- Une société appartenant à plusieurs membres.
- Un prêt entre deux membres.
- Un transfert de liquidités familial.
- Un changement historique du périmètre familial.

### Sociétés et propriété

- Une société détenue à 100 %.
- Une participation minoritaire.
- Une chaîne de participations sur trois niveaux.
- Plusieurs chemins indirects vers une même société.
- Un immeuble détenu par une société étrangère.
- Une société possédant de la trésorerie et un immeuble.
- L'absence de double comptabilisation.
- Une structure de détention invalide.

### Immobilier

- Acquisition.
- Valorisation.
- Loyer attendu.
- Paiement partiel.
- Paiement tardif.
- Impayé.
- Vacance.
- Vente du bien.
- Emprunt et remboursement.

### Trésorerie

- Solde initial.
- Entrée et sortie.
- Transfert entre comptes.
- Transfert entre membres.
- Conversion multidevise avec frais.
- Rapprochement bancaire.

### Métaux

- Achat d'or.
- Achat d'argent.
- Pureté inférieure à 100 %.
- Vente partielle.
- Conversion des unités de poids.
- Modification des cours.

### Historique

- Consultation à une date passée.
- Changement de propriétaire.
- Changement de participation.
- Taux de change historique.
- Absence de taux.
- Correction rétroactive.

### Portabilité

- Export CSV complet.
- Export XLSX.
- Sauvegarde SQLite.
- Restauration de documents.
- Réimportation CSV.
- Vérification des relations.
- Comparaison des résultats financiers après restauration.

Les tests doivent vérifier les calculs attendus, et non uniquement le bon fonctionnement des interfaces.

---

## 25. STRATÉGIE DE DÉVELOPPEMENT

Le projet doit être réalisé progressivement.

Ne cherche pas à développer l'intégralité de l'application en une seule génération.

### PHASE 1 — CONCEPTION ET ARCHITECTURE

Produire :

- Architecture technique.
- Organisation du dépôt.
- Modèle de données.
- Règles de propriété.
- Moteur de consolidation individuelle et familiale.
- Règles de valorisation.
- Gestion des devises.
- Règles d'historisation.
- Stratégie documentaire.
- Stratégie de sauvegarde.
- Scénarios de tests.

Identifier les décisions techniques importantes avant d'écrire le code.

### PHASE 2 — SOCLE APPLICATIF

Créer :

- Projet Tauri.
- Interface React/TypeScript.
- Backend Rust.
- Base SQLite.
- Migrations versionnées.
- Navigation.
- Composants communs.
- Gestion des paramètres.
- Premier mécanisme de sauvegarde/restauration.

Le projet doit être exécutable sur Linux.

### PHASE 3 — FAMILLE, ENTITÉS ET PROPRIÉTÉ

Développer :

- Membres de la famille.
- Personnes extérieures.
- Sociétés.
- Participations.
- Relations de propriété.
- Moteur de consolidation.
- Tests de non-double-comptabilisation.

Cette phase doit être validée avant de développer les tableaux de bord financiers.

### PHASE 4 — DEVISES ET HISTORIQUE

Développer :

- Référentiel de devises.
- Saisie manuelle des taux.
- Historisation.
- Conversions financières.
- Gestion des dates de référence.
- Tests de précision.

### PHASE 5 — IMMOBILIER ET LOYERS

Développer :

- Biens immobiliers.
- Propriétaires.
- Valorisation.
- Unités locatives.
- Baux.
- Échéances.
- Paiements.
- Charges.
- Indicateurs.

### PHASE 6 — LIQUIDITÉS ET DETTES

Développer :

- Comptes bancaires.
- Caisses physiques.
- Transactions.
- Transferts.
- Conversions.
- Dettes.
- Remboursements.
- Prêts intrafamiliaux.

### PHASE 7 — MÉTAUX ET SOCIÉTÉS

Développer :

- Inventaire d'or et d'argent.
- Cotations manuelles.
- Mouvements.
- Valorisation.
- Indicateurs simplifiés des sociétés.

### PHASE 8 — DOCUMENTS ET PORTABILITÉ COMPLÈTE

Développer :

- Gestionnaire documentaire.
- Export XLSX.
- Export CSV complet.
- Sauvegardes avec documents.
- Restauration intégrale.
- Réimportation CSV.
- Tests automatisés de récupération.

Le mécanisme minimal de sauvegarde existe déjà depuis la phase 2 ; cette phase réalise et valide l'ensemble des fonctions avancées de portabilité.

### PHASE 9 — TABLEAUX DE BORD ET RAPPORTS

Développer :

- Tableau de bord familial.
- Tableaux de bord individuels.
- Tableau de bord des sociétés.
- Graphiques historiques.
- Répartitions patrimoniales.
- Rapports.
- Alertes.

Les graphiques doivent utiliser exclusivement les calculs métier réellement implémentés et testés.

### PHASE 10 — FINALISATION ET LIVRAISON

Effectuer :

- Tests complets.
- Vérification des calculs.
- Tests de restauration.
- Contrôle des performances.
- Corrections d'interface.
- Documentation utilisateur.
- Documentation technique.
- Construction AppImage.
- Test d'installation et de mise à jour.

---

## 26. EXIGENCES DE QUALITÉ POUR CODEX

À chaque étape :

1. Proposer un périmètre de travail précis.
2. Utiliser une architecture claire et modulaire.
3. Produire du code compilable.
4. Exécuter les tests disponibles.
5. Corriger les erreurs identifiées.
6. Documenter les décisions.
7. Conserver les migrations nécessaires.
8. Éviter les changements destructeurs.
9. Ne pas inventer des données financières.
10. Ne pas déclarer une fonctionnalité terminée si elle ne fonctionne pas réellement.

Ne pas générer de code inutilement complexe.

Ne pas multiplier les dépendances sans justification.

Ne pas réécrire toute l'application lors de chaque modification.

Maintenir un fichier documentant les décisions d'architecture et un autre indiquant les fonctionnalités terminées et restant à développer.

En cas d'ambiguïté métier susceptible de fausser les calculs, la signaler explicitement.

Privilégier une implémentation simple et vérifiable plutôt qu'une abstraction excessive.

Le logiciel doit rester utilisable et évolutif pendant plusieurs années.

---

## 27. CRITÈRES DE RÉUSSITE DU PROJET

L'application sera considérée comme fonctionnellement aboutie lorsque l'utilisateur pourra :

1. Créer plusieurs membres de sa famille.
2. Enregistrer des sociétés et des participations indirectes.
3. Attribuer les actifs aux propriétaires corrects.
4. Consulter un patrimoine individuel.
5. Consulter un patrimoine familial consolidé.
6. Gérer des biens immobiliers et leurs loyers.
7. Suivre plusieurs comptes bancaires et caisses en différentes devises.
8. Gérer les positions en or et en argent.
9. Suivre les dettes.
10. Consulter les indicateurs de sociétés.
11. Ajouter des documents.
12. Saisir des taux de change manuellement.
13. Consulter l'évolution patrimoniale historique.
14. Exporter les données au format XLSX et CSV.
15. Réaliser une sauvegarde complète.
16. Restaurer ses données et documents.
17. Retrouver exactement les mêmes résultats patrimoniaux après restauration.
18. Utiliser le logiciel sur Linux sans dépendance à un service externe.

---

## 28. PREMIÈRE ACTION DEMANDÉE À CODEX

Commence par lire intégralement ce cahier des charges.

N'essaie pas de créer toute l'application immédiatement.

**Exécute uniquement la PHASE 1 : conception et architecture.**

Je souhaite recevoir dans ta première réponse :

1. Une analyse structurée du besoin.
2. Une proposition d'architecture technique.
3. Une arborescence du projet.
4. Un modèle relationnel détaillé de la base SQLite.
5. Un schéma des relations entre personnes, sociétés et actifs.
6. Une proposition de moteur de consolidation familiale et individuelle.
7. Les règles précises de calcul des montants patrimoniaux.
8. La méthode de gestion de l'historique.
9. La méthode de gestion des taux de change manuels.
10. L'architecture de stockage des documents.
11. La stratégie complète d'export et de restauration.
12. Une liste des risques techniques et métier.
13. Une liste de tests prioritaires.
14. Un plan d'implémentation décomposé en tâches exécutables.
15. Les éventuelles questions de conception réellement bloquantes.

Tu peux formuler des hypothèses raisonnables pour les points non bloquants, à condition de les documenter.

Ne produis pas immédiatement des milliers de lignes de code.

La priorité est de construire des fondations fiables afin que le logiciel puisse évoluer sans perte de données ni erreurs dans les calculs patrimoniaux.

**Objectif final : une application Linux locale, fiable, indépendante, durable, permettant de piloter un patrimoine familial international complexe avec une récupération intégrale des données à tout moment.**