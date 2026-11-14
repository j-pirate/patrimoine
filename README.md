# Patrimoine familial international

Projet d’application Linux locale de gestion patrimoniale familiale, en français.

**État : phase 10 — qualification et livraison Linux locale.** Le socle Tauri/React/Rust gère les métiers patrimoniaux, leurs sources, leurs historiques et leurs documents. Le tableau de bord présente les bilans familiaux ou individuels, les répartitions, l’historique et les alertes ; les sociétés disposent d’une vue de pilotage de leurs données enregistrées. Les graphiques utilisent les montants exacts des moteurs Rust. Un net complet exige toujours une couverture explicitement déclarée et des sources cohérentes.

## Installer et récupérer ses données

**[Télécharger l’application, le kit et les guides](distribution/README.md)** — version 0.1.1 pour Debian 13 x86_64, avec instructions de reconstitution et de lancement.

**Pop!_OS 22.04 / Ubuntu 22.04 : [paquets compatibles glibc 2.35](distribution/0.1.1-popos22.04/README.md)**, avec un `.deb` et une AppImage entière. Cette reconstruction corrige l’incompatibilité de bibliothèques du paquet Debian 13 sur ces systèmes, sans modification des calculs ni du schéma 6.

La livraison cible **Debian 13 x86_64**, avec les bibliothèques d’exécution GTK 3/WebKitGTK 4.1. La construction locale produit l’AppImage et son fichier `SHA256SUMS` ; le rapport de qualification identifie les artefacts effectivement testés. Le script de construction ne téléverse aucun paquet et l’application ne se met pas à jour automatiquement. Node et Rust ne sont pas requis pour utiliser le paquet.

Le [guide d’installation, mise à jour et récupération](docs/23-installation-et-recuperation-linux.md) décrit le lancement, les données XDG, la sauvegarde préalable, le retour à une version compatible et les deux voies de récupération. Le [rapport de qualification](docs/22-validation-phase10.md) donne les versions et systèmes effectivement testés, les mesures de performance et leurs limites. L’inventaire des composants accompagne la livraison dans ses notices.

Les sources de la version **0.1.1**, les tests et la documentation figurent dans ce dépôt. Les six ZIP de téléchargement sont versionnés dans `distribution/0.1.1/`. Les coffres synthétiques et les sorties locales de construction ne sont pas suivis par Git. Pour construire l’AppImage, utiliser `bash scripts/build-linux.sh` avec les prérequis du [guide de développement](docs/07-developpement.md) ; le [générateur du kit de récupération](scripts/build-recovery-kit.py) produit le secours autonome. Les chemins `.local/` et `/workspace/.tooling/` présents dans les rapports désignent des preuves conservées dans l’environnement de qualification, absentes du dépôt GitHub.

Sur le grand profil synthétique, la cible de deux secondes n’est pas atteinte et le tableau de bord dépasse le budget de 120 secondes. Deux avertissements de dépendances amont restent documentés dans la [revue de sécurité](docs/24-revue-securite.md).

## Développement

Voir le [guide Linux et cloud](docs/07-developpement.md) pour les prérequis. Sur Linux avec GTK/WebKit installés :

```bash
npm ci
npm run tauri dev
```

Vérifications : `bash scripts/check.sh`. Dans le cloud non privilégié, `bash scripts/setup-cloud.sh`, puis `source scripts/cloud-env.sh` préparent la compilation. Le parcours graphique réel s’exécute avec `bash scripts/test-desktop-cloud.sh`, dans un conteneur de test sans réseau.

Les données sont conservées dans `${XDG_DATA_HOME:-$HOME/.local/share}/patrimoine`, hors répertoire d’installation. Les sauvegardes ZIP incluent un snapshot SQLite autonome, les originaux et leurs empreintes. Les [preuves et limites de validation actuelles](docs/22-validation-phase10.md) précisent le périmètre réellement testé ; les rapports des phases précédentes conservent leurs preuves historiques. Aucun jeu de données fictif n’est injecté dans le coffre utilisateur.

## Utiliser le registre

Créer les personnes et leurs périodes familiales, puis les sociétés et participations éventuelles. Dans « Patrimoine », enregistrer les actifs, leurs propriétaires et leurs estimations. « Immobilier et loyers » complète ces mêmes actifs avec leurs fiches, unités et contrats, sans double valorisation. Rattacher les comptes et caisses aux actifs existants, puis confirmer leur ouverture explicitement. « Comptes bancaires » et « Espèces et devises → Caisses physiques » suivent le journal ; rapprocher une ligne déjà présente évite de la doubler lors d’une saisie de loyer, de dépôt ou de financement.

« Dettes et financements » distingue les parties et droits propres du prêt, son principal, ses intérêts, ses frais et les garanties conditionnelles. Dans « Patrimoine → Bilan du périmètre », renseigner les couvertures et créanciers locatifs nécessaires avant de calculer un net complet. « Espèces et devises → Taux et conversions » conserve les cours EUR/TND/CHF/USD/AED ; « Historique » donne accès aux anciennes connaissances et au pont de variation historique du registre. Les corrections conservent les anciennes révisions. Les coffres et sauvegardes des schémas 1 à 5 sont migrés dans une copie séparée vers le schéma courant 6, sans inventer de trésorerie pour les anciennes sources.

« Or et argent » complète les actifs avec des lots, stocks confirmés, mouvements, cours et primes explicites. « Sociétés → Indicateurs et dividendes » conserve les observations périodiques séparément des valorisations et comptes. Un bénéfice retenu ou un dividende déclaré n’est pas un encaissement personnel ; un paiement réel suit son journal et ses contreparties.

« Documents » conserve descriptions, versions originales, classement et liens aux personnes, actifs, baux, financements ou lots. « Sauvegarde et export » produit des archives SQLite ou CSV avec leurs originaux et un XLSX de consultation. Le réimport CSV reconstruit une base vierge contrôlée avant confirmation du remplacement ; il ne fusionne pas deux coffres. Tous les exports intégraux portent sur le coffre actuel et son histoire, indépendamment des filtres de consultation.

« Tableau de bord » reprend le contexte famille/membre, la date économique et les connaissances K. Ses filtres sélectionnent les positions après éliminations internes ; le rapport JSON exporté conserve ce contexte et ses sources. L’évolution d’un sous-total connu n’est pas présentée comme une performance d’investissement. Cette consultation ne modifie pas les données du coffre et son rapport filtré ne remplace pas une sauvegarde complète.

La recherche globale de l’entête retrouve les objets métier et documents connus à K dans tout le coffre de l’administrateur local, avec renvoi vers leur module et leurs identifiants. Son périmètre est indépendant des filtres du tableau de bord.

[Tableaux de bord et rapports](docs/21-tableaux-de-bord.md) · [Famille et propriété](docs/09-famille-et-propriete.md) · [Devises et historique](docs/11-devises-et-historique.md) · [Immobilier et loyers](docs/13-immobilier-et-loyers.md) · [Comptes et financements](docs/15-comptes-et-financements.md) · [Métaux et sociétés](docs/17-metaux-et-societes.md) · [Documents et portabilité](docs/19-documents-et-portabilite.md) · [Validation actuelle](docs/22-validation-phase10.md)

## Dossier de conception

1. [Besoin, architecture et organisation du projet](docs/01-architecture.md)
2. [Modèle relationnel SQLite détaillé](docs/02-modele-relationnel.md)
3. [Propriété, consolidation, calculs et historique](docs/03-regles-metier.md)
4. [Documents, export, sauvegarde et restauration](docs/04-documents-et-portabilite.md)
5. [Tests prioritaires, risques et tâches d’implémentation](docs/05-validation-et-plan.md)
6. [Registre des décisions d’architecture](docs/decisions.md)
7. [Avancement et limites](docs/avancement.md)
8. [Cahier des charges fourni — version 3.0](docs/cahier-des-charges.md)

Les fonctions livrées et leurs limites sont consignées dans les guides et rapports des phases 2 à 10. La suite concerne la maintenance, les retours d’usage et la qualification d’autres postes ou volumes selon les besoins. L’application n’envoie aucune donnée à un service externe et ne requiert ni compte ni serveur distant.
