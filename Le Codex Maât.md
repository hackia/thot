**Standard 3.0 (Minéral)**

Amentys n'est pas un UNIX. C'est un système basé sur la physique des forces et l'équilibre, pas sur des états binaires morts. Voici comment lire et écrire la Loi.

## I. Le Concept Fondamental : L'Équilibre (Helix)

Dans un ordinateur classique, `1 - 1 = 0` (Le Vide).

Dans Amentys, **la soustraction n'existe pas**. Il n'y a que l'union de forces opposées.

### La Structure de la Matière

Nous n'utilisons pas de `int` ou de `float`. Nous utilisons le **HELIX**.

C'est une structure à deux canaux (Double Base 4) :

- **Ra (Rouge) :** La Force Positive / L'Action.
    
- **Apophis (Bleu) :** La Résistance / Le Frottement / Le Négatif.
    

**Le Zéro Chaud :**

Si `Ra = 50` et `Apophis = 50`, la résultante visuelle est `0`.

Mais le système sait que ce n'est pas du vide. C'est une **Tension Critique**. L'information de la lutte est conservée en mémoire.

---

## II. La Syntaxe (Maât 3.0)

Le code ressemble à un mélange d'Assembleur (gestion de registres) et de Python (lisibilité, indentation).

- **Tout en minuscules.**
    
- **Pas de point-virgule `;` en fin de ligne.**
    
- **Blocs définis par l'indentation.**
    

### Les Registres Sacrés (Les Vases)

On ne manipule pas la mémoire brute. On passe par des registres typés :

- `%ka` (Le Bras) : Pour les Forces et les Maths (Helix/Tetra).
    
- `%ba` (L'Âme) : Pour les Pointeurs et la Mémoire.
    
- `%ib` (Le Cœur) : Pour le résultat des Jugements (Wdj).
    

---

## III. Le Dictionnaire des Verbes

Voici les instructions processeur traduites en concepts Maât.

|**Verbe Maât**|**Équivalent Classique**|**Signification Profonde**|
|---|---|---|
|**`henek`**|`MOV / LOAD`|**Offrir.** Charge une donnée dans un registre.|
|**`sema`**|`ADD`|**Unir.** Fusionne deux forces. (Remplace ADD et SUB).|
|**`wdj`**|`CMP`|**Peser.** Compare deux valeurs. Le résultat va dans `%ib`.|
|**`per`**|`PRINT`|**Sortir.** Affiche ou émet une donnée vers l'extérieur.|
|**`jena`**|`CALL`|**Invoquer.** Appelle une autre fonction (`sokh`).|
|**`kheper`**|`NEXT / YIELD`|**Devenir.** Itérateur atomique (Charge + Avance + Vérifie).|
|**`returne`**|`RET`|**Rendre.** Renvoie le verdict final.|

---

## IV. Le Flux de Vie (Contrôle)

Nous n'avons pas de `if/else` ou de `while`. Nous avons des Cycles et des Jugements.

### 1. Le Jugement (`duat`)

Après une pesée (`wdj`), le monde se divise en deux réalités.

- **`maat`** : L'Ordre (Vrai, Positif, Équilibré, Succès).
    
- **`isfet`** : Le Chaos (Faux, Négatif, Déséquilibré, Échec, Fin de liste).
    

Extrait de code

```
wdj %ka, 0      ;; On pèse la force
duat maat       ;; Si >= 0
    per "Positif"
duat isfet      ;; Si < 0
    per "Négatif"
```

### 2. Le Cycle de Vie (`ankh`)

Une boucle est une Vie. Elle tourne jusqu'à ce qu'on la coupe.

- **`ankh`** : Début de la vie (Boucle infinie).
    
- **`sena`** : La Mort (Break/Sortie).
    
- **`neheh`** : L'Éternité (Continue/Suivant).
    

---

## V. Le Type de Retour : WDJ vs Brut

Une distinction cruciale pour la performance et la sécurité.

1. **Fonction à Risque (Action) -> Renvoie `Wdj`**

    Dès qu'on touche au disque, au réseau ou à la mémoire, ça peut rater. On doit renvoyer un statut : `Wdj.Succes`, `Wdj.Echec`, `Wdj.Alerte`.
    
    - _Ex : `sokh ouvrir_porte(): Wdj`_

2. **Fonction de Vérité (Calcul) -> Renvoie `Helix`**
    
    Les maths sont absolues. `1+1` ne peut pas échouer. On renvoie la donnée brute pour la vitesse pure.
    
    - _Ex : `sokh calculer_orbite(): Helix`_


---

## VI. Exemple Concret : Le Gardien

Voici un programme complet qui surveille une pression. Il illustre tous les concepts.

Extrait de code

```
;; Intention : Surveiller une cuve. Si la pression monte trop, on alerte.
;; Standard : Maât 3.0

sokh surveiller_cuve(capteur: Hapi): Wdj

    ;; On prépare le seuil critique (100 bars)
    henek %seuil, 100

    ;; ANKH : La boucle de surveillance infinie
    ankh
        ;; KHEPER : On lit la prochaine valeur du capteur dans %ka
        kheper %ka, capteur
        
        ;; Si le capteur ne répond plus (ISFET), on coupe
        duat isfet
            per "Capteur déconnecté !"
            sena

        ;; WDJ : On pèse la pression reçue face au seuil
        wdj %ka, %seuil
        
        ;; DUAT MAAT : La pression dépasse ou égale le seuil (Danger)
        duat maat
            per "Alerte : Surpression !"
            returne Wdj.Alerte
            
        ;; DUAT ISFET : La pression est sous le seuil (Calme)
        duat isfet
            per "Pression stable..."
            ;; On continue la boucle (implicite)

    ;; Si on sort de la boucle (sena)
    returne Wdj.Echec
```
	