# Famille, sociétés et propriété — utilisation de la phase 3

Ce guide décrit le registre initial. Les taux, estimations multidevises, profils datés et connaissances historiques sont décrits dans [le complément de phase 4](11-devises-et-historique.md).

L’application travaille sur une base commune. Une vue individuelle est une sélection patrimoniale ; elle ne limite pas les accès de l’administrateur local. Les nouveaux coffres sont vides : les exemples des tests ne sont jamais chargés dans vos données.

## Premier enregistrement

1. Dans **Famille**, créer une personne. Le nom affiché identifie sa fiche ; prénom, nom, naissance, résidence, devise préférée et notes peuvent être renseignés séparément. Une personne créée n’est pas automatiquement membre du périmètre familial : elle peut représenter un copropriétaire extérieur.
2. Ajouter une **période familiale** pour chaque membre inclus, avec début, fin éventuelle et lien familial. La date de fin est exclue : une fin au 1er juillet signifie que la personne est encore incluse le 30 juin, puis exclue le 1er juillet. Plusieurs périodes non chevauchantes sont possibles.
3. Dans **Sociétés**, créer les sociétés concernées, puis leurs participations datées. Le propriétaire d’une société peut être une personne ou une autre société. Préciser séparément droits juridiques, économiques et droits de vote.
4. Définir pour chaque société sa **méthode datée**. Le mode détaillé retient ses sous-jacents ; le mode global retient une estimation nette des capitaux propres et s’arrête sur ce chemin. La couverture et la responsabilité des associés sont des déclarations explicites, jamais déduites d’une table vide.
5. Dans **Patrimoine**, créer l’actif générique, ses relations de propriété et une estimation datée en EUR. Indiquer sa source ou sa méthode, par exemple « estimation personnelle ». Une fiche générique ne remplace pas encore les modules immobilier/loyers, banque ou métaux.
6. Choisir le périmètre familial ou individuel et la date en haut de l’écran, puis **Contrôler les attributions**. Les sources des droits et estimations de chaque contribution sont consultables.

Les filtres de gestion permettent d’afficher l’ensemble du registre, toutes les périodes ou les fiches archivées. Ils facilitent la saisie d’un futur droit ou d’un actif pas encore attribué. Le calcul reste effectué côté Rust avec le contexte sélectionné, indépendamment du filtre visuel de gestion.

## Droits exacts et données inconnues

Les droits se saisissent sous forme de fraction : 50 % = `1/2`, 30 % = `3/10`, un tiers = `1/3`. Une fraction non réduite, telle que `50/100`, est réduite exactement par Rust avant stockage. Numérateur et dénominateur doivent être des entiers ; un dénominateur nul ou une fraction supérieure à un sont refusés.

« Inconnu », `0/1` et `1/1` sont trois valeurs distinctes. Le droit économique n’est pas copié automatiquement depuis le titre juridique ou les votes. Si les détenteurs connus ne couvrent pas 100 %, le reliquat demeure inconnu ; pour un tiers identifié, créer réellement sa fiche et son droit. Les cycles et les droits incohérents sont refusés sur toutes les périodes affectées, y compris dans le passé ou le futur.

Exemple synthétique : un actif de 400 000 EUR détenu par A à `1/2`, B à `3/10` et un tiers à `1/5` attribue 200 000 EUR à A, 120 000 EUR à B et 320 000 EUR à la famille. Si A possède 60 % de X et X 50 % de Y, le chemin A→X→Y transmet 30 % ; les chemins indépendants s’additionnent.

## Estimations et contrôle

Une nouvelle estimation datée ajoute une observation ; « Modifier » corrige celle sélectionnée en conservant l’ancienne révision. La dernière estimation applicable avant ou à la date de référence est utilisée. Une observation future ne sert jamais à compléter une date passée.

Le formulaire accepte le point ou la virgule et conserve une valeur décimale canonique côté Rust. Aucun calcul financier n’utilise les nombres binaires JavaScript. Les multiplications et additions restent rationnelles exactes ; l’affichage EUR est arrondi au centime, avec égalité vers le pair.

Le contrôle présente un **sous-total des contributions valorisables** et ses limites. S’il n’existe aucune contribution calculable faute de données, il affiche « Non calculable ». Les valeurs globales de parts ne sont jamais ajoutées une seconde fois à leurs sous-jacents. Un chemin direct indépendant vers une filiale reste cependant conservé.

Le contrôle n’affirme pas un patrimoine net exhaustif :

- Les passifs et créances détaillés ne sont pas encore saisis dans l’application.
- Le change n’est pas encore implémenté. Une valeur étrangère reçue par l’API reste dans sa devise et ne reçoit pas de parité supposée ; les formulaires de cette phase sont en EUR.
- La date et les révisions des droits, appartenances, méthodes et estimations sont prises en compte. Les profils et libellés sont les versions connues à la borne de connaissance, sans intervalle civil distinct à ce stade.
- La responsabilité des associés et les couvertures ambiguës doivent être qualifiées ; une exposition négative ne devient pas automatiquement une dette personnelle.

Le moteur contient aussi des tests de positions de prêts et d’éliminations proportionnelles. Il s’agit d’un contrat du domaine pur, sans gestionnaire de dettes persistant ni formulaire ; la saisie des financements appartient à la phase 6.

## Corrections et archivage

Chaque création/correction porte un motif. Une correction exige la révision que vous avez ouverte ; si une version plus récente existe, le coffre refuse l’écriture. Recharger puis rouvrir la fiche permet de comparer et reprendre la modification. L’échec ne laisse ni demi-enregistrement ni audit de mutation validée.

Archiver une personne, une société ou un actif ne clôture ni ses droits ni sa période familiale. Utiliser la fin d’un intervalle pour représenter un changement économique. Une annulation de relation ou d’estimation est une correction de la connaissance ; elle conserve les versions précédentes. Une identité encore référencée ne peut pas être annulée en laissant des relations actives orphelines.

La borne de connaissance est une séquence de modification monotone. Le backend sait recalculer une ancienne séquence ; le sélecteur d’interface utilise pour l’instant les connaissances actuelles. La consultation avancée des anciennes révisions et l’historique multidevise sont prévus en phase 4.

## Sauvegardes et anciennes données

Le ZIP complet contient également tous les enregistrements du registre, ses révisions, l’audit et les pièces originales. Le filtre de contexte ne limite pas cette sauvegarde. Les identifiants et les résultats du contrôle sont vérifiés après restauration dans les tests.

Un coffre du schéma 1 est validé, copié avec l’API SQLite puis migré en schéma 2 dans une nouvelle génération. L’ancienne reste conservée. Une archive du schéma 1 est d’abord vérifiée dans son format original, puis sa copie préparée est mise à niveau ; le résumé annonce cette migration avant confirmation. Une ancienne version de l’application ne doit pas ouvrir en écriture une base plus récente.
