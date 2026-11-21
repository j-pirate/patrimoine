# Qualification Linux — phase 10

Campagne des **12 et 13 novembre 2026, Europe/Paris**. Cette qualification porte sur un **candidat local 0.1.1**, Debian 13 x86_64, et des données exclusivement synthétiques. Elle ne vaut pas publication ni certification générale. Les limites de performances et les avertissements de dépendances sont conservés ci-dessous.

Les liens de preuves passent par l’[index des pièces publiées](preuves.md). La [matrice des campagnes](campagnes-validation.md) distingue l’AppImage, les ELF de développement et les binaires de service, avec leurs empreintes et les métadonnées source effectivement enregistrées.

## Paquet et installation

L’[AppImage 0.1.1](../../distribution/0.1.1/README.md) est construite avec `source scripts/cloud-env.sh` puis `bash scripts/build-linux.sh`, sortie 0. Le [journal](preuves.md#preuve-066) et le [manifeste de construction](../../distribution/0.1.1/build-manifest.json) identifient système, noyau, glibc, outils, dépendances verrouillées et composants WebKit copiés.

| Artefact | Taille | SHA-256 |
|---|---:|---|
| `Patrimoine_0.1.1_amd64.AppImage` | 112 196 088 octets | `6abd1f90a70baa001560787f46e5b10ced7e3fd364b428c99541c454b5d9c240` |
| [Kit de récupération](../../distribution/0.1.1/Patrimoine_0.1.1_recuperation.zip) | 17 640 octets | `bd659307716e374e42b69e7ad6100523c807e4323c7f4ac61a09853de5221044` |

Les [empreintes du paquet](../../distribution/0.1.1/SHA256SUMS) et du [kit](../../distribution/0.1.1/SHA256SUMS) détectent une différence avec ces valeurs ; elles ne constituent pas une signature de l’auteur. Les versions npm, workspace, quatre crates locales et configuration Tauri sont cohérentes à `0.1.1`. Le schéma SQLite reste **6**, sans modification des migrations 1–6.

La construction utilise un cache utilisateur pour les outils de packaging, un runtime AppImage officiel épinglé par empreinte et les processus WebKit correspondant au sysroot. Le sysroot sert à compiler ; les plugins GTK de packaging utilisent les chemins d’exécution de `/usr`. Les dépendances sont verrouillées et les composants copiés empreintés ; l’identité binaire entre deux constructions n’est pas revendiquée.

L’installation, le démarrage, les dialogues et l’IPC réels sont exercés dans Debian 13 sous utilisateur non privilégié, réseau désactivé, sans Rust, Cargo, Node, npm, GCC ni G++. Le mode effectivement vérifié est **`APPIMAGE_EXTRACT_AND_RUN=1`**, sans montage FUSE. Cela ne démontre pas le fonctionnement d’un montage FUSE sur tous les postes, ni une compatibilité Ubuntu, ARM ou avec une glibc plus ancienne. Le [guide Linux](../utilisateur/installation-linux.md) fournit les commandes et les prérequis.

## Vérifications automatisées

La [campagne complète finale](preuves.md#preuve-061), puis les tests du lecteur Python après son optimisation équivalente, réussissent :

| Campagne | Résultat |
|---|---|
| TypeScript / Vite | Types et compilation réussis |
| Interface Vitest | **165 tests**, 19 fichiers, aucun échec ni ignoré |
| Domaine Rust | **219 tests** |
| Infrastructure Rust | **197 tests** |
| Workspace Rust | **416 tests réussis**, aucun échec ni ignoré |
| Secours CSV Python | **16 tests réussis**, [journal final](preuves.md#preuve-069) |
| Lecteur d’avis JavaScript | **7 contrôles locaux** de frontières de plages OSV |
| Formatage / Clippy | `rustfmt` conforme ; toutes cibles et fonctionnalités avec `-D warnings` |
| Binaire de production | Construction optimisée avec interface embarquée, puis paquet AppImage |

Les ajouts Rust couvrent les capacités d’archives, les racines de chaînes de révisions et l’ordre d’import des 62 tables. Les deux nouveaux tests Python vérifient indépendamment l’ordre parent/enfant du schéma local, l’auto-référence admise et le refus d’un cycle entre tables. Les deux tests UI supplémentaires contrôlent le focus des formulaires.

## Parcours du paquet et mise à jour

`bash scripts/test-desktop-cloud.sh .local/releases/0.1.1/Patrimoine_0.1.1_amd64.AppImage` réussit les **50 étapes natives** des phases précédentes sur le paquet lui-même : [résultats finaux](preuves.md#preuve-031). Le [fichier du candidat testé](preuves.md#preuve-030) enregistre l’empreinte de l’AppImage montée en lecture seule, pour distinguer ce test de celui d’un ELF de développement. Les montants exacts, anciens K, originaux, deux récupérations et export du rapport filtré restent contrôlés.

Les inspections indépendantes du même parcours réussissent pour les [archives SQLite et CSV](preuves.md#preuve-033), le [XLSX avant et après réenregistrement réel dans LibreOffice](preuves.md#preuve-034), et les [rapports du tableau de bord](preuves.md#preuve-032). Elles comparent les 62 tables et 758 lignes sources, les originaux et les références. Le contrôle du dashboard recalcule les répartitions et neutralisations en fractions exactes ; il contrôle les dates, sources et rapprochements historiques sans réimplémenter tous les moteurs financiers historiques.

Le scénario séparé `scripts/test-appimage-cloud.sh` installe **deux paquets réellement différents**, `0.1.0` puis `0.1.1`, au même chemin d’installation. Ce sont des candidats locaux ; `0.1.0` n’est pas présentée comme une ancienne diffusion publique. Les six étapes contrôlent :

1. Ouverture du candidat précédent sur une copie d’un coffre riche placé sous `XDG_DATA_HOME/patrimoine`.
2. Remplacement du programme par `0.1.1` : les **62 tables, 758 lignes sources, deux originaux**, génération et neuf snapshots métier restent identiques, ainsi que le bilan au K source 120.
3. Écriture réelle de réglages par la nouvelle version : un changement et un audit supplémentaires, sans modification des autres tables ni du passé à K=120.
4. Redémarrage hors ligne de `0.1.1`, conservant ces réglages et K=121.
5. Retour au paquet `0.1.0`, qui lit le même coffre de schéma 6, avec les nouveaux réglages et les anciens résultats intacts.
6. Refus réel par l’ancien exécutable d’une copie passée artificiellement au schéma 7, sortie non nulle, sans modification des tables ni de la génération de cette copie.

Les [six résultats finaux de mise à jour et retour arrière](preuves.md#preuve-004) sont réussis et enregistrent les deux empreintes. Les essais n’utilisent pas `PATRIMOINE_DATA_DIR` pour ce scénario : ils contrôlent le vrai mécanisme XDG. Aucun changement de paquet n’efface les données dans ces essais. Un retour entre deux versions partageant le schéma 6 ne prouve pas qu’un ancien programme pourrait lire un futur schéma ; la récupération par sauvegarde compatible reste nécessaire dans ce cas.

Un premier candidat privé `0.1.1` a été [conservé avec son empreinte](preuves.md#preuve-059) avant la correction d’ordre d’import découverte par le gros essai CSV. Le candidat désigné en tête de ce rapport est celui reconstruit après ce correctif ; les essais finaux portent sur ses octets.

## Accessibilité et fenêtre minimale

Les deux audits WebKit réels réussissent : [1280 × 860](preuves.md#preuve-001) et [900 × 650](preuves.md#preuve-002). Ils couvrent **17 états par taille**, soit **34 états**, avec axe-core 4.14.0 : les treize sections, deux formulaires, la recherche globale et la vue minimale complémentaire. Aucun résultat financier ou état React simulé n’est injecté ; la bibliothèque axe est locale, hors réseau.

Les mesures automatiques donnent **zéro violation détectée** et aucun débordement horizontal global. Six contrôles clavier par taille sont réussis : navigation, ouverture et fermeture de formulaire, focus du titre puis retour au déclencheur, saisie de recherche et Échap. La séquence métier reste inchangée. La [capture à la largeur minimale](preuves.md#preuve-003) conserve la mise en page vérifiée.

Les corrections portent sur contrastes, rôles ARIA de groupes/régions, focus des formulaires, accès clavier aux tableaux défilants et passage des filtres du dashboard à deux colonnes dès 1 100 px. Le débordement de 16 px observé sur la sélection de société a été identifié puis corrigé, sans masquer le débordement global ni retirer l’assertion.

Les résultats axe **`incomplete` restent présents**, notamment sur les graphiques et la barre latérale défilée. Seuls les symboles décoratifs de navigation ont une revue explicite complémentaire dans ces rapports. Zéro violation automatique ne signifie donc pas une certification WCAG complète, ni une validation exhaustive par lecteur d’écran.

## Volume, performances et corrections nécessaires

Le [pilote](preuves.md#preuve-055) et le [profil complet](preuves.md#preuve-052) utilisent le vrai moteur en **release**. Les oracles monétaires sont calculés séparément en `Decimal` Python ; les vérifications de reprise comparent aussi les types, lignes, identités et originaux. Les comparaisons exactes du [pilote](preuves.md#preuve-056) et du [profil complet](preuves.md#preuve-053) portent sur les 62 tables et les deux voies de récupération. La [synthèse des mesures](preuves.md#preuve-051) conserve les dépassements et leur portée.

| Profil | Personnes / sociétés | Actifs | Opérations de journal | Originaux |
|---|---:|---:|---:|---:|
| Pilote | 10 / 10 | 100 | 1 000 | 100 |
| Complet | 100 / 100 | 10 000 | 100 000 | 10 000 |

Le profil complet contient **773 124 lignes**, 160 726 identités et révisions, un DAG de sociétés et **deux changements** : lot initial et correction d’une estimation. Il ne représente pas 100 000 commits séquentiels. Chaque original fait **128 octets** ; le nombre de fichiers est éprouvé, pas un volume de 10 000 PDF ou scans usuels. La limite totale d’archive reste applicable à de vraies pièces plus volumineuses.

Machine déclarée : Linux x86_64, AMD EPYC 9V74, cinq processeurs logiques visibles, quota cgroup de **quatre CPU** et **32 Gio** de mémoire. Ces ressources ne sont pas une machine de test réservée ; les charges et versions sont enregistrées. L’ouverture réelle et ses validations sont mesurées séparément des services. Un mode persistant évite de réouvrir artificiellement le coffre avant chaque opération ; les caches et la rétention mémoire sont donc explicitement conservés. Les anciens essais par processus séparés et les interruptions sont aussi conservés.

La cible provisoire de **deux secondes n’est pas atteinte sur le profil complet**. L’ouverture optimisée a été observée autour de 62–82 secondes selon l’essai et la charge. Les premières consultations mesurées sont d’environ 3,48 s pour le registre, 2,03 s pour la trésorerie, 6,88 s pour la bibliothèque, 7,53 s pour le compte, 18,27 s pour le bilan, 23,19 s pour l’ancien K, 21,21 s pour la vue individuelle et 5,15 s pour la conversion. Ces mesures de service ne certifient pas la latence de l’interface sur le même volume.

Deux goulets réels ont été corrigés :

- Les validations de racines de révisions utilisaient un prédicat `IS NULL` qui conduisait SQLite à choisir un index inadapté sur les gros lots. Le comptage conditionnel par objet conserve exactement les règles, sans nouveau schéma ni index ; les cas zéro, une et plusieurs racines restent testés.
- L’import CSV alphabétique chargeait notamment l’audit avant les révisions. La résolution différée des références provoquait alors des scans répétés des enfants. Le nouvel ordre provient des seules clés étrangères du **schéma local approuvé**, avec parents avant enfants ; les auto-références restent différées. Les clés étrangères restent actives et toutes les validations sont conservées. L’export et le dictionnaire restent alphabétiques. L’ancien essai a dépassé 600 secondes dans l’insertion ; le nouvel essai d’analyse complète aboutit autour de **192,77 secondes**, validations comprises.

La sauvegarde complète a abouti en **165,39 secondes**, l’export CSV en **87,83 secondes**, la restauration SQLite dans une cible vierge en **362,12 secondes** et le réimport CSV complet en **446,95 secondes**. Le tableau de bord du grand profil a dépassé **120,04 secondes** et a été interrompu par le budget prévu. Le bilan final est donc de **14 opérations abouties sur 15**, avec ce délai dépassé ; il ne constitue pas une réussite complète du profil. Le premier budget de 120 secondes était également insuffisant pour les archives et son interruption est conservée. Les reprises utilisent un budget d’archive explicite de 600 secondes, sans transformer les dépassements de la cible de consultation en succès de performance.

La comparaison exacte finale vérifie directement la base de la sauvegarde immuable contre la source : **62 schémas, 773 124 lignes typées, identités déterministes UUID5 et 10 000 originaux**. Les deux cibles conservent les données et les montants attendus : banque 100 001,00 EUR, actifs et net 799 931,00 EUR à K=1 puis 799 932,00 EUR à K=2, passifs nuls explicitement couverts. La reprise SQLite reste à K=2 ; la reprise CSV ajoute seulement la provenance technique à K=3, sans nouvelle révision métier.

L’empreinte physique de la source a changé pendant la campagne : le premier snapshot l’ouvre en WAL, ce qui modifie **six octets de l’entête SQLite** (journalisation, compteur et version du moteur). L’assertion d’identité binaire reste donc **fausse** dans les preuves. Le [diagnostic](preuves.md#preuve-054) reconstruit uniquement ces six octets en mémoire et retrouve l’empreinte préalable du fichier entier ; tous les octets après l’entête, les tables, l’histoire et les pièces sont préservés. Le vérificateur n’a pas réécrit la source pour obtenir ce résultat.

Les mesures de RSS portent sur le processus pendant l’étape, avec allocations éventuellement conservées par le processus persistant. Le pic historique `VmHWM` est signalé comme cumulatif lorsque sa remise à zéro est refusée ; il n’est pas présenté comme une nouvelle allocation propre à chaque étape. Aucun engagement universel de débit, mémoire ou capacité d’interface n’est fixé à partir de ces seules mesures.

## Capacités d’archives et secours autonome

Les plafonds nécessaires au jeu préparé sont bornés et testés : **11 000 entrées**, manifeste **4 Mio**, SQLite canonique **256 Mio**, chaque autre fichier ou table **128 Mio**, total décompressé **512 Mio**. Les limites de chemins, doublons, ratios de compression, taille de cellule et invariants métier restent inchangées. Le [contrôle de capacité réel](preuves.md#preuve-050) inclut une base SQLite de plus de 128 Mio et des manifestes dépassant l’ancien plafond de 2 Mio ; les frontières 11 000 / 11 001 sont exercées séparément.

Le kit contient le lecteur Python standard, les six migrations vérifiées par empreintes et une procédure française. Son [essai hors réseau](preuves.md#preuve-058), sans exécutable Patrimoine, reconstruit le gros CSV puis compare **les 62 tables, les 773 124 lignes typées et les 10 000 originaux** avec la sauvegarde SQLite indépendante : identité exacte, K source 2 conservé. Ce secours n’exécute pas les invariants financiers Rust ; sa copie est destinée à la consultation indépendante et au secours, pas à l’activation automatique d’un coffre.

## Sécurité et composants

La [revue de sécurité](../technique/revue-securite.md) conserve les méthodes et limites des audits locaux. Les avis publics JavaScript sont rapprochés localement sans envoyer l’inventaire à npm. RustSec conserve **deux avertissements amont**, dont l’API GLib affectée non identifiée dans les appels examinés ; aucune absence universelle de risque ni correction de cette dépendance n’est revendiquée.

Les [notices tierces](../technique/licences-tierces.md) recensent **482 composants** et les textes/provenances conservés. Le [contrôle d’extraction du candidat final](preuves.md#preuve-060) vérifie les 747 fichiers du dossier embarqué, y compris le runtime et AppRun. Les versions binaires non retrouvées sont signalées ; la présence des notices et d’URLs ne constitue pas une offre complète de sources ni une autorisation automatique de redistribution publique.

Les données restent hors du paquet, en clair sous les protections du compte et du support. Aucun changement de configuration cloud, envoi d’inventaire de dépendances ou publication distante n’est effectué par cette qualification. Les performances des très grands coffres, les deux avertissements de dépendances et les postes non testés demeurent des limites explicites du candidat.
