# Installation, mise à jour et récupération Linux

**Pop!_OS 22.04 / Ubuntu 22.04 :** les paquets compatibles sont disponibles sur la [page dédiée](../../distribution/0.1.1-popos22.04/README.md). Les instructions Debian 13 ci-dessous ne s'appliquent pas à ces systèmes sans adaptation.

Guide du **13 novembre 2026, Europe/Paris**. Le [rapport de qualification](../historique/validation-phase-10.md) donne les paquets, versions, systèmes et volumes effectivement testés. Les [résultats de sécurité](../technique/revue-securite.md) distinguent les contrôles réalisés de leurs limites. Les données des essais sont synthétiques.

## Choisir le paquet

La cible de cette livraison est **Debian 13, x86_64**, avec session graphique Linux et bibliothèques d’exécution GTK 3/WebKitGTK 4.1. La qualification de cette cible ne vaut pas compatibilité avec toutes les distributions, les anciennes versions de glibc, les machines ARM ou toutes les configurations graphiques.

L’AppImage contient le programme et son interface. Node, npm, Rust, un serveur et un compte externe ne sont pas nécessaires pour l’utiliser. Les outils de développement du [guide technique](../technique/developpement-linux.md) servent uniquement à construire ou vérifier les sources. Les lecteurs de documents et outils bureautiques sont séparés : l’export d’un original ne dépend pas de la présence d’un lecteur associé.

Les commandes de ce guide utilisent le candidat `Patrimoine_0.1.1_amd64.AppImage`, accompagné de `SHA256SUMS` et de `build-manifest.json`. Le rapport de qualification identifie les artefacts effectivement construits et testés. Conserver ces trois fichiers ensemble. Le fichier d’empreintes détecte une différence avec les octets annoncés ; il ne constitue pas une signature de l’auteur. Vérifier aussi la provenance du paquet avant de l’exécuter.

Dans le dossier contenant les fichiers livrés :

```bash
sha256sum --check SHA256SUMS
```

Ne pas utiliser un paquet dont la vérification échoue. Les notices des composants tiers accompagnent le paquet ; leur [inventaire](../technique/licences-tierces.md) précise les licences et la méthode de collecte. La licence du code propre à ce dépôt n’est pas modifiée par ces notices.

## Installer pour son compte Linux

Un fichier AppImage peut être conservé dans un dossier personnel. Par exemple, depuis le dossier du paquet :

```bash
mkdir -p "$HOME/.local/opt/patrimoine"
install -m 755 Patrimoine_0.1.1_amd64.AppImage "$HOME/.local/opt/patrimoine/Patrimoine_0.1.1_amd64.AppImage"
"$HOME/.local/opt/patrimoine/Patrimoine_0.1.1_amd64.AppImage"
```

Cette copie du programme ne demande pas de privilèges administrateur. L’installation des bibliothèques système éventuellement manquantes dépend de l’administration du poste ; elle n’est pas effectuée automatiquement par Patrimoine.

Si le système ne permet pas le montage FUSE utilisé par le format AppImage, le mode d’extraction du paquet permet un lancement sans ce montage :

```bash
"$HOME/.local/opt/patrimoine/Patrimoine_0.1.1_amd64.AppImage" --appimage-extract-and-run
```

Ce mode extrait le programme pour son exécution ; il ne restaure pas un coffre. Il demande de l’espace temporaire disponible. Les conditions effectivement exercées figurent dans le rapport de qualification, y compris la distinction entre lancement monté et lancement par extraction.

Au premier lancement, le coffre est vide. Les données de démonstration des campagnes de test ne sont pas ajoutées au coffre de l’utilisateur. Renseigner la famille, les actifs et leurs droits, puis les sources financières et couvertures avant d’attendre un patrimoine net complet. Le [guide des tableaux de bord](tableaux-de-bord.md) explique les montants connus, les valeurs inconnues et les filtres.

## Où résident les données

Par défaut, l’application utilise `${XDG_DATA_HOME:-$HOME/.local/share}/patrimoine`. Une valeur XDG relative est ignorée. Le programme et les données sont donc dans des dossiers distincts : remplacer une AppImage ne remplace pas les fichiers du coffre.

Le dossier de données comprend le pointeur `CURRENT`, les générations SQLite et documentaires, les zones de préparation et les informations de reprise. Les fichiers originaux sont copiés dans le coffre ; leur conservation ne dépend plus du chemin importé. Deux documents logiques peuvent partager les mêmes octets sans fusionner leurs descriptions ou associations.

Une seule instance peut ouvrir un coffre. Fermer complètement l’application avant de copier manuellement son dossier ou de lancer un autre paquet sur ce même coffre. Un fichier `.lock` présent n’est pas, à lui seul, la preuve qu’une autre instance fonctionne ; ne pas modifier le coffre pour contourner un refus d’ouverture.

Ne pas synchroniser une base SQLite active entre postes par un dossier de fichiers partagé. Pour déplacer les données, employer une sauvegarde complète, puis l’analyser et la restaurer sur le poste destinataire. Une copie manuelle de secours doit conserver tout le dossier après fermeture, y compris les générations et documents ; copier uniquement le fichier SQLite principal d’une base ouverte peut perdre des données encore dans le WAL.

Les archives et documents ne sont pas chiffrés par l’application. Utiliser les protections du compte Linux, du disque et du support de sauvegarde adaptées à ces données.

## Préparer une mise à jour

Avant de changer de paquet :

1. Dans **Sauvegarde et export**, choisir **Créer une sauvegarde complète** et conserver le ZIP sur un autre support approprié.
2. Vérifier que le fichier a été créé, puis conserver aussi la version actuelle de l’AppImage et son empreinte.
3. Fermer Patrimoine. Vérifier le nouveau paquet, le copier sous son nom versionné et le lancer sur le même compte Linux avec le même emplacement de données.
4. Vérifier le nom de l’espace, les documents et quelques sources connues avant de poursuivre la saisie.

Le paquet ne se met pas à jour automatiquement. Garder des noms versionnés permet de choisir explicitement le programme à lancer ; un raccourci éventuel doit viser le paquet choisi.

La campagne de cette livraison a exercé deux **candidats locaux**, `0.1.0` puis `0.1.1`, pour vérifier le remplacement du programme et le retour au précédent. Elle ne prétend pas que `0.1.0` était une ancienne distribution publique. Ces deux candidats utilisent le schéma SQLite 6 ; les résultats des essais exécutés et la conservation observée figurent dans le rapport.

## Revenir à la version précédente

Fermer la version courante avant de relancer la précédente. Le retour direct sur le même coffre n’est possible que si cette version sait lire son schéma et ses formats. L’application refuse un schéma trop récent plutôt que de réinitialiser silencieusement les données.

Une future migration peut rendre le retour direct incompatible. Dans ce cas, garder le coffre récent intact et restaurer, dans un emplacement séparé, une sauvegarde antérieure compatible avec l’ancien programme. Ne pas remplacer quelques tables ni modifier manuellement le numéro de schéma pour forcer l’ouverture. Le retour au paquet précédent et la restauration d’une sauvegarde sont deux opérations différentes.

Pour ouvrir explicitement un coffre séparé, le programme accepte `PATRIMOINE_DATA_DIR` avec un chemin absolu. Exemple à adapter à un dossier personnel durable et neuf :

```bash
PATRIMOINE_DATA_DIR=/chemin/absolu/coffre-recuperation ./Patrimoine_0.1.1_amd64.AppImage
```

Cette option permet un essai de récupération sans sélectionner le coffre habituel. Ne pas utiliser un dossier temporaire pour y conserver les seules données réelles.

## Restaurer avec l’application

La voie habituelle est l’analyse d’une archive depuis **Sauvegarde et export** :

- **Choisir une sauvegarde à analyser** pour un ZIP SQLite complet.
- **Choisir une archive CSV à analyser** pour le CSV intégral avec ses originaux.

Vérifier le résumé, le schéma, les compteurs, le nom de l’espace et la date de l’archive. L’analyse seule et son annulation laissent le coffre actif intact. La confirmation remplace l’espace entier ; elle ne fusionne pas deux familles ni quelques lignes sélectionnées. La sauvegarde préalable de l’état courant est cochée par défaut.

Après confirmation, l’application active une nouvelle génération et conserve la précédente. Une interruption de cette opération est traitée par la reprise prévue à l’ouverture ; ne pas éditer `CURRENT` ou le journal d’activation pour choisir arbitrairement un mélange de fichiers. Les anciens coffres et archives SQLite des schémas 1 à 5 sont migrés dans une copie vers le schéma 6, sans modifier leur source. Le premier format CSV pris en charge concerne le schéma 6.

Une archive incomplète, altérée, trop volumineuse ou d’un format futur est refusée. Conserver sa source et le message exact pour le diagnostic. Ne pas retirer des originaux ou corriger le manifeste à la main afin de forcer l’activation.

### Limites des archives de cette version

Les sauvegardes SQLite et archives CSV restent bornées pendant l’écriture et la lecture :

| Élément | Limite |
|---|---|
| Entrées du ZIP, manifeste et fichiers annexes compris | 11 000 |
| Manifeste | 4 Mio |
| Base SQLite canonique `data/patrimoine.sqlite3` d’une sauvegarde | 256 Mio |
| Un autre fichier, notamment un original ou une table CSV | 128 Mio |
| Total décompressé | 512 Mio |
| Table CSV | Deux millions de cellules |
| Cellule CSV | 1 Mio |

La capacité de 256 Mio concerne uniquement l’entrée SQLite canonique ; elle ne relève pas la taille autorisée d’un document. Les fichiers de dictionnaire, de notice et de table comptent dans le nombre d’entrées. Dépasser une de ces bornes provoque un refus sans troncature ni remplacement partiel. Un ancien lecteur peut avoir un budget plus faible et refuser une archive volumineuse même si son numéro de schéma est identique ; conserver une sauvegarde compatible avec le programme utilisé pour la récupération.

## Lire une sauvegarde SQLite sans Patrimoine

Une sauvegarde complète contient `data/patrimoine.sqlite3`, les originaux sous `documents/`, le manifeste, le dictionnaire et `recovery/LISEZMOI.md`. Sur une copie de confiance extraite dans un dossier neuf, ouvrir la base avec un outil SQLite standard, de préférence en lecture seule.

```bash
sqlite3 -readonly recuperation/data/patrimoine.sqlite3 \
  'PRAGMA integrity_check; PRAGMA foreign_key_check;'
```

Le premier contrôle doit répondre `ok` ; le contrôle des clés étrangères ne doit retourner aucune ligne. Ils ne remplacent pas la validation applicative des règles financières. Les montants restent des textes décimaux et les droits des fractions : les convertir implicitement en flottants peut perdre de la précision.

Pour retrouver un original, joindre `document_versions.blob_sha256` à `document_blobs.sha256`. Le nom logique se trouve dans `original_name`, le contenu physique dans le chemin documentaire référencé. Vérifier tailles et SHA-256 avant d’exporter une copie nommée à convenance. Une suppression documentaire est logique : les versions historiques et leurs originaux restent dans les archives.

## Reconstruire le CSV sans binaire applicatif

Le [kit autonome](../../.local/releases/0.1.1/Patrimoine_0.1.1_recuperation.zip) contient le lecteur et ses six migrations, sans nécessiter le binaire ou un accès réseau. Le dépôt fournit également [recover-csv.py](../../scripts/recover-csv.py), utilisable avec Python 3.10 ou plus récent et SQLite 3.37 ou plus récent. Aucun paquet Python externe n’est nécessaire. Depuis une copie du dépôt correspondant au format de l’archive :

```bash
mkdir recuperation
python3 scripts/recover-csv.py patrimoine-csv.zip \
  --output recuperation/patrimoine.sqlite3 \
  --documents-dir recuperation/documents
```

Les destinations doivent être inexistantes. Le script utilise les six migrations SQL locales dont les empreintes sont épinglées ; `--schema-directory` permet d’en préciser le dossier. Il n’exécute aucun SQL reçu de l’archive. Il conserve toutes les lignes et le K source, sans ajouter l’événement technique de réimportation destiné au coffre applicatif.

Les documents extraits sont placés sous `sha256/` dans le dossier choisi. Les chemins enregistrés dans la base gardent leur préfixe `documents/` ; le retirer pour les résoudre depuis `--documents-dir`. Format, empreintes, types, nullité, références, intégrité SQLite et originaux sont contrôlés. Les invariants financiers Rust ne sont pas réexécutés par ce secours externe : utiliser l’analyse et le réimport applicatifs pour remettre les données en service dans Patrimoine.

Le [guide des documents et formats](documents-et-portabilite.md) détaille CSV, XLSX, versions et associations. Le XLSX et le rapport JSON filtré servent à la consultation ; ils ne remplacent pas une archive récupérable avec ses originaux.

## Volumes, accessibilité et diagnostic

Le [rapport de qualification](../historique/validation-phase-10.md) donne les temps et mémoires observés, les caractéristiques de la machine, les tailles de fixtures et les cas interrompus ou non exécutés. La cible de consultation inférieure à deux secondes du plan était provisoire ; elle ne constitue pas une garantie universelle de temps de réponse. Les limites de formats ou de budget sont refusées explicitement, sans supprimer une partie du coffre pour accélérer un calcul.

Sur le grand profil testé, cette cible de deux secondes n’est atteinte par aucune des dix consultations mesurées. Le tableau de bord dépasse son budget de 120 secondes : 14 opérations sur 15 ont abouti. Les deux récupérations complètes réussissent, mais ces résultats ne qualifient pas une utilisation fluide de l’interface à ce volume.

Le profil volumineux de qualification prépare 100 personnes, 100 sociétés, 10 000 actifs, 100 000 opérations de journal et 10 000 originaux distincts. Sa profondeur historique est de deux changements : ce volume d’opérations ne représente pas 100 000 commits successifs. Les mesures distinguent l’ouverture et sa validation des lectures de service ; elles ne mesurent pas à elles seules la latence de l’interface graphique.

Les contrôles d’accessibilité automatiques, les parcours clavier et les essais WebKit couvrent les écrans et scénarios consignés dans le rapport. Leur réussite ne constitue pas une certification complète d’accessibilité pour toute combinaison de lecteur d’écran et de poste Linux.

Pour signaler un problème, conserver la version du paquet, la distribution et l’architecture, l’action effectuée, le message exact et les dates D/K pertinentes. Ne pas joindre un coffre ou des documents personnels à un signalement public. Les traces et fixtures de validation du dépôt sont synthétiques. L’application n’envoie pas de télémétrie et ne requiert ni service distant ni compte externe.
