# Métaux et sociétés — utilisation de la phase 7

La phase 8 complète ce guide avec les [versions documentaires, associations multiples et exports réimportables](documents-et-portabilite.md). Les limites de portabilité indiquées comme futures ci-dessous correspondent à l’état de la phase 7.

Guide du **11 novembre 2026, Europe/Paris**. Les exemples illustrent les règles ; ils ne sont pas injectés dans le coffre. Les [preuves de validation](../historique/validation-phase-7.md) distinguent calcul pur, persistance, interface et parcours Linux réel.

La date économique **D** et les connaissances **K** restent celles du bandeau commun. Les rapports utilisent les sources effectives à D connues à K. Toute séquence K fixe, y compris `0`, donne une consultation en lecture seule. Les sommes et quantités sont saisies en texte décimal ; le moteur conserve des fractions exactes et arrondit les montants à l’affichage.

## Créer un lot physique et confirmer son stock

Dans **Or et argent → Lots**, choisir **Créer un lot de métal**. Rattacher le lot à un actif déjà créé dans **Patrimoine** ; **Gérer les actifs et leurs propriétaires** permet de préparer cet actif et ses droits. Un lot complète un actif unique. Il ne peut pas utiliser un actif déjà spécialisé comme compte ou bien immobilier.

Renseigner le métal, la forme, la désignation, le poids brut **d’une unité physique**, l’unité de poids, la pureté, le début du suivi, la devise des coûts, le titulaire initial et le lieu initial. La pureté est une fraction décimale strictement supérieure à `0` et inférieure ou égale à `1`, par exemple `0,9`, et non un pourcentage à interpréter. La quantité restante peut être nulle après une vente complète ; cette possibilité ne s’applique pas à la pureté. Les unités sont **Gramme**, **Kilogramme** et **Once troy (31,1034768 g)**. Une once ordinaire n’est pas une once troy.

Les conversions sont exactes : `1 kg = 1 000 g` et `1 once troy = 31,1034768 g`. Le poids fin est la quantité restante multipliée par le poids brut unitaire en grammes, puis par la pureté. Deux pièces de `31,1034768 g`, de pureté `0,9`, représentent exactement `55,98625824 g` de métal fin.

Le **Titulaire juridique initial du lot** est une information descriptive d’origine. Les droits juridiques et économiques datés de l’actif déterminent ses propriétaires courants et son attribution familiale ou individuelle. Le titulaire initial ne devient pas implicitement propriétaire de la totalité du lot et ne change pas lors d’un transfert de droits.

Après création, actif, métal, poids, unité, pureté, début du suivi et devise des coûts sont fixes. Une correction descriptive ne doit pas réinterpréter les anciens mouvements dans une autre unité. L’archivage conserve le lot et son historique ; il ne crée aucune vente.

Créer la fiche ne confirme aucun stock. Choisir **Confirmer le stock d’ouverture**, saisir une quantité et éventuellement son coût historique total, puis enregistrer à la date exacte de début du suivi. Une quantité explicitement nulle se saisit `0`. Un coût vide reste inconnu ; il ne devient ni zéro ni une estimation déduite du cours actuel. L’ouverture historique n’est pas un nouvel achat et ne crée aucun débit bancaire.

## Achats, ventes partielles et inventaire

Dans **Mouvements**, choisir **Enregistrer un mouvement de métal**. Les quantités d’ouverture et d’achat sont positives, les quantités vendues négatives. Pour une vente partielle d’une pièce sur un stock de deux, saisir `−1`, puis le prix total réellement convenu et les frais distincts. Les corrections rétroactives ne peuvent pas rendre le stock négatif à une date intermédiaire.

Le **Coût ajouté dans la devise du lot** représente le coût total de l’entrée, dans la devise historique fixe du lot. Une vente et une sortie d’inventaire retirent exactement leur part du coût restant du lot homogène. Il n’y a pas de FIFO global ni de réévaluation du coût au cours actuel. Avec un stock de deux unités dont le coût restant est `2 000 EUR`, la vente d’une unité alloue `1 000 EUR` à cette sortie et laisse `1 000 EUR` au stock.

Les achats et ventes décrivent séparément prix avant frais, frais réels, devise, **Bénéficiaire ou payeur du cash** et éventuelle contrepartie. Le destinataire du cash est choisi explicitement, indépendamment du titulaire initial du lot. La vente ne rajoute pas son produit à la valeur du métal conservé : le stock diminue et le cash réellement reçu figure dans le journal.

Le **Traitement de la trésorerie métal** propose quatre cas :

- **Source historique non rapprochée** : conserver la source avec un motif, sans inventer de compte ou mouvement. Un flux réel manquant peut empêcher un bilan complet.
- **Créer l’opération bancaire avec le mouvement** : choisir le compte et saisir son montant total signé réel, frais compris. La source, le journal et ses liens sont enregistrés ensemble.
- **Rapprocher une opération bancaire existante** : sélectionner sa révision, ses lignes et les montants signés à affecter à cette source. Aucun second crédit ou débit n’est créé.
- **Corriger l’opération bancaire d’origine avec la source** : conserver le journal d’origine, modifier explicitement les lignes concernées et les montants rapprochés. Toutes les autres lignes et sources partageant le journal restent contrôlées.

Par exemple, une vente de prix `1 500 EUR` avec `10 EUR` de frais correspond à un crédit net de `1 490 EUR`. Si son coût alloué est `1 000 EUR`, le gain réalisé est `490 EUR`. Le gain reste un calcul ; il ne produit pas un deuxième encaissement.

Une **Correction d’inventaire** est une entrée ou sortie physique justifiée. Une entrée peut nécessiter un coût explicite ; une sortie retire le coût proportionnel. Ce mouvement n’est pas un substitut à une vente ayant réellement donné lieu à paiement.

## Transférer le lieu ou les droits

Un **Transfert du lieu de stockage** déplace le lot entier. Saisir une variation de quantité de `0`, son nouveau pays et son nouveau lieu. Il ne modifie ni le stock, ni le coût, ni le revenu, ni la banque. Le rapport restitue le lieu applicable à D. Les autres mouvements indiquent leur lieu de provenance.

Un lot suivi dans cette version a un seul lieu à chaque date. Il n’existe pas de commande de scission automatique avec filiation de lots et coût transféré. Des stocks répartis physiquement doivent être suivis comme lots identifiés distincts avec leurs sources ; ne pas déplacer artificiellement la totalité du lot si seule une partie a changé de lieu.

Dans **Lots**, sélectionner le lot puis **Transférer les droits du lot**. Choisir la date et décrire la **nouvelle répartition complète** des propriétaires avec fractions juridiques et économiques. Pour transférer la moitié d’un membre à un autre, conserver le premier propriétaire à `1/2` et ajouter le second à `1/2`, en renseignant les droits concernés.

**Enregistrer le transfert des droits** clôture les droits actifs à la date du transfert et ouvre les nouveaux dans le même changement. Les fractions juridiques et économiques doivent être clarifiées avant ce transfert ; leurs sommes sont conservées. Les périodes futures et références courantes sont contrôlées. La quantité physique et la valeur du lot entier restent identiques. Un transfert entre membres réorganise leurs parts ; il ne crée pas de gain familial ni de cash. Les anciens droits restent consultables à leur K.

## Cours, prime, décote et valeur retenue

Dans **Cours et primes → Ajouter un cours métal**, renseigner métal, date, prix, devise, unité de cotation et source. Le prix porte sur le **métal fin**, par gramme, kilogramme ou once troy. Le calcul retient un cours antérieur ou égal à D ; il n’utilise pas de cours futur. Les cours anciens ou contradictoires sont signalés avec le choix déterministe et ses références.

Avec `55,98625824 g` fins et un cours de `60 EUR/g`, la valeur intrinsèque exacte vaut `3 359,1754944 EUR`, affichée `3 359,18 EUR`. Le prix de revient est une autre source, et le cours ne le remplace pas.

**Définir une prime de revente** demande sa date, sa convention, sa valeur, la devise, une décote ou des frais estimés par unité physique et une source. La prime est soit un **Montant par unité physique**, multiplié par le stock restant, soit une **Fraction décimale de la valeur intrinsèque**. La décote reste exprimée par unité physique dans la devise déclarée.

Une prime absente n’est pas présumée nulle. Pour confirmer qu’aucune prime ni décote ne s’applique, saisir explicitement `0` pour chacune dans une source datée et motivée. L’estimation de revente vaut intrinsèque + prime − décote ; si les composantes manquent ou donnent une revente négative, cette valeur reste non calculable.

La fiche choisit la **Valeur retenue dans le bilan** : **Valeur intrinsèque**, ou **Estimation de revente avec prime et décote explicites**. Le premier choix peut être valorisé sans prime connue ; le second exige les conditions de revente. Les deux valeurs sont présentées séparément. Le bilan et le contrôle d’attribution utilisent cette source du lot à la place d’une estimation générique du même actif.

Dans **Contrôler un lot**, sélectionner le lot puis **Calculer le lot de métal**. Le contrôle porte sur le **lot entier** ; le bilan applique les droits du périmètre choisi. Il distingue quantité, poids fin, coût restant, intrinsèque, prime, décote, revente, valeur retenue, gains réalisés et latents. Les allocations de coût aux sorties, fractions exactes, cours, conversions et révisions sources restent consultables. Le coût historique se convertit aux dates de ses entrées ; la valeur du stock utilise les cours de D.

## Conserver les fiches et méthodes des sociétés

Dans **Sociétés**, **Fiches et participations** conserve le registre existant : profils, propriétaires, participations et méthode datée **détaillée** ou **valeur globale des capitaux propres**. Les [règles de propriété](famille-et-propriete.md), [dates et connaissances](devises-et-historique.md), [couvertures et financements](comptes-et-financements.md) restent applicables.

La valeur globale réellement retenue se saisit par les valorisations du registre et sa méthode explicite. Un chiffre d’affaires, une trésorerie indicative ou une estimation de gestion ne remplace pas cette valorisation par synchronisation implicite.

## Saisir des observations périodiques

Choisir **Indicateurs et dividendes → Observations périodiques → Ajouter une observation de société**. Renseigner société, fréquence, devise, début et fin exclue de période. Le début est le premier jour d’un mois ; une période dure un, trois ou douze mois. Un exercice annuel peut être décalé.

Les champs sont facultatifs : chiffre d’affaires, charges d’exploitation, résultat d’exploitation déclaré, résultat net, trésorerie indicative, dettes indicatives, créances importantes, dividendes déclarés indicatifs et valeur globale indicative. Une case vide reste **Non renseigné** ; saisir `0` uniquement si ce zéro est confirmé. Les résultats et la trésorerie peuvent être signés selon la source ; les autres montants respectent leurs contraintes de signe.

Les périodes de même société et fréquence ne se chevauchent pas, **même si les devises diffèrent**. Changer de devise ne permet pas de créer une observation concurrente pour la même période. Des observations mensuelles et annuelles peuvent coexister, mais elles ne sont pas additionnées. Les montants sont des observations de pilotage ; ils ne créent aucun compte, dette, créance, bénéfice reçu personnellement ou deuxième valeur de société dans le bilan.

Dans **Consulter une société**, choisir une société et éventuellement une fréquence, puis **Calculer le contrôle de société**. Seules les périodes terminées à D sont analysées. Le rapport compare des périodes adjacentes de fréquence et devise compatibles. Il expose la différence de montant et sa variation en pourcentage ; une base nulle laisse le pourcentage non calculable. Une lacune ou une période incompatible ne produit pas de tendance inventée.

Le contrôle rapproche aussi chiffre d’affaires moins charges du résultat d’exploitation déclaré, sans l’écraser. Les tableaux présentent les participations, actifs directement détenus, comptes suivis, financements et sources de distribution à la même connaissance K. Les soldes bancaires sont les **soldes entiers** des comptes dont la société est titulaire juridique ; les droits économiques restent ceux du registre. Les financements présentent déjà la quote-part contractuelle de la société. Ce contrôle n’est ni un bilan comptable général ni un total patrimonial à additionner à la famille.

## Déclarer puis payer un dividende

Dans **Distributions réelles → Enregistrer une distribution de société**, choisir société, bénéficiaire, date, devise et montant. **Dividende déclaré, sans banque** conserve une décision. Il ne crée ni encaissement ni créance ou dette supplémentaire dans le bilan.

**Dividende réellement payé** est une autre source, éventuellement reliée à la déclaration qu’il règle. Le bénéficiaire et le montant sont explicites ; ils ne se déduisent pas automatiquement du résultat ou du capital. Plusieurs paiements peuvent régler une déclaration dans la limite de son montant aux dates concernées.

Le **Traitement de la trésorerie du dividende** permet :

- **Rapprocher une opération déjà enregistrée** : sélectionner la révision du journal et ajouter les côtés avec leurs lignes et montants signés. Le côté société est un débit, celui du bénéficiaire un crédit.
- **Créer le journal avec le paiement** : choisir les comptes réellement suivis de la société et du bénéficiaire, dans la même devise. Lorsque les deux sont suivis, une seule commande écrit les deux lignes et la source.
- **Corriger l’opération liée avec le paiement** : réviser le journal d’origine avec toutes ses lignes, les montants rapprochés et la source dans le même changement. Aucune deuxième opération bancaire n’est substituée à l’origine.
- **Conserver une source historique non rapprochée** : expliquer les côtés non suivis. Le moteur conserve l’incomplétude et ne crée pas de banque extérieure par hypothèse.

Un paiement déjà lié ne peut pas être transformé en déclaration ni remplacé par un nouveau journal. En cas de conflit de séquence ou de révision, les choix et montants restent dans le formulaire ; recharger les sources et vérifier leurs dépendances avant une nouvelle correction.

Un dividende entre une société familiale et un membre redistribue la trésorerie réellement suivie. Il ne constitue pas un gain familial à hauteur interne. Une société évaluée globalement exige toujours ses couvertures et une valorisation adéquate ; le résultat de pilotage ne justifie pas une mise à jour automatique de ses capitaux propres.

Lorsqu’un dividende, un achat/une vente de métal ou un transfert de droits traverse le périmètre d’une société globale, une estimation antérieure à l’opération ne permet plus de certifier le net. Saisir une valeur globale tenant compte de l’opération. Le moteur laisse le bilan incomplet jusque-là, sans inventer la variation des parts. Les deux côtés d’un mouvement interne au même périmètre opaque se compensent avant ce contrôle.

Le pont de variation d’**Historique** reste consacré aux estimations du registre. Il exclut les stocks métal calculés et n’utilise pas leurs anciennes estimations génériques comme substitut. Son avertissement explicite empêche de prendre ce pont partiel pour une performance ou une variation du bilan financier complet.

## Corrections, sauvegarde et limites

Les commandes **Modifier** et **Annuler l’enregistrement** exigent un motif et conservent les anciennes révisions. Le stock, les déclarations réglées, capacités de lignes bancaires, signes, dates, devises, propriétaires et références exactes restent contrôlés. Une correction incompatible est refusée avant validation du changement.

La sauvegarde complète inclut les lots, mouvements, cours, primes, droits, périodes de société, distributions et liens, avec leur historique. Les anciens coffres et archives des schémas 1 à 5 sont mis à niveau sur copie vers le schéma 6 ; leur source reste conservée. Les liens documentaires métier complets, CSV/XLSX et réimportation exhaustive restent la phase 8. Les tableaux de bord graphiques globaux restent la phase 9 ; les tableaux de contrôle actuels n’affichent que les sources réellement saisies.

Ces fonctions n’assurent ni suivi fiscal des plus-values, ni comptabilité générale, TVA, facturation ou paie. Aucun cours automatique ou service externe n’est requis. Une couverture complète dépend toujours de l’inventaire réellement vérifié par l’utilisateur ; des données absentes ne deviennent pas des zéros.
