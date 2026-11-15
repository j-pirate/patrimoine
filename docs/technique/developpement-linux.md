# Développement et qualification Linux

L’application desktop locale inclut les métiers des phases 2 à 7, le gestionnaire documentaire avec portabilité de phase 8 et les tableaux de bord de phase 9. La phase 10 ajoute la construction et la qualification de la livraison Linux, les mesures et les procédures d’installation/récupération. Les exports SQLite et CSV incluent les originaux et leurs historiques ; le XLSX sert à la consultation. Le bilan financier exige une couverture explicite et conserve les montants inconnus. Voir [l’avancement](../projet/avancement.md) et [la validation actuelle](../historique/validation-phase-10.md) pour les résultats réellement vérifiés ; les rapports des phases 2 à 9 conservent leurs preuves historiques.

## Outils et dépendances

- Node **24.22.1**, indiqué dans `.node-version` ; dépendances résolues par `package-lock.json`.
- Rust **1.99.1**, épinglé dans `rust-toolchain.toml` ; dépendances résolues par `Cargo.lock`.
- Recharts **3.10.1**, compatible avec React 19, épinglé dans les dépendances npm et embarqué avec le module de tableau de bord ; aucun chargement distant à l’exécution.
- Python **3.10 ou plus récent**, avec le module standard `sqlite3` lié à SQLite **3.37 ou plus récent**, pour les tests et l’outil indépendant de récupération CSV. Aucune dépendance Python externe n’est nécessaire à cet outil.
- Linux avec GTK 3 et WebKitGTK 4.1, outils C/C++, `pkg-config` et ressources graphiques.
- Plateforme de compilation et cible de qualification locale : Debian 13 x86_64. Le rapport actuel précise les conditions de lancement du paquet ; aucune compatibilité universelle avec les autres distributions ou anciennes glibc n’est présumée.

Sur une machine Debian/Ubuntu administrée, installer avec le gestionnaire système : `build-essential`, `pkg-config`, `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `librsvg2-dev` et `patchelf`. Les noms exacts dépendent de la distribution. Installer Rust avec son gestionnaire officiel puis les dépendances npm avec `npm ci`.

Dans le cloud Debian 13 sans accès administrateur, depuis le checkout existant :

```bash
cd /workspace/patrimoine
bash scripts/setup-cloud.sh
source scripts/cloud-env.sh
```

Le script place Rust et un sysroot GTK/WebKit dans `/workspace/.tooling`, hors checkout. Les index APT restent signés et les paquets sont vérifiés par APT ; l’installeur Rust est comparé à son checksum officiel, puis Rust vérifie ses composants. Aucune désactivation de TLS ou de signature. Les sources, manifests et lockfiles du projet ne sont pas modifiés. Les répertoires `/workspace/.tooling` et `target` sont des outils/caches, pas des données patrimoniales. Le sysroot permet la compilation ; il ne remplace pas l’installation système nécessaire à WebKit pour lancer ses sous-processus. Dans ce cloud non privilégié, le contrôle graphique utilise le conteneur de test décrit plus bas.

## Lancer l’application

```bash
npm run tauri dev
```

Cette commande de lancement suppose les bibliothèques installées aux chemins système sur une machine Linux avec session graphique. Dans le cloud actuel, utiliser le test desktop conteneurisé pour exécuter le binaire avec un environnement graphique complet.

Vite sert uniquement au développement. `npm run dev` seul lance l’interface dans un navigateur sans backend Tauri ; il ne fournit pas de coffre ni de persistance de remplacement. Utiliser le lancement desktop pour importer, sauvegarder ou restaurer.

Pour construire un exécutable local embarquant l’interface :

```bash
npm run build
cargo build -p patrimoine-desktop --features custom-protocol --locked
./target/debug/patrimoine
```

Pour construire le paquet AppImage en mode release, après les vérifications et avec les outils GTK/WebKit et `patchelf` disponibles :

```bash
bash scripts/build-linux.sh
```

Le script ne publie aucun fichier. Les paquets et leurs métadonnées sont produits dans `target/release/bundle/appimage/`, avec `SHA256SUMS` et `build-manifest.json`. Les notices tierces sont intégrées aux ressources du paquet. Un exécutable de développement reste distinct de cette livraison. Le [guide Linux](../utilisateur/installation-linux.md) décrit installation, mise à jour et récupération ; le [rapport](../historique/validation-phase-10.md) conserve les essais du paquet et leurs limites.

## Vérifier

```bash
bash scripts/check.sh
```

Cette commande vérifie TypeScript, exécute les tests UI, construit l’interface, teste l’outil Python de récupération CSV, contrôle le format Rust, exécute les tests du workspace, lance Clippy et compile le desktop. Les tests utilisent des coffres temporaires ; aucune donnée personnelle n’est nécessaire. La CI Linux reprend ces vérifications.

Une session graphique Linux est requise pour lancer la fenêtre desktop. Un écran virtuel sert aux contrôles automatisés headless ; il ne remplace pas une qualification complète du paquet sur le poste de l’utilisateur.

## Contrôle desktop réel dans le cloud

```bash
bash scripts/test-desktop-cloud.sh
```

Ce contrôle nécessite le démon Docker géré localement. Le script construit l’image Debian de test avec TLS vérifié et certificat monté temporairement, puis lance le binaire réel sous WebKitGTK. La phase d’exécution utilise `--network none`, un utilisateur non privilégié et un coffre temporaire sans données réelles. Le binaire, le pilote et le script sont montés en lecture seule. Les seuls fichiers conservés sont les artefacts synthétiques dans un nouveau dossier `.local/desktop-smoke.*` ignoré par Git.

Le scénario exerce l’IPC, les paramètres via l’interface, les dialogues natifs de documents et de sauvegarde, puis l’analyse/confirmation de restauration. Le rapport JSON distingue les étapes réussies, échouées et non exécutées. Une capture d’écran est enregistrée. Les tests Rust restent la preuve des contrôles d’intégrité détaillés ; ce parcours vérifie leur intégration desktop. Xvfb est démarré directement car le script `xvfb-run` attendait un signal de disponibilité non transmis dans le conteneur imbriqué de cette session.

## Outils de qualification complémentaires

Le driver de performance utilise les services réels en mode release et un coffre synthétique isolé :

```bash
cargo build --release -p patrimoine-infrastructure --example qualify_performance --locked
python3 scripts/qualify-performance.py --output .local/performance-pilot --profile pilot
```

Le profil `full` prépare 100 personnes, 100 sociétés en DAG, 10 000 actifs, 100 000 opérations de journal et 10 000 originaux distincts. Le dossier de sortie doit être neuf ; `--reuse` reprend uniquement une fixture du même profil. Les insertions de préparation suivent le schéma livré et sont ensuite soumises à une véritable ouverture validante du coffre. Deux changements portent la fixture : elle mesure un volume, pas 100 000 commits successifs. Le rapport conserve séparément validation d’ouverture, temps des opérations, mémoire échantillonnée, matériel et interruptions. Il ne constitue pas une mesure de latence du DOM ni une garantie de réponse inférieure à deux secondes.

L’[audit d’accessibilité natif](../../scripts/audit-desktop-accessibility.py) utilise axe-core épinglé, le pilote WebKit et `tauri-driver` dans une session graphique de test. Il reçoit un binaire, une sauvegarde synthétique et le chemin du moteur axe local :

```bash
python3 scripts/audit-desktop-accessibility.py /chemin/absolu/patrimoine \
  --backup /chemin/absolu/sauvegarde-synthetique.zip \
  --axe node_modules/axe-core/axe.min.js \
  --artifacts .local/accessibilite-1280 --width 1280 --height 860
```

Le test vérifie la sauvegarde puis ouvre sa copie privée, parcourt les écrans et formulaires prévus et contrôle des interactions clavier. Les règles automatiques incomplètes demandent une lecture explicite ; leur absence de violation ne suffit pas à une certification générale d’accessibilité. Le rapport actuel précise aussi la campagne à largeur 900 px.

Le [comparateur d’avis JavaScript](../../scripts/audit-js-offline.mjs) lit le lockfile et un clone Git local de la base publique GitHub Advisory Database, avec la version SemVer épinglée dans les dépendances de développement :

```bash
mkdir -p .local
node scripts/audit-js-offline.mjs \
  --database /chemin/absolu/advisory-database \
  --output .local/avis-javascript.json
```

Le comparateur ne contient pas de client réseau. Le commit de la base, l’empreinte du lockfile, les correspondances et les plages non résolues sont enregistrés. Les avis Rust et bibliothèques natives ont leur propre revue ; la méthode et les limites sont exposées dans la [revue de sécurité](revue-securite.md).

Le [collecteur de notices](../../scripts/collect-third-party-licences.py) lit les dépendances résolues et, avec `--appdir`, les composants du paquet préparé. Son inventaire et ses limites figurent dans les [licences tierces](licences-tierces.md). Régénérer les notices après une modification des dépendances, puis reconstruire le paquet qui les embarque.

## Données et récupération

Par défaut : `${XDG_DATA_HOME:-$HOME/.local/share}/patrimoine`. Les valeurs XDG relatives sont ignorées. Le dossier contient `CURRENT`, les générations SQLite/documents et les zones de préparation. Une deuxième instance sur le même coffre est refusée. Les fichiers originaux sont importés dans le coffre et ne dépendent plus de leur chemin source.

Pour un essai isolé, passer un chemin absolu propre à cette commande, sans changer le répertoire personnel :

```bash
PATRIMOINE_DATA_DIR=/tmp/patrimoine-essai ./target/debug/patrimoine
```

Cette variable est une option de développement/portabilité ; ne pas utiliser `/tmp` pour des données réelles. Ne pas partager une base SQLite active sur un système de synchronisation de fichiers.

La section « Sauvegarde et export » produit un ZIP contenant un snapshot SQLite autonome, les originaux et un manifeste SHA-256. L’analyse de restauration ne remplace rien. L’activation exige une confirmation et conserve la génération précédente ; une sauvegarde exportée de l’état actuel est proposée par défaut. Une archive de version future, incohérente ou altérée doit être rejetée.

Pour une récupération indépendante, décompresser une archive de confiance dans un dossier neuf, ouvrir `data/patrimoine.sqlite3` avec un outil SQLite standard et lire les originaux de `documents/`. Les noms d’origine sont conservés dans les métadonnées. Ne pas copier seulement le fichier principal d’une base ouverte en mode WAL. Les détails de format et de sûreté figurent dans [la conception de portabilité](conception-documents-portabilite.md).

Le socle n’introduit aucun chiffrement applicatif ni télémétrie. Les archives contiennent les données en clair : utiliser un support protégé et les protections du système Linux. Le gestionnaire documentaire conserve les versions et associations ; les exports CSV/XLSX et la reconstruction CSV sont disponibles. Une suppression documentaire est un retrait logique, sans purge implicite des originaux historiques.

## Registre, finances et schéma 6

Les guides [tableaux de bord](../utilisateur/tableaux-de-bord.md), [famille et propriété](../utilisateur/famille-et-propriete.md), [devises/historique](../utilisateur/devises-et-historique.md), [immobilier/loyers](../utilisateur/immobilier-et-loyers.md), [comptes/financements](../utilisateur/comptes-et-financements.md), [métaux/sociétés](../utilisateur/metaux-et-societes.md) et [documents/portabilité](../utilisateur/documents-et-portabilite.md) décrivent les écrans utilisables. La migration `0006_metals_companies.sql` complète les cinq scripts précédents, conservés à l’identique. L’ouverture d’un ancien coffre et la préparation d’une ancienne sauvegarde des schémas 1 à 5 migrent une copie séparée vers le schéma 6 et gardent la génération source. Les phases 8 à 10 conservent ce schéma 6 et les six scripts inchangés. Le premier format CSV prend en charge ce schéma 6. Les campagnes détaillées dans les rapports de validation exercent également les anciennes bases et les données WAL.

Le contrôle d’attribution reprend les valeurs dérivées des métaux en remplacement de leur estimation générique. Le pont de variation garde son périmètre historique du registre et signale cette exclusion ; il ne prétend pas décomposer la performance des nouveaux stocks. Le panneau distinct « Bilan du périmètre » utilise `get_financial_report` pour intégrer métaux, trésorerie, créances, passifs et éliminations. Les valeurs d’origine, taux et fractions exactes transitent sous forme de chaînes. `convert_currency` et `explain_variation` utilisent un snapshot unique borné par K ; les profils datés ne modifient pas les droits. Les anciens points d’entrée `consolidate` et `consolidate_with_loans` restent distincts de ce bilan complet intermodules. Une valeur absente et un sous-total connu ne doivent pas être présentés comme un net complet ; une table vide n’affirme aucune couverture.

Les traits `PropertyService` et `RentalService` ajoutent des snapshots séparés, avec une lecture SQLite commune au registre pour les rapports à K. `dispose_property`, les corrections de cession et `refund_rent_payment` regroupent les révisions liées dans un même `change_set`. La validation croisée des mutations du registre empêche de rompre une dépendance immobilière/locative. Les nouveaux montants restent des textes exacts ; les unités techniques n’ont aucune valorisation propre. Les rapports exposent dates, IDs des sources et cotations de conversion ; leur résultat n’est pas un rapport immuable enregistré.

Les traits `TreasuryService`, `DebtService` et `FinancialService` ajoutent les snapshots financiers bornés par K et des rapports à D/K. Les commandes `post_rent_payment`, `post_rental_expense`, `post_deposit` et `post_debt_movement` regroupent la source, ses affectations éventuelles et les liens vers les révisions et lignes exactes du journal. Elles vérifient une séquence globale attendue. Le choix d’une ligne existante ne crée pas une seconde écriture bancaire ; une ouverture, compensation de dépôt ou dépréciation ne représente pas un flux. Les déclarations d’inventaire, créanciers d’échéance et couvertures des valeurs globales sont explicites et historisées. La migration des anciennes sources ne crée pas de mouvements de trésorerie par hypothèse.

Les traits `MetalService` et `CompanyService` complètent ces lectures à K. Les commandes `post_metal_movement`, `transfer_metal_ownership` et `post_company_distribution` regroupent les effets corrélés sous la séquence attendue. Les comptes et actifs du registre sont réutilisés ; aucun indicateur périodique n’est injecté comme valorisation, dette ou banque. Les modules React de métaux et observations de sociétés sont chargés à la navigation depuis les ressources embarquées locales, sans appel réseau.

Les traits `DocumentService` et `PortabilityService` ajoutent les versions documentaires, associations et exports intégraux. Les CSV reconstruisent une candidate avec les seules migrations livrées ; les identités et anciens K sont conservés, puis un événement technique de provenance est ajouté dans la candidate. La confirmation réutilise `activate_restore` et son contrôle de génération/séquence. Les exports utilisent un snapshot SQLite cohérent ; ni les CSV ni le XLSX ne dépendent des filtres de consultation.

Les budgets de portabilité de cette version sont de 11 000 entrées ZIP, 4 Mio pour le manifeste, 256 Mio pour l’entrée SQLite canonique seule, 128 Mio pour les autres fichiers et 512 Mio décompressés au total. Les CSV gardent deux millions de cellules par table et 1 Mio par cellule. Ces bornes sont contrôlées avant activation ; les anciens lecteurs peuvent appliquer des budgets plus faibles. Le schéma et les six migrations restent inchangés.

Le trait `DashboardService` ajoute une lecture cohérente du tableau de bord et l’export JSON d’un rapport filtré. Une transaction de lecture résout K une seule fois et charge les snapshots utilisés par tous les moteurs, les membres et les points historiques. Le domaine Rust répartit exactement les éliminations sur les faces concernées avant les filtres et regroupements ; les garanties conditionnelles restent hors des totaux. Le frontend ne convertit les montants finaux en nombres que pour la géométrie Recharts ; les libellés et tables conservent les chaînes exactes du backend. L’export du rapport est une opération de consultation sans nouvelle révision ni activation de coffre. Les sauvegardes et exports de portabilité restent intégraux.

Pour **Ouvrir le document**, une session Linux avec `xdg-open` et un lecteur associé au format choisi est nécessaire. L’export d’une copie reste disponible indépendamment de ce lecteur. La validation bureautique utilise openpyxl et LibreOffice comme outils de contrôle indépendants ; ils ne sont requis ni par l’application ni par `scripts/check.sh`. Aucun tableur n’est requis pour conserver ou réimporter une archive CSV.
