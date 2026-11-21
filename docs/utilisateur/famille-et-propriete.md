# Famille, sociétés et propriété

Ce guide explique la saisie des fiches et des droits. Les taux, estimations multidevises et profils datés ont leur propre guide. [Devises, dates et historique](devises-et-historique.md)

L’application travaille sur une base commune. Une vue individuelle est une sélection patrimoniale ; elle ne limite pas les accès de l’administrateur local. Les nouveaux coffres sont vides : les exemples des tests ne sont jamais chargés dans vos données.

## Membres, contacts et droits

1. Dans **Famille**, choisissez **Ajouter un membre de la famille** pour créer sa fiche et commencer à suivre ses biens. Le formulaire **Créer une personne** sert aussi aux contacts extérieurs à la famille, par exemple un copropriétaire ou un locataire.
2. Dans **Périodes d’appartenance à la famille**, modifiez les dates d’inclusion lorsque nécessaire. La date de fin est exclue. Une fin au 1er juillet signifie que la personne est encore incluse le 30 juin, puis exclue le 1er juillet. Plusieurs périodes non chevauchantes sont possibles.
3. Dans **Autres biens et dettes → Sociétés**, choisissez **Ajouter une société**, puis renseignez ses associés ou ajoutez des participations datées. Le propriétaire d’une société peut être une personne ou une autre société. Précisez séparément les droits juridiques, économiques et de vote.
4. Définir pour chaque société sa **méthode datée**. Le mode détaillé retient ses sous-jacents ; le mode global retient une estimation nette des capitaux propres et s’arrête sur ce chemin. La couverture et la responsabilité des associés sont des déclarations explicites, jamais déduites d’une table vide.
5. Dans **Autres biens et dettes → Patrimoine**, ajouter les droits des copropriétaires ou créer un bien sans rubrique dédiée. Pour un bien immobilier, un compte ou un lot de métal, commencez dans la rubrique correspondante.
6. Choisir le périmètre familial ou individuel, puis **Contrôler les attributions**. **Date et historique** permet de changer la date de référence. Les sources des droits et estimations sont consultables pour chaque contribution.

Les filtres de gestion permettent d’afficher l’ensemble du registre, toutes les périodes ou les fiches archivées. Ils facilitent la saisie d’un futur droit ou d’un actif pas encore attribué. Le calcul reste effectué côté Rust avec le contexte sélectionné, indépendamment du filtre visuel de gestion.

## Droits exacts et données inconnues

Les droits se saisissent sous forme de fraction : 50 % = `1/2`, 30 % = `3/10`, un tiers = `1/3`. Une fraction non réduite, telle que `50/100`, est réduite exactement par Rust avant stockage. Numérateur et dénominateur doivent être des entiers ; un dénominateur nul ou une fraction supérieure à un sont refusés.

« Inconnu », `0/1` et `1/1` sont trois valeurs distinctes. Le droit économique n’est pas copié automatiquement depuis le titre juridique ou les votes. Si les détenteurs connus ne couvrent pas 100 %, le reliquat demeure inconnu ; pour un tiers identifié, créer réellement sa fiche et son droit. Les cycles et les droits incohérents sont refusés sur toutes les périodes affectées, y compris dans le passé ou le futur.

Exemple synthétique : un actif de 400 000 EUR détenu par A à `1/2`, B à `3/10` et un tiers à `1/5` attribue 200 000 EUR à A, 120 000 EUR à B et 320 000 EUR à la famille. Si A possède 60 % de X et X 50 % de Y, le chemin A→X→Y transmet 30 % ; les chemins indépendants s’additionnent.

## Estimations et contrôle

Une nouvelle estimation datée ajoute une observation ; « Modifier » corrige celle sélectionnée en conservant l’ancienne révision. La dernière estimation applicable avant ou à la date de référence est utilisée. Une observation future ne sert jamais à compléter une date passée.

Le formulaire accepte le point ou la virgule et conserve une valeur décimale canonique côté Rust. Aucun calcul financier n’utilise les nombres binaires JavaScript. Les multiplications et additions restent rationnelles exactes ; l’affichage EUR est arrondi au centime, avec égalité vers le pair.

Le contrôle présente un **sous-total des contributions valorisables** et ses limites. S’il n’existe aucune contribution calculable faute de données, il affiche « Non calculable ». Les valeurs globales de parts ne sont jamais ajoutées une seconde fois à leurs sous-jacents. Un chemin direct indépendant vers une filiale reste cependant conservé.

Pour interpréter le contrôle des attributions, vérifiez les points suivants.

- Le contrôle des attributions porte sur le registre des actifs. Consultez le bilan financier pour intégrer les dettes, les créances et les comptes. [Comptes, financements et bilan](comptes-et-financements.md)
- Les estimations en devises gardent leur valeur d’origine. La conversion utilise les taux manuels disponibles à la date de référence. [Devises, dates et historique](devises-et-historique.md)
- Les droits, appartenances, méthodes et estimations suivent leurs dates et les versions connues. Les profils peuvent aussi être associés à des périodes.
- Renseignez la responsabilité des associés et la couverture des sources lorsque ces éléments sont connus.

Pour enregistrer un prêt, ouvrez **Autres biens et dettes → Dettes et financements** et créez un seul contrat pour le prêt et la dette correspondante. [Financements et éliminations internes](comptes-et-financements.md)

## Corrections et archivage

Chaque création/correction porte un motif. Une correction exige la révision que vous avez ouverte ; si une version plus récente existe, le coffre refuse l’écriture. Recharger puis rouvrir la fiche permet de comparer et reprendre la modification. L’échec ne laisse ni demi-enregistrement ni audit de mutation validée.

Archiver une personne, une société ou un actif ne clôture ni ses droits ni sa période familiale. Utiliser la fin d’un intervalle pour représenter un changement économique. Une annulation de relation ou d’estimation est une correction de la connaissance ; elle conserve les versions précédentes. Une identité encore référencée ne peut pas être annulée en laissant des relations actives orphelines.

Dans **Date et historique**, le sélecteur **Connaissances disponibles** permet de consulter les versions disponibles après une modification enregistrée. Une ancienne séquence s’ouvre en lecture seule. [Comprendre les dates et l’historique](devises-et-historique.md)

## Sauvegardes et anciennes données

Le ZIP complet contient également tous les enregistrements du registre, ses révisions, l’audit et les pièces originales. Le filtre de contexte ne limite pas cette sauvegarde. Les identifiants et les résultats du contrôle sont vérifiés après restauration dans les tests.

Les coffres et archives SQLite des schémas 1 à 5 sont vérifiés dans leur format d’origine, puis mis à niveau vers le schéma 6 sur une copie. Le résumé de l’archive annonce la mise à niveau avant la confirmation. [Mise à jour et restauration](installation-linux.md)
