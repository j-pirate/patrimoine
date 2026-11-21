# Tableaux de bord et rapports — utilisation de la phase 9

Guide du **12 novembre 2026, Europe/Paris**. Les [preuves de validation](../historique/validation-phase-9.md) distinguent les calculs exacts, les lectures historiques, l’interface et le parcours Linux. Les graphiques présentent les données enregistrées dans le coffre, sans jeu de démonstration ajouté à l’espace utilisateur.

## Choisir le contexte et calculer

Ouvrez **Tableau de bord** et choisissez la famille ou un membre dans **Périmètre de consultation**. Pour remonter dans le temps, ouvrez **Date et historique**. La **Date de référence** détermine le jour de consultation. Les **Connaissances disponibles** déterminent les modifications que le rapport peut utiliser. Vous pouvez ainsi revoir une situation telle qu’elle était connue avant une correction.

Choisissez **Actualiser le tableau de bord** pour voir le brut, les dettes, le net et les catégories. Les options du tableau de bord permettent de préciser le début et le pas de l’historique. Le dernier point correspond à la date de référence. Tous les chiffres du rapport sont calculés à partir du même état du coffre.

Le changement de contexte ou de filtre rend le résultat précédent indisponible jusqu’à un nouveau calcul. Si une requête plus ancienne se termine après la nouvelle, elle ne remplace pas le résultat du nouveau contexte. En cas d’erreur, vérifier le message puis relancer ; un échec ne certifie aucun montant.

Pour consulter une société, sélectionner **Vue de société**. Cette vue de pilotage utilise les comptes, financements, participations et observations de la société choisie. Les filtres des positions familiales ou individuelles sont alors désactivés. Revenir au contexte familial ou individuel pour consulter le bilan économique consolidé de ce périmètre.

## Lire les indicateurs

Les montants consolidés du tableau de bord sont exprimés en **EUR**. Les valeurs d’origine, devises, droits et références restent accessibles dans les détails. La préférence de devise de l’espace ne transforme pas la devise de consolidation.

Un **patrimoine net complet** exige les couvertures d’inventaire et toutes les sources nécessaires. Une absence d’ouverture de compte, de change, de droits économiques, de créancier ou de valorisation ne vaut pas zéro. Quand le moteur ne peut pas établir le total complet, consulter les **sous-totaux connus** et les alertes : leur différence ne certifie pas un patrimoine net exhaustif.

Les positions recouvrent les actifs estimés, banques, espèces, métaux, participations, créances et passifs réellement pris en charge par les moteurs. Les garanties conditionnelles restent séparées des dettes certaines. Une observation indicative de trésorerie ou de dette dans une société ne remplace pas un compte ou un financement enregistré.

Une estimation effectivement retenue dans les sources du calcul déclenche l’alerte **estimation ancienne** au-delà de **365 jours** à D, également pour les dates de l’historique. Cette alerte d’information ne remplace pas la valeur, ne change pas son montant et ne rend pas à elle seule un bilan incomplet. Le seuil de cette version est fixe ; une estimation de 365 jours ne dépasse pas encore le seuil.

Les indicateurs locatifs gardent le périmètre et le pays de leur rapport. Leur période est **[début, D)** : la date D est exclue des flux de période, tandis que les positions de stock du tableau de bord sont observées à D. Si début et D sont identiques, aucun rapport de période locative n’est produit ; cette absence ne certifie pas un revenu nul. Les filtres de devise, type, propriétaire associé et recherche des positions ne recalculent pas ces indicateurs. Le revenu couru, le loyer encaissé, les impayés et le cash-flow après financement gardent leurs significations distinctes ; un dépôt de garantie n’est pas un loyer.

## Filtrer et retrouver les positions

Les champs **Pays**, **Devise d’origine**, **Type de position**, **Propriétaire associé (positions entières)** et **Rechercher une position** sélectionnent les positions déjà attribuées au périmètre familial ou individuel. Le pays suit les métadonnées datées disponibles ; la devise est celle de la source, même lorsque son montant est affiché en EUR.

Le filtre de propriétaire associé retient une position entière concernant ce propriétaire. Il ne transforme pas la sélection en calcul de sa quote-part juridique ou en nouveau bilan individuel. Pour consulter la part économique d’un membre, utiliser le sélecteur global de membre. Les propriétaires associés et les références de chaque position permettent d’interpréter les biens communs.

Les éliminations internes sont appliquées avant les filtres. Une créance et une dette réciproques ne réapparaissent donc pas simplement parce qu’un filtre masque une de leurs faces. Les détails montrent les montants avant élimination, la réduction attribuée et le montant résiduel ; les regroupements portent sur ces résidus exacts.

Les alertes de couverture du périmètre restent visibles après filtrage. Masquer une position inconnue ne confirme ni sa valeur ni l’exhaustivité du coffre. Le total filtré décrit les positions sélectionnées, avec la portée indiquée par le rapport.

**Rechercher dans le coffre**, dans l’entête, permet aussi de retrouver les objets métier et documents connus à K. Saisir au moins deux caractères dans **Nom, description documentaire ou identifiant** ; chaque résultat indique son identifiant et permet d’ouvrir son module. **Afficher les résultats suivants** donne accès aux résultats au-delà des premiers affichés. Cette recherche porte sur l’ensemble du coffre accessible à l’administrateur local, indépendamment du membre et des filtres du tableau de bord. Les libellés des personnes, sociétés et actifs suivent les descriptions disponibles à D ; les éléments retirés sont signalés. Elle ne recalcule aucun montant.

## Répartitions et comparaison des membres

Les répartitions par **type de position**, **pays** et **devise** reprennent les positions résiduelles du même rapport. Les montants exacts figurent dans les tables associées aux graphiques ; la taille d’une barre est une représentation de ces montants. Les positions non calculables et leur nombre restent visibles, au lieu d’être remplacées par une barre nulle.

La vue **Différences connues par membre** compare les bilans individuels calculés avec les mêmes D et K. Un prêt entre deux membres peut figurer dans chacun de leurs bilans, puis être neutralisé dans le bilan familial. Additionner les actifs bruts et les dettes brutes des membres ne remplace donc pas le calcul familial. Une différence individuelle connue garde les limites de son propre bilan.

## Historique et variation de période

Le **Pas de l’historique** permet une lecture mensuelle ou annuelle entre le début choisi et D. Il conserve le début exact, les fins civiles de mois ou les 31 décembre strictement intermédiaires, puis D exactement. Le rapport contient au maximum **61 points** ; une période dépassant cette capacité doit être raccourcie ou utiliser le pas annuel. Aucune partie de la période n’est tronquée silencieusement.

Tous les points utilisent les connaissances K sélectionnées. Une correction apprise après K ne modifie pas la courbe de cet ancien état. Modifier D ou le début de période ne change pas la borne K du rapport.

La série de patrimoine net complet garde une lacune lorsqu’un point n’est pas calculable. La vue **Sous-totaux connus** permet de consulter séparément les informations disponibles ; sa courbe ne certifie pas que le périmètre est exhaustif. Les tables de l’historique exposent dates, valeurs et états, y compris sans lecture visuelle du graphique.

La variation entre les deux bornes est une différence de stocks. Elle peut inclure des changements de droits, de périmètre, de valeurs, de change et des mouvements réels. Elle n’est pas un rendement d’investissement ; un apport n’est pas présenté comme une plus-value par cette seule différence.

Le **Pont des estimations du registre** conserve le périmètre historique de ce moteur : méthodes et périmètre, droits, estimations et change. Il ne constitue pas une explication exhaustive des mouvements du bilan intermodules. Les stocks dérivés de métaux et les autres exclusions indiquées dans ce pont ne sont pas réintroduits par le graphique. Vérifier ses références et ses avertissements avant de rapprocher son sous-total de celui du bilan.

## Vue de société

La **Vue de société** présente ses comptes et leurs soldes issus du journal, ses positions de financement et leurs parts contractuelles, ses observations de gestion et ses distributions. Elle dénombre aussi les actifs associés et les participations suivies ; leurs fiches restent consultables dans les modules de saisie. Les sources et avertissements sont ceux du rapport métier de société à D/K.

Le début de période sélectionne les observations terminées dont la fin est **strictement postérieure au début choisi et au plus tard à D**. Ces observations restent des périodes entières : une période ayant commencé avant le début choisi peut être affichée si sa fin se situe dans cette fenêtre, sans prorata. Les distributions sont sélectionnées du début choisi à D, **deux bornes incluses**. Les comptes, financements, actifs, propriétaires et méthodes restent observés à D ; modifier le début de période ne les transforme pas en flux.

Une période mensuelle, trimestrielle ou annuelle est une observation de pilotage. Les fréquences et périodes incompatibles ne s’additionnent pas pour fabriquer un résultat annuel. Un résultat, une trésorerie indicative ou un dividende déclaré ne crée pas un mouvement bancaire ni une valeur de parts. La vue ne produit pas de nouveau patrimoine net consolidé de société à partir de ces indicateurs.

Dans **Évolution des observations de société**, choisir **Fréquence des observations** et **Devise des observations**. Le graphique représente une seule fréquence et une seule devise à la fois, avec le chiffre d’affaires et le résultat net déclaré lorsqu’ils existent. La date affichée est la fin de la période, exclue ; une valeur manquante reste une lacune. Le tableau associé conserve les montants et identifiants des observations sélectionnées.

La déclaration d’un dividende reste distincte de son paiement. Les rapprochements des faces société et bénéficiaire permettent de voir quelles sommes ont été réellement enregistrées dans la trésorerie. Les corrections et les anciens K restent disponibles dans leurs modules de saisie.

## Exporter le rapport filtré

Après le calcul, choisir **Exporter le rapport filtré**, puis son emplacement dans le dialogue Linux. Le fichier **JSON** conserve la requête, le contexte D/K, la période, les filtres, la version du format et du moteur, les valeurs exactes, les séries et les références de source. Avec des filtres de positions actifs, il conserve les positions, totaux, répartitions, membres et historiques filtrés et omet les bilans sources, indicateurs locatifs et pont non filtrés ; les sections exclues sont indiquées dans le fichier. Sans ces filtres, les rapports sources du périmètre restent inclus, avec leurs portées propres. Annuler le dialogue ne crée pas de fichier et ne modifie pas le coffre. Une destination déjà présente est refusée sans écrasement.

Ce fichier sert à consulter ou traiter le rapport sélectionné. Il ne contient pas une base restaurable ni tous les originaux documentaires, et l’application ne l’utilise pas pour activer un coffre. Pour récupérer les données, utiliser les sauvegardes SQLite ou les archives CSV intégrales avec leurs pièces dans **Sauvegarde et export**. Le XLSX de cette section reste lui aussi intégral, indépendamment des filtres de consultation.

Les lectures du tableau de bord et l’export du rapport ne créent aucune révision financière ni nouvel événement de réimportation. Le schéma reste en version 6. La qualification des volumes, performances et paquets de livraison Linux relève de la phase 10 ; les [résultats de cette version](../historique/validation-phase-9.md) précisent les contrôles effectivement réalisés.
