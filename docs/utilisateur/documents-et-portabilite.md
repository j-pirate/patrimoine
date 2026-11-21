# Documents, sauvegardes et exports

Guide du **12 novembre 2026, Europe/Paris**. Les exemples sont synthétiques. Les [preuves de validation](../historique/validation-phase-8.md) distinguent les tests de format, de persistance, d’interface et les deux parcours de récupération.

## Choisir le bon format

| Action | Contenu et usage |
|---|---|
| Sauvegarde complète ZIP | Base SQLite autonome, documents originaux, dictionnaire et empreintes ; restauration fidèle du coffre |
| Archive CSV intégrale ZIP | Toutes les tables, leurs historiques et relations, les originaux et un dictionnaire ; reconstruction dans une base vierge |
| Classeur XLSX complet | Consultation dans un tableur des tables, relations et métadonnées documentaires ; les originaux restent dans les ZIP |
| Export d’une copie documentaire | Octets de la version choisie, sans conversion de format |

Les opérations de **Sauvegarde et export** portent toujours sur l’intégralité du coffre actuel et de son histoire. Elles ne sont pas limitées au membre, à la date, aux connaissances K ou aux filtres de la vue affichée. La bibliothèque **Documents** se consulte en revanche selon les connaissances K sélectionnées, et les périodes d’association utilisent la date économique D.

## Importer et retrouver un original

Ouvrez **Outils et réglages → Documents**. Vous pouvez ouvrir **Ajouter une description à l’import** pour renseigner le champ facultatif avant de choisir **Importer un document**. Patrimoine copie le fichier dans le coffre. Vous pourrez le retrouver même si le fichier source a été déplacé.

La bibliothèque présente les documents connus à K. Utiliser la recherche dans le nom, la description ou le type ; les filtres de statut, de type d’objet et d’objet associé permettent de retrouver les pièces. Le sélecteur global de membre ne masque pas automatiquement une partie de cette bibliothèque. Les filtres d’association ne retiennent que les liens applicables à D.

Chaque fiche expose le nom original, le type, la taille, la description, le statut, la date d’ajout, les identifiants et le SHA-256. Deux documents logiques peuvent partager les mêmes octets ; le contenu physique est dédupliqué sans fusionner leurs descriptions ni leurs associations.

**Exporter une copie** produit les octets de la version affichée. **Ouvrir le document** prépare une copie privée vérifiée et demande son ouverture au lecteur associé du système Linux. Si le lecteur associé échoue, l’application affiche un message et l’export manuel reste disponible. Les copies privées de consultation sont nettoyées après sept jours, à l’ouverture du coffre ou lors d’une nouvelle consultation. Les autres types de fichiers restent conservables et exportables ; l’application ne transforme pas un original en un autre format.

## Description, remplacement et classement

**Modifier la description ou le statut** conserve une nouvelle révision avec son motif. Une description peut être vide ; elle est limitée à 10 000 octets. Les statuts sont **Actif**, **Archivé** et **Retiré du classement**. Pour retrouver une pièce archivée ou retirée, choisir ce statut dans le filtre ou **Tous les statuts**.

**Remplacer l’original** demande un motif puis le choix du nouveau fichier. Le document conserve son identité et ses associations ; une nouvelle version porte les nouveaux octets et leur empreinte. Les anciennes versions et leurs contenus restent disponibles dans **Versions antérieures de ce document**, avec **Ouvrir cette version** et **Exporter cette version**.

**Supprimer le document** ouvre une confirmation de retrait du classement courant. Cocher **Confirmer le retrait du document du classement courant**, saisir le motif et enregistrer. Cette commande ne purge pas les originaux historiques. **Rétablir le document** lui rend le statut actif ; une pièce simplement archivée se réactive par la modification de statut.

Une correction exige la révision qui était affichée lors de la saisie. Si une autre modification l’a remplacée, le refus conserve la saisie ; recharger la bibliothèque et vérifier la nouvelle source avant de reprendre. Annuler un dialogue de fichier conserve également le formulaire.

## Associer une pièce à plusieurs objets

Depuis une fiche, choisir **Associer un objet**. Sélectionner l’objet métier : personne ou société, actif, bail, opération bancaire, financement, lot de métal ou autre source proposée. Le même document peut avoir plusieurs associations sans copie physique supplémentaire.

Le **Début de l’association** est obligatoire et reprend initialement D. La fin est facultative et exclue : une association du 1er janvier au 1er avril s’applique jusqu’au 31 mars. Le rôle est facultatif. Saisir le motif puis **Enregistrer l’association documentaire**.

**Modifier l’association** corrige son rôle ou sa période. Pour changer d’objet, retirer le lien puis en créer un nouveau. **Retirer l’association** exige son motif et sa confirmation ; le document et ses autres liens restent conservés, et l’ancien lien reste reconstituable à K.

Une cible retirée ultérieurement peut encore apparaître sur une ancienne association avec l’indication **retiré**. La création d’un nouveau lien exige une cible disponible ; l’historique n’est pas réécrit pour faire disparaître les liens antérieurs.

Dans **Outils et réglages → Historique**, les révisions documentaires et les associations donnent accès à leurs séquences de connaissance. Toute séquence K fixe, y compris `0`, rend les écritures documentaires indisponibles. Vous pouvez ouvrir et exporter les versions connues. Pour restaurer le coffre, utilisez **Sauvegarde et export**.

## Sauvegarder et restaurer SQLite

Dans **Sauvegarde et export**, choisir **Créer une sauvegarde complète**. Le ZIP contient une base SQLite cohérente, autonome sans fichier WAL, tous les originaux référencés par les versions conservées, les métadonnées, associations, dictionnaire et empreintes. Les pièces archivées ou retirées et leurs anciennes versions font partie de la récupération.

Pour restaurer, choisir **Choisir une sauvegarde à analyser**. Vérifier le résumé, puis confirmer le remplacement ou **Annuler**. L’analyse seule ne modifie pas le coffre actif. Les archives anciennes des schémas 1 à 5 sont mises à niveau dans une copie vers le schéma courant 6 ; leur source reste intacte.

La sauvegarde préalable de l’état courant est cochée par défaut. Au moment de **Confirmer le remplacement du coffre**, choisir son emplacement. Si ce choix est annulé, l’activation ne se poursuit pas. La restauration active une nouvelle génération et conserve la précédente pour récupération. Si la nouvelle génération est active mais que la finalisation du journal échoue, l’application affiche les données actives et indique la reprise à terminer. Fermer puis rouvrir Patrimoine permet de retenter cette finalisation.

## Exporter et réimporter le CSV intégral

Choisir **Exporter une archive CSV intégrale**. Le ZIP contient un fichier par table persistante, même vide, avec les identités, révisions, relations, réglages, sources financières, documents et audit. Les originaux figurent sous `documents/sha256/` ; leur nom logique se retrouve par les métadonnées documentaires.

Le format CSV 1 utilise UTF-8 sans BOM, virgule, guillemets doubles et fins de ligne CRLF. Les montants, dates ISO, identifiants et fractions restent exacts. Pour une colonne nullable, son marqueur `<colonne>__is_null` distingue `1` pour NULL et `0` pour une valeur, y compris une chaîne réellement vide. Zéro, vide et NULL ne sont pas interchangeables.

Les textes commençant par `=`, `+`, `-` ou `@` restent inchangés dans cette source réimportable. Pour la consultation bureautique, utiliser le XLSX plutôt que demander au tableur d’interpréter les CSV comme des nombres ou formules. Le manifeste et `dictionary/schema.json` décrivent les colonnes, clés, relations, unités, conventions et empreintes.

La réimportation est une commande distincte : **Choisir une archive CSV à analyser**. Elle construit une base vierge dans la zone de préparation, contrôle toutes les tables et pièces, puis affiche **Vérifier avant de réimporter**. Elle ne fusionne pas deux coffres et n’ajoute pas simplement quelques lignes à l’espace courant.

Lire le résumé, garder ou ajuster le choix de sauvegarde préalable, puis **Confirmer le remplacement du coffre**, ou **Annuler**. Le remplacement utilise le même mécanisme de génération que la restauration SQLite. Si le coffre change après l’analyse, une nouvelle analyse est requise.

Les identités, valeurs, versions et dates de saisie d’origine sont conservées. La candidate reçoit un événement technique de réimportation avec un nouveau changement et une ligne d’audit ; les anciens K restent reconstituables. Cet ajout technique ne réécrit aucun fait métier et n’est pas une opération financière.

Le premier format CSV implémenté prend en charge le schéma 6. Un format ou schéma non pris en charge est refusé ; le service n’exécute aucun SQL reçu dans l’archive. Chaque cellule est limitée à 1 Mio, chaque table à deux millions de cellules et 128 Mio, le total décompressé à 512 Mio. Un dépassement fait échouer l’opération, sans troncature ni remplacement partiel.

## Consulter le XLSX

**Exporter un classeur XLSX complet** produit un fichier de consultation, lisible dans un tableur. Les feuilles **Informations** et **Dictionnaire** donnent le contexte du snapshot, les correspondances entre noms français et tables techniques, les types, clés, relations et conventions. Les vues **Banques** et **Caisses** facilitent la lecture des comptes ; elles reprennent des données déjà présentes dans les tables sources et ne constituent pas des montants supplémentaires à additionner.

Les identifiants, dates ISO, décimaux et autres valeurs sont des cellules texte explicites. Le fichier ne crée pas de formules, macros ou liens actifs depuis les données utilisateur. Les colonnes de nullité restent disponibles pour distinguer NULL du vide.

Les tables dépassant une feuille sont réparties sur plusieurs feuilles référencées dans le dictionnaire. Les textes dépassant la capacité d’une cellule, contenant des caractères incompatibles avec XML ou un retour chariot sont référencés dans les feuilles **Textes longs**. Ce dernier cas préserve notamment les fins de ligne CRLF après un réenregistrement dans LibreOffice. Chaque fragment est une chaîne JSON : décoder les fragments et les concaténer dans leur ordre pour retrouver le texte complet. Les cellules ne sont pas tronquées silencieusement.

Le XLSX contient les métadonnées et associations des pièces, pas les octets des originaux. Il n’existe pas d’import XLSX. Conserver une sauvegarde SQLite ou une archive CSV avec originaux pour récupérer l’ensemble du coffre.

## Récupération indépendante et limites

Dans un ZIP SQLite de confiance, lire `recovery/LISEZMOI.md`, ouvrir `data/patrimoine.sqlite3` avec un outil SQLite et consulter le dictionnaire. Les montants sont des textes décimaux ; les calculs exacts demandent un outil décimal ou rationnel, sans conversion implicite en flottants.

Dans l’archive CSV, les tables et leur dictionnaire sont lisibles avec des outils standards, sans l’application. Les documents gardent leurs octets d’origine. Joindre `document_versions.blob_sha256` à `document_blobs.sha256`, retrouver `original_name` et vérifier les tailles et SHA-256 pour exporter une copie nommée à convenance.

Le script indépendant [recover-csv.py](../../scripts/recover-csv.py) reconstruit un fichier SQLite et un dossier d’originaux avec Python et SQLite standards, sans exécutable Patrimoine :

```bash
mkdir recuperation
python3 scripts/recover-csv.py patrimoine-csv.zip \
  --output recuperation/patrimoine.sqlite3 \
  --documents-dir recuperation/documents
```

Il utilise les six migrations SQL textuelles du dépôt, dont les empreintes sont épinglées dans le script ; `--schema-directory` permet de préciser leur dossier local. Il n’exécute pas le SQL décrit dans l’archive et refuse d’écraser les destinations. Les originaux sont placés sous `documents/sha256/`. Le K source et les lignes restent identiques, sans événement ajouté dans cette copie de secours externe.

Ce secours contrôle format, empreintes, types, références, intégrité SQLite et documents. Il ne réexécute pas les invariants financiers Rust : pour remettre la récupération en service dans Patrimoine, employer l’analyse et la validation applicatives. Il ne remplace pas la procédure de réimport dans l’interface.

Les empreintes détectent les différences par rapport au manifeste de l’archive. Patrimoine conserve l’historique et produit des ZIP non chiffrés. [Tableaux de bord](tableaux-de-bord.md) · [Installation et récupération Linux](installation-linux.md)
