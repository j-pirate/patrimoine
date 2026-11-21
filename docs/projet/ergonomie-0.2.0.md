# Refonte de l’ergonomie de Patrimoine 0.2.0

## Phase A — Audit de la version 0.1.2

Le dépôt de référence était `main` au commit `05f6d80`. Les vérifications ont porté sur le code des écrans et les parcours dans l’AppImage distribuée. La fenêtre Tauri a été ouverte hors ligne dans Ubuntu 22.04, avec un coffre neuf réservé à l’audit.

| Rubrique | Difficulté constatée | Refonte retenue |
| --- | --- | --- |
| Navigation | Treize rubriques de même niveau. Chaque page expose simultanément le périmètre familial, la date économique et la séquence de connaissance. | Prioriser les parcours courants. Regrouper le reste et laisser le périmètre visible en repliant les options de consultation historique. |
| Tableau de bord | Les filtres et réglages de calcul prennent la place des premiers repères. Un coffre vide donne peu d’indications sur la prochaine action. | Mettre les totaux, catégories et prochaines actions en évidence. Conserver le détail des avertissements et les réglages à la demande. |
| Famille et sociétés | Les premières actions utilisent le registre générique. Personnes, qualité de membre, sociétés et participations demandent de comprendre plusieurs formulaires. | Ajouter des fiches guidées et renvoyer les structures plus complexes aux écrans de détail. |
| Immobilier | Un bien demande d’abord la création d’un actif dans le registre, puis la création d’une fiche dans un autre écran. Les actions de bail et de valorisation sont denses au premier regard. | Enregistrer le bien depuis Immobilier, puis proposer la consultation de la fiche, l’estimation, les propriétaires et les loyers. |
| Banque, espèces, métaux | Un actif et un titre de propriété sont requis avant le compte ou le lot. Les termes du registre masquent les informations dont le logiciel a besoin. | Enregistrer en une opération les données saisies et leurs liens. Expliquer le titulaire, le solde de départ, le poids et la pureté au moment de la saisie. |
| Sauvegarde, documents, historique | L’écran de sauvegarde et d’export utilise des termes proches pour des opérations aux effets différents. Les écrans d’historique demandent de connaître le fonctionnement des séquences. | Décrire séparément la copie du coffre, l’export pour consultation et la restauration. Expliquer comment retrouver l’état connu avant une modification. |

La base SQLite, les commandes d’export et de restauration et les calculs Rust servent de référence aux régressions. Une saisie simplifiée doit enregistrer les mêmes objets avec les mêmes contrôles et laisser une donnée inconnue explicitement manquante.

## Phase B — Navigation et saisies guidées

Les rubriques les plus fréquentes restent visibles dans le menu. Le périmètre familial ou individuel se choisit sur chaque page. Les dates et états historiques disposent de leurs explications et d’une commande de retour à l’état actuel. Chaque rubrique principale indique son usage et propose un accès aux fiches ou à la première saisie.

Les ajouts guidés couvrent la famille, l’immobilier, les comptes, les espèces, les métaux et les sociétés. Dans Immobilier, une seule saisie enregistre la fiche et le bien. L’estimation et le propriétaire peuvent être renseignés ultérieurement. Les valeurs, propriétaires et dettes sont identifiés séparément. Le parcours des loyers distingue le bail, les échéances et les paiements reçus.

Les commandes guidées utilisent les validations et les écritures courantes, dans une transaction SQLite par saisie. Chaque demande porte un identifiant permettant de confirmer le résultat si la fenêtre se ferme au moment de l’enregistrement. Le schéma reste à la version 6.

**Vérifications de la phase.** Dans la fenêtre Linux Tauri, un membre puis un bien immobilier ont été enregistrés depuis leurs rubriques et relus par l’application. Le bien était visible sans propriétaire ni estimation. Les tests des commandes vérifient aussi la confirmation après réouverture, l’annulation complète d’une saisie refusée et la conservation des données inconnues.

## Phase C — Assistant de premier lancement

L’assistant s’ouvre sur un coffre neuf et propose sept étapes facultatives. Il est possible de ne rien saisir, de revenir sur une étape, de quitter et de reprendre depuis les paramètres. Le récapitulatif reprend les fiches effectivement enregistrées. Les coffres déjà utilisés accèdent directement à l’interface.

La progression et les brouillons sont conservés dans un fichier local propre à la génération active du coffre, en dehors des sauvegardes et exports patrimoniaux. La tentative est écrite avant l’opération métier. Un redémarrage au moment de l’enregistrement peut retrouver sa confirmation grâce au même identifiant. Une fiche déjà validée reste dans SQLite et dans les sauvegardes.

**Vérifications de la phase.** Les cinq scénarios graphiques natifs hors ligne passent, dont la fermeture suivie d’une reprise de la saisie exacte, les sept étapes renseignées et le rejet d’une estimation incohérente avec conservation des membres déjà enregistrés. Le contrôle Rust couvre aussi la lecture après fermeture, la sauvegarde et la restauration d’un coffre ayant un brouillon.

## Phase D — Tour d’horizon illustré

**Découvrir Patrimoine** est accessible depuis Aide, la fin de l’assistant et les boutons d’aide des rubriques. Les huit étapes couvrent le tableau de bord, la famille, les biens immobiliers, les loyers, les comptes, les métaux, les sociétés et les sauvegardes. Les images sont des recadrages de la fenêtre de Patrimoine, réalisées sur un coffre fictif. Elles sont intégrées au paquet pour un usage hors ligne.

**Vérifications de la phase.** Le test graphique a vérifié le décodage des huit illustrations locales, le clavier et le retour du focus, l’ouverture du bon onglet des baux et l’accès depuis l’assistant. Le coffre, les écritures du registre et le compteur des sauvegardes sont restés identiques après la démonstration.
