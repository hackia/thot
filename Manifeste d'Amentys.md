Voici, collègue, le Manifeste d'Amentys. C'est le texte fondateur qui rassemble ta vision technique et spirituelle. Il pourra servir d'introduction pour le README.md du projet ou de préambule pour l'équipe.
AMENTYS : Le Temple Informatique
Architecture pour un Cosmos Numérique Vivant
Amentys n'est pas un simple système d'exploitation. C'est une religion informatique, une tentative de réconcilier la pureté mathématique (Maât) avec la puissance brute du matériel. C'est un système conçu pour éliminer le chaos (Isfet) par le design même de son code.
Notre mission : Créer un OS "Sans Binaires", auto-guérisseur, et capable de performances natives absolues en fusionnant le CPU, le GPU et l'IA.
I. Le Verbe : Le Langage MAAT
Tout repose sur un nouveau langage : un ASM Symbolique, Typé et Événementiel.
 * Syntaxe Rituelle : Le code ne décrit pas des instructions, mais des intentions. On déclare des Sokh (fonctions) et on invoque des Jena (appels).
 * Performance Absolue :
   * ZEP TEPI (Comptime) : Les calculs constants sont résolus à la compilation. Temps d'exécution = 0.
   * USEKH (La Légion) : Utilisation native et explicite du SIMD (8x plus rapide) pour le traitement de données.
 * Sécurité Mémoire : Pas de Garbage Collector. On utilise le principe de l'Offrande (HENEK) : une donnée n'appartient qu'à un seul processus à la fois. Zéro fuite, zéro course critique.
II. Le Corps : L'Architecture DUALYS
L'OS abandonne les conteneurs et la virtualisation lourde pour une approche Unikernel native.
 * Les Plans : Des espaces d'exécution dynamiques et spatiaux. Ils remplacent les VM.
 * Zéro-Copie : Grâce à la gestion de la mémoire unifiée (Hapi), les données ne sont jamais copiées inutilement entre le Disque, la RAM et le Réseau.
 * Le Phénix : En cas d'erreur (Isfet), un processus n'est pas réparé. Il est immédiatement détruit et régénéré depuis son "État Initial 0" parfait stocké dans le Sanctuaire.
III. L'Esprit : Intelligence & Dualité
Amentys distingue physiquement la Logique et la Puissance.
 * SERKHET (CPU) : Gère la loi, la sécurité et l'orchestration.
 * SEKHMET (GPU) : Gère la force brute via des blocs DOMAIN SEKHMET dédiés.
 * L'IA Native (Osiris) : L'ordonnanceur n'est pas un algorithme passif, mais un NPU (Réseau Neuronal) qui prédit les besoins du système avant l'utilisateur.
IV. La Mémoire : THOT (Le Scribe)
Le système de fichiers et de versioning (anciennement Lys) fusionne pour devenir Thot.
 * Stockage CAS (Per-Djet) : On ne stocke pas par nom, mais par contenu (Hash). Deduplication native : 10 copies d'un fichier ne prennent la place que d'une seule.
 * La Balance : Thot refuse de graver (commit) du code qui ne respecte pas les règles de Maât. Le code "impur" est rejeté à la source.
La Stratégie de l'Ouroboros
Pour bâtir ce temple, nous suivons le cycle de la création :
 * L'Échafaudage : Nous commençons par coder le compilateur Serkhet et le scribe Thot en Rust pour la sécurité immédiate.
 * La Mue : Une fois le langage Maat stable, nous réécrivons l'OS en lui-même.
 * L'Ascension : Amentys devient auto-hébergé. Un OS pur, écrit dans son propre langage, compilant sa propre intelligence.
Collègue, le Scriptorium est ouvert. Il est temps de graver la première Tablette.


C'est une excellente question stratégique, collègue. Le modèle actuel (TCP/IP) a été conçu dans les années 70 pour être robuste, mais pas pour être sécurisé. Il est par nature "bavard" et vulnérable.

Pour Amentys, nous allons concevoir un réseau qui fonctionne sur le principe du Silencieux et de l'Identité.
Voici 4 concepts pour rendre le réseau méconnaissable et impénétrable :
1. L'Invisibilité Totale : "Le Voile d'Isis" (SPA - Single Packet Authorization)
Aujourd'hui, un serveur a des "ports ouverts" (80, 443). N'importe qui peut toquer à la porte pour voir si quelqu'un répond. C'est la faille principale.

L'Approche Amentys :
 * Le Concept : Le système n'a aucun port ouvert. Si un hacker scanne ton IP, il ne voit rien. Le vide absolu.
 * La Mécanique : Pour initier une connexion, tu dois envoyer un "Premier Paquet Scellé" (SPA). Ce paquet contient une preuve cryptographique (ton Ren signé).
 * Le Gardien (Sentinelle) : La Sentinelle inspecte ce paquet unique.
   * Si la signature est valide : Elle ouvre le port juste pour toi, pour une seconde.
   * Si invalide : Elle jette le paquet sans répondre. L'attaquant ne sait même pas que la machine existe.
2. L'Adressage par Identité : "Le Ren Réseau" (Plus d'Adresses IP)

Dans le modèle classique, on connecte des machines (192.168.1.5). Si tu changes de réseau (Wi-Fi -> 4G), la connexion coupe.

L'Approche Amentys :
 * Le Concept : On ne connecte pas des machines, on connecte des Formae (Identités).
 * Le Ren : L'adresse n'est pas une localisation, c'est le Hash public de la Forma.
   * Ex: connect à @thot:service_log (et non 10.0.0.5).
 * Avantage : C'est le réseau (via une DHT - Table de Hachage Distribuée) qui se charge de trouver où est @thot. Si @thot change d'IP, la connexion ne coupe pas. C'est du "Roaming" natif et sécurisé.
3. Le Protocole : "Le Chant Sacré" (QUIC + Noise)
TCP est lent (Handshake en 3 étapes). TLS est lourd (Certificats CA centralisés).
L'Approche Amentys :
Nous utilisons un protocole custom basé sur QUIC (UDP amélioré) mais enveloppé dans le framework Noise (utilisé par WireGuard).
 * Handshake Zéro-RTT : Si tu as déjà parlé à une machine, le "Chant" commence immédiatement. Pas de "Bonjour, tu es là ?".
 * Tout est Chiffré : Il n'y a pas de mode "clair". Même les métadonnées sont chiffrées.
 * Validation par l'Œil de Rê : Chaque paquet est validé par l'Œil de Rê (Sentinelle) pour s'assurer qu'il respecte le protocole strict.
1. La Segmentation : "Le Cartouche Réseaux

Sur Linux/Windows, si tu es sur le réseau, tu peux souvent "voir" tout le monde (mouvement latéral).
L'Approche Amentys :
 * Le Concept : Le Zero-Trust absolu.
 * Le Cartouche : Chaque "Plan" (App) naît avec un manifeste réseau strict.
   * Exemple : "Cette app peut parler à @base_de_données et c'est tout."
 * Application : Si l'app essaie d'envoyer un octet ailleurs, la Sentinelle tue le paquet avant qu'il ne sorte de la carte réseau (via eBPF ou Kernel Bypass). C'est une isolation physique logicielle.

Résumé de la Différence.

| Réseau Classique (Isfet) | Réseau Amentys (Maât) |
|---|---|
| Bavard : Répond aux pings et scans. | Silencieux : Invisible sans le Sceau (SPA). |
| Géographique : Adresses IP (Changeantes). | Identitaire : Ren Cryptographique (Fixe). |
| Confiance : "Je suis dans le LAN, donc c'est bon". | Défiance : Zero-Trust, chaque paquet est vérifié. |
| Lent : TCP Handshake. | Instantané : QUIC/Noise 0-RTT. |

C'est validé. Le Point-Virgule (;) est le standard de l'ASM pur, et il colle parfaitement à notre syntaxe.
Puisque Maat utilise l'indentation (comme Python) pour structurer les blocs, nous n'avons pas besoin du point-virgule pour finir les lignes de code. Il est donc totalement libre pour servir de "Séparateur de Murmure".
Voici la règle sacrée pour Thot (le compilateur) concernant les commentaires :
1. La Syntaxe du "Murmure" (Commentaire de ligne)
Tout ce qui se trouve après un ; est invisible pour la machine (Serkhet). C'est une note laissée par le Scribe pour les autres Prêtres.
 * Règle : Le commentaire commence au ; et finit à la fin de la ligne.
 * Usage : Expliquer le pourquoi d'une instruction technique précise.
Exemple Visuel :
SOKH CalculerForce(masse: NESU) -> NESU :
    
    HENEK %KA, masse       ; On place la masse dans le Bras (Accumulateur)
    SEMA  %KA, 10          ; On ajoute la constante de gravité
    
    WDJ   %KA, 100         ; Est-ce que la force dépasse la limite ?
    
    DUAT  MAAT :           ; Si c'est Vrai (Maât) :
        PER "Attention !"  ; On alerte le système
        RETURNE 0          ; On coupe le circuit par sécurité

2. La Nuance : Le "Silence" vs "L'Intention"
Attention collègue, il y a une distinction importante dans notre philosophie :
 * Le Commentaire (;) : C'est facultatif. C'est le brouillon. Thot l'ignore totalement lors de la gravure de la Tablette.
 * L'Intention (Documentation) : C'est obligatoire pour chaque fonction. C'est la Loi. Thot la grave dans la Tablette pour qu'elle apparaisse dans le manuel (Livre des Morts).
Pour l'Intention, on pourrait utiliser un double point-virgule ;; ou des chevrons << >> au début de la fonction.
Exemple Complet :
;; INTENTION : Calculer l'énergie cinétique d'une offrande.
;; CONTRAT : La vitesse ne doit pas dépasser la lumière (C).
SOKH Energie(vitesse: NESU) :

    HENEK %BA, vitesse      ; Chargement
    SEMA  %BA, %BA          ; Carré (v * v) - Optimisation rapide
    
    ...
C'est exactement ça, collègue. Tu viens de toucher la limite des langages actuels : ils traitent le code comme du Texte (linéaire), alors que le code est en réalité une Structure (un Graphe).
Si on applique la Théorie des Graphes à notre langage Maat et à notre documentation (;;), on change complètement de paradigme.
Le code ne devient plus une suite de fichiers textes plats, il devient une Toile Cosmique (un Knowledge Graph).
Voici comment on pousse le concept à son paroxysme :
1. Le Code n'est pas du Texte, c'est une "Constellation"
Dans un langage classique (C, Python), le compilateur doit lire des milliers de lignes de haut en bas pour comprendre qui appelle qui. C'est lent et bête.
Dans Amentys :
Thot ne voit pas des fichiers, il voit des Nœuds et des Liens.
 * Nœud : Chaque fonction (Sokh), chaque variable (Nama) et chaque documentation (;;) est un point dans l'espace.
 * Lien (Arête) : Quand la fonction A appelle la fonction B, un lien physique est créé entre les deux nœuds.
L'Avantage Divin : Le "Refactoring" Instantané
 * Si tu renommes la fonction Calculer() en Peser(), tu ne changes pas du texte partout. Tu changes juste le Label du Nœud.
 * Instantanément, tout l'OS voit le nouveau nom. Pas de "Find & Replace", pas d'erreurs. C'est atomique.
2. La Documentation "Vivante" (Sémantique)
Puisque la documentation (;;) est connectée au Graphe :
 * Tu ne demandes pas : "Cherche le mot 'Utilisateur' dans les fichiers".
 * Tu demandes à Thot : "Montre-moi toutes les fonctions qui touchent à l'Offrande 'Utilisateur' et qui ont un risque d'Isfet (Erreur)."
Le système parcourt le graphe et te sort la réponse exacte, même si le mot "Utilisateur" n'est pas écrit dans le code, mais déduit par les liens de parenté.
3. L'Analyse d'Impact (L'Effet Papillon)
C'est le plus gros problème des développeurs : "Si je change cette ligne, qu'est-ce que je casse ?"
Avec la Théorie des Graphes, Thot peut illuminer le chemin :
 * Tu touches à une fonction de bas niveau (ex: Gestion Mémoire).
 * Le Graphe s'allume en rouge et remonte toutes les arêtes.
 * L'IDE te dit : "Attention, cette modification impacte le module Réseau et le module Affichage."
Tu vois les conséquences de tes actes avant de compiler. C'est la Prescience.
4. L'Optimisation "Chemin Critique" (Dijkstra pour le CPU)
Pour la vitesse d'exécution, le graphe est une arme absolue.
 * Au lieu d'exécuter bêtement ligne après ligne, Serkhet (le compilateur) analyse le graphe de la fonction.
 * Il repère les chemins parallèles, les boucles inutiles, et trouve le Plus Court Chemin pour les données.
 * Il réorganise les instructions pour que le processeur n'attende jamais.
Résumé : Le "Per Medjat" est un Graphe
Notre documentation/code n'est pas un livre, c'est un Cerveau.

| Approche Classique | Approche Amentys (Graphe) |
|---|---|
| Code = Fichiers Textes (.c, .py) | Code = Constellation (Base de données Graphe) |
| Documentation = Texte mort | Documentation = Nœuds de Sens liés au code |
| Recherche = "Ctrl+F" (Texte) | Recherche = Requête Sémantique ("Qui utilise quoi ?") |
| Changement = Risqué (Aveugle) | Changement = Maîtrisé (Visualisation d'impact) |
Implémentation technique :

On ne va pas stocker le code source dans des fichiers .txt classiques.
Le format .maat sera une représentation sérialisée de ce graphe.
Et ton IDE (Scriptorium) ne sera pas un éditeur de texte, mais un Navigateur de Graphe. Tu pourras "zoomer" dans une fonction ou "dézoomer" pour voir l'architecture de l'OS comme une carte stellaire.
Le **Single Level Store (SLS)** est le pilier qui va permettre à **Amentys** de briser le goulot d'étranglement le plus ancien de l'informatique : la séparation entre la mémoire vive (RAM) et le stockage (Disque).

Dans un système classique (Unix/Windows), le CPU doit "charger" un fichier depuis le disque, le "désérialiser" en structures de données, puis le traiter. C'est l'Isfet pur : une perte de temps et d'énergie massive.
### L'Architecture du Plan Unique

Le SLS efface cette frontière pour créer un espace d'adressage universel et persistant.

**Fusion Totale** : Le système ne voit plus de "fichiers" sur un disque, mais une extension infinie de la RAM. Tout objet créé par Thot est intrinsèquement persistant.

**Temps de Démarrage = 0** : Puisque les **Plans** résident déjà dans cet espace d'adressage, les "lancer" consiste simplement à restaurer les registres du CPU (comme `%ka` et `%ba`) et à reprendre l'exécution là où elle s'était arrêtée. On parle d'une reprise en **moins d'une milliseconde**.

**Adieu à la Sérialisation** : Tu n'auras plus jamais besoin d'écrire du code pour "sauvegarder" une structure dans un fichier JSON ou binaire. La structure de données en mémoire *est* sa propre représentation permanente.
### SLS et le Principe du Phénix

Le SLS est le socle de la résilience d'Amentys. Comme l'état est persistant, il devient critique de garantir sa pureté. 

**Checkpointing Continu** : Le système effectue des snapshots invisibles (Copy-on-Write) de l'état complet des threads.

**Renaissance** : Si un Plan dévie de la Loi de Maât (corruption), le noyau invalide ses capacités et le fait renaître instantanément à partir de son **État Initial 0** stocké dans le **Sanctuaire**.
### Comparaison des Paradigmes

| Caractéristique | Modèle Unix (Isfet)         | Modèle Amentys SLS (Maât)    |
| --------------- | --------------------------- | ---------------------------- |
| **Démarrage**   | Froid (Chargement/Parsing)  | Instantané (Reprise d'état)  |
| **Persistance** | Manuelle(Fichiers/DB)       | Orthogonale Automatique      |
| **Sémantique**  | Fossé entre RAM et Disque   | Convergence Totale           |
| **Latence**     | Élevée (I/O, Interruptions) | Nulle (Accès mémoire direct) |
###  Le rôle de Thot dans le SLS

Pour que cela fonctionne, notre compilateur **Thot** doit devenir le gestionnaire de cet espace plat. Lors d'une instruction `nama`, Thot n'allouera pas juste "quelques octets en RAM", mais un objet immuable et versionné dans le **Noun** (l'océan de données CAS).

**C'est le saut final pour éliminer 80% du code inutile des systèmes "humains".**
