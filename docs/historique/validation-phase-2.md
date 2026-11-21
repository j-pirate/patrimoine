# Validation du socle — phase 2

Périmètre : socle local Linux, paramètres, base SQLite versionnée, documents minimaux et premier cycle de sauvegarde/restauration. Les règles financières, le graphe familial et les exports CSV/XLSX complets relèvent des phases ultérieures. Les critères ci-dessous traduisent [la stratégie de récupération](../technique/conception-documents-portabilite.md) pour ce premier schéma.

## Critères d'acceptation

| ID | Scénario | Résultat requis |
|---|---|---|
| P2-D01 | Première ouverture puis réouverture d'un coffre | Génération stable, schéma migré une seule fois, paramètres persistants ; pas de réinitialisation d'un coffre existant endommagé. |
| P2-D02 | Deux instances sur le même coffre | Verrou exclusif ; la seconde échoue explicitement sans modifier la base. |
| P2-D03 | `CURRENT` invalide, chemin externe ou génération manquante | Refus fermé, génération antérieure conservée ; aucune base vide créée pour masquer l'erreur. |
| P2-D04 | Paramètres invalides | Écriture atomique refusée avec code d'erreur exploitable ; paramètres antérieurs conservés. |
| P2-B01 | Sauvegarder une base ouverte avec données récentes et document original | SQLite autonome sauvegardé par l'API SQLite, avec toutes les lignes et pièces référencées ; aucun WAL nécessaire à la lecture indépendante. |
| P2-B02 | Document source identique ou nom dangereux | Contenu stocké par hash ; nom d'origine descriptif sans maîtrise du chemin interne. |
| P2-B03 | Restaurer dans une génération neuve | Paramètres, métadonnées, octets et empreintes documentaires identiques ; ancienne génération de récupération retenue. |
| P2-R01 | Analyser puis annuler la restauration | Aucun remplacement ; la confirmation explicite est nécessaire pour activer. |
| P2-R02 | Écriture postérieure à l'analyse | Jeton périmé refusé avant activation ; aucun écrasement d'une mutation non présentée. |
| P2-R03 | Candidate modifiée après analyse | Revalidation/hash détecte la mutation avant activation. La source ZIP originale peut être déplacée après analyse si la candidate validée est autonome. |
| P2-R04 | Panne autour du déplacement de génération et du renommage de `CURRENT` | Au redémarrage : ancien ou nouvel état complet ; jamais mélange ni création silencieuse d'une base vide. |
| P2-A01 | Manifestes ou archives malformés, version future | Refus avant activation ; coffre courant inchangé. |
| P2-A02 | Hash, taille, liste de fichiers ou nombre de lignes incohérents | Refus explicite ; un manifeste modifié avec des empreintes recalculées ne remplace pas la validation de structure SQLite. |
| P2-A03 | Chemin `..`, absolu, doublon, lien symbolique ou type de fichier spécial | Refus sans extraction en dehors de la préparation ; aucune exécution. |
| P2-A04 | Fichier supplémentaire, absent, bombe de décompression | Liste fermée et limites avant/pendant lecture ; échec préservant l'état actif. |
| P2-A05 | SQLite schéma inattendu, vue/trigger ajouté ou base corrompue | Schéma attendu, `integrity_check`, clés étrangères et validateurs du schéma courant ; aucune extension/migration de l'archive exécutée. |
| P2-L01 | Ressources Linux | Compilation Rust/TypeScript et construction Tauri ; lancement et interaction réelle sous Linux à distinguer d'une simple compilation. |
| P2-L02 | Usage local | Aucune API distante nécessaire au fonctionnement, ressources d'interface embarquées et données hors dépôt/AppImage. |

Les tests de panne simulant un retour d'erreur ne prouvent pas à eux seuls la résistance à une coupure électrique. Les points `fsync`, renommages et reprise doivent être recensés ; les limites de la campagne exécutée restent explicites.

## Tests et preuves à recueillir

- Suite Rust : tests unitaires et intégration du domaine, de l'application et de l'infrastructure ; tests adversariaux dans `crates/infrastructure/tests/recovery_adversarial.rs`.
- Suite interface : navigation, paramètres, documents et restauration en deux étapes ; erreurs et annulations visibles.
- Compilation : vérification TypeScript, construction Vite et compilation Tauri Linux.
- Vérification locale : ouvrir SQLite de sauvegarde avec un lecteur indépendant, contrôler les pièces originales, relancer le coffre restauré ; dialogue de choix et confirmations réels si une session graphique est disponible.
- Conservation : contrôle du diff pour séparer le socle livré des modules financiers encore absents ; aucune donnée de démonstration injectée dans le coffre utilisateur.

## Résultats observés

Contrôles exécutés le 4 novembre 2026, dans le dépôt, avec Rust 1.99.1 installé sous `/workspace/.tooling` :

```sh
CARGO_HOME=/workspace/.tooling/cargo RUSTUP_HOME=/workspace/.tooling/rustup \
PATH=/workspace/.tooling/cargo/bin:$PATH \
cargo test -p patrimoine-infrastructure --test recovery_adversarial --offline
```

Résultat final de cette commande : **29 tests exécutés, 29 réussis, aucun échec, aucun ignoré**, code de sortie 0. Les contrôles couvrent :

- Schémas/formats futurs, `user_version` futur, ajout de vue/trigger SQLite même après recalcul des hashes, référence orpheline, paramètres invalides et devise de référence manquante.
- Empreintes modifiées, fichier absent ou supplémentaire, noms ZIP dangereux, doublons réels des en-têtes ZIP et liens symboliques.
- Verrou exclusif, `CURRENT` interdit comme chemin, pointeur trop long, génération absente sans création de base vide.
- Confirmation obligatoire, péremption du jeton après paramètres ou document, révocation lors d'une nouvelle analyse, candidate modifiée ou fichier WAL ajouté après analyse.
- Candidate autonome après suppression du chemin ZIP d'origine ; sauvegarde restaurable d'un original fortement compressible.
- Reprise d'états d'activation interrompue construits sur disque : avant/après bascule du pointeur, nouvelle base corrompue et original manquant dans la nouvelle génération. Les deux derniers cas réactivent l'ancienne génération conservée ; l'original y reste exportable.

Commande complémentaire, avec les mêmes variables d'outillage :

```sh
cargo clippy -p patrimoine-infrastructure --test recovery_adversarial --offline -- -D warnings
```

Résultat : réussite, code 0, sans avertissement. Le formatage Rust ciblé du fichier de tests a également réussi.

La revue a conduit à corriger la lecture bornée de `CURRENT`, la compatibilité des sauvegardes fortement compressibles avec les limites du lecteur, la validation des référentiels, les fichiers supplémentaires de candidate et la détection de doublons masqués par l'index de la bibliothèque ZIP. Leurs tests de régression font partie des 29 résultats ci-dessus.

## Vérification intégrée finale

Le 4 novembre 2026, le socle utilise les versions verrouillées suivantes : Rust 1.99.1, Node 24.22.1, Tauri natif 2.12.5, React 19.3.1, `rusqlite` 0.41.1 avec SQLite embarqué 3.54.1, `zip` 9.3.0. Les deux dernières bibliothèques ont été actualisées avant la campagne finale pour ouvrir les entrées non fiables avec leurs versions actuelles.

| Contrôle | Résultat observé |
|---|---|
| TypeScript strict et Vite production | Réussite, ressources embarquables produites |
| Vitest / Testing Library | **10 tests exécutés, 10 réussis**, aucun ignoré |
| Intégration Rust | **33 tests exécutés, 33 réussis** : 4 scénarios standards et 29 adversariaux, aucun ignoré |
| Formatage Rust et Clippy workspace, toutes cibles et fonctionnalités | Réussite, aucun avertissement accepté |
| Binaire Linux avec `custom-protocol` | Compilation réussie, interface embarquée sans serveur Vite |
| Préparation cloud répétée | `scripts/setup-cloud.sh` réexécuté avec les lockfiles, réussite |
| Lecture indépendante de la sauvegarde | SQLite Python standard : `integrity_check=ok`, aucune violation FK ; paramètres lisibles et hash de l’original vérifié |

Les crates de contrats et les doc-tests ne contiennent pas encore de tests unitaires : leur résultat « zéro test » ne constitue pas une preuve métier. Les 33 tests ci-dessus ont bien exécuté leurs assertions d’intégration sur fichiers réels.

### Parcours desktop réel sans réseau

Commande reproductible : `bash scripts/test-desktop-cloud.sh`. Le conteneur Debian 13 de test est exécuté sous utilisateur 1000 avec `--network none`, écran Xvfb et coffre neuf temporaire ; il reçoit uniquement le binaire, le pilote et le script en lecture seule, plus le dossier d’artefacts synthétiques. Aucun compte ni donnée utilisateur n’est utilisé.

La campagne finale a terminé avec le statut `passed` et neuf étapes réussies :

1. Rendu réel WebKitGTK et appel IPC du statut : schéma 1, quatre pays et cinq devises.
2. Saisie et sauvegarde des paramètres dans l’interface, relus via l’IPC Rust.
3. Vue documentaire et liste IPC initialement vides.
4. Import d’un original par dialogue GTK, nom et SHA-256 contrôlés.
5. Export par dialogue GTK : copie identique octet pour octet.
6. Sauvegarde ZIP complète par dialogue GTK.
7. Analyse, affichage du résumé puis annulation : génération et compteur de mutation inchangés.
8. Nouvelle analyse et restauration confirmée, avec sauvegarde préalable sélectionnée dans le dialogue : génération remplacée, paramètres et document restaurés.
9. Capture de l’écran final.

Rapport de cette exécution : `.local/desktop-smoke.WToKYmZZ/desktop-result.json`, capture `desktop.png` et archives synthétiques dans le même dossier, ignorés par Git. Chaque nouvelle exécution crée son propre dossier et son propre rapport.

Disponibilité actualisée : ces originaux sont [conservés seulement dans l’environnement](preuves.md#disponibilité-par-campagne), hors ZIP distribué ; le présent compte rendu est la pièce versionnée de cette campagne.

Deux problèmes ont été distingués et corrigés pendant cette intégration : l’automatisation WebKit envoyait une rafale de touches trop rapidement (attente de chaque valeur DOM ajoutée ; mises à jour React fonctionnelles et test de saisie concomitante), et GTK ne sélectionnait pas l’extension du nom dans le dialogue de sauvegarde (sélection complète du champ ajoutée au pilote). Le parcours final ne contourne ni les dialogues ni les portées de fichiers du backend.

Le sysroot non privilégié permet la compilation dans le cloud, mais WebKit attend ses sous-processus aux chemins système pour lancer une fenêtre. La vérification graphique utilise donc l’image Debian complète ; elle n’est pas revendiquée comme lancement direct sur l’hôte cloud. Sur un poste Linux, le guide demande une installation normale des bibliothèques système.

## Limites de cette campagne

- Les états de reprise sont simulés par des fichiers réellement présents avant réouverture. Il ne s'agit pas d'une injection de panne à chaque appel système, ni d'un test de coupure électrique du système de fichiers. La résistance à tous les points de panne physique n'est donc pas déclarée prouvée.
- Les limites de taille, de nombre et de compression sont relues dans le code ; cette suite ne remplit pas un disque et ne provoque pas de panne matérielle. Le test de forte compressibilité vérifie qu'une pièce admise reste restaurable.
- La revue des chemins relève un contrôle préalable de tous les composants et `O_NOFOLLOW` sur le fichier terminal ; elle ne constitue pas une preuve contre un processus du même utilisateur remplaçant concurremment des répertoires parents. Le coffre est privé ; la V1 ne protège pas d'un administrateur local malveillant.
- Les 29 tests portent uniquement sur le schéma du socle. Ils ne prouvent aucun calcul patrimonial, cycle familial, conversion financière ou export CSV/XLSX, absents de cette phase.
- La commande Rust n'exécute ni WebView Tauri, ni dialogue natif. La compilation, les tests d'interface et le lancement Linux doivent être consignés séparément ; aucune validation graphique ne découle de ce résultat.
