Le contrôle utilise `syn` et le compilateur TypeScript pour comparer les formes
JSON des DTO et les paramètres/résultats des commandes Tauri. Il vérifie aussi
le registre des commandes et les valeurs Serde d’`ErrorCode`.

Lancer depuis la racine :

```sh
python3 scripts/check-contracts.py
python3 scripts/test-check-contracts.py
```

Les versions Rust sont déjà présentes dans le verrou de l’application. Le
petit binaire d’extraction a son propre verrou ; Cargo travaille hors ligne et
écrit ses produits dans `.local/contract-check-target`. Aucun service ni coffre
utilisateur n’est ouvert.

La comparaison conserve les listes, dictionnaires, objets imbriqués, champs
obligatoires et unions avec `null`. `serde(default)` autorise un ancien document
en entrée mais ne rend pas un champ facultatif dans le JSON émis. Les unions de
littéraux TypeScript sont comparées à leur primitive Rust (`String`/`bool`) ;
elles ne constituent donc pas une preuve de validation métier des valeurs.
Les alias TypeScript portant le nom d’une enum Serde sont comparés par leurs
valeurs exactes, par exemple `DocumentStatus`. Cette liste d’enums vient de
l’AST Rust ; ajouter ou retirer une variante d’un seul côté échoue en CI.

Les fonctions de sérialisation personnalisées et les options Serde non prises
en charge, dont `flatten`, font échouer le contrôle explicitement. Les macros
de registres connues sont analysées avec `syn` et leur définition est protégée
par une empreinte de jetons : une modification impose de revoir leur expansion
et le contrôleur. Une syntaxe non reconnue n’est jamais ignorée.
