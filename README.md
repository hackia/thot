# Thot

| Verb      | desc     | action                                                                                                        |
|-----------|----------|---------------------------------------------------------------------------------------------------------------|
| henek     | MOV      | Give Assigns an immediate value or register to a ship (register).                                             |
| sema      | ADD      | Unite: Adds a value to the force contained in a ship.                                                         |
| kheb      | SUB      | Reduce: Subtracts a value from the force contained in a ship.                                                 |
| wdj       | CMP      | Weigh: Compares (weighs) a ship against a value on the Balance of Maat.                                       | 
| ankh      | JE       | Life: Conditional jump to a label if Libra is in balance (Tie).                                               |
| isfet     | JNE      | Chaos: Conditional jump to a label if Libra is broken (Difference).                                           |
| her       | JG       | avoid: Conditional jump if the ship is strictly greater than the value.                                       |
| kher      | JL       | Depth: Conditional jump if the ship is strictly less than the value.                                          |
| her_ankh  | JGE      | Peak: Conditional jump if the ship is greater than or equal to the value.                                     |
| kher_ankh | JLE      | Base: Conditional jump if the ship is less than or equal to the value.                                        |
| neheh     | JMP      | Eternity: Unconditional jump (infinite loop) to a label.                                                      |
| jena      | CALL     | Summon: Calls a ritual (function) and prepares for the return of the code soul.                               |
| return    | RET      | Return: Leaves a ritual to resume the thread of the previous existence.                                       |
| duat      | DB / MOV | Burn: Writes a sentence in RAM with the automatic Sign of Silence (null terminator).                          |
| sena      | MOV      | Collect: Loads data into %ka from a RAM address or a [%ba] pointer.                                           |
| kheper    | MOV      | Embody: Saves the contents of %ka to a RAM address or [%ba] pointer.ka from a RAM address or a [%ba] pointer. |
| henet     | AND      | assemble: Logical operation AND (Bitwise AND).                                                                |
| mer       | OR       | Link: Logical operation OR (Bitwise OR).                                                                      |
| per       | INT 10h  | Manifest: Displays a character, register or phrase on the screen.                                             |
| sedjem    | INT 16h  | Listen: Interrupts time and waits for a keyboard pulse (stored in %ka).                                       |
| wab       | INT 10h  | Purify: Clears the screen and resets the sacred void (Clear Screen).                                          |

| register | description                                                   |
|----------|---------------------------------------------------------------|
| %ka      | Accumulator (EAX) – The seat of mathematical power.           |
| %ib      | Counter (ECX) – The master of time and cycles.ematical power. |
| %da      | Data (EDX) – The servant of inputs/outputs.                   |
| %ba      | Base (EBX) – The finger that points to memory.                |
| %si      | Source (ESI) – The origin of data flows.                      |
| %di      | Destination (EDI) – The arrival of data flows.                |

**Register Levels**
The register name encodes the Helix size. Sizes are total Helix width (Ra + Apophis); each channel is half.

| level   | prefix | total Helix size | per-channel size |
|---------|--------|------------------|------------------|
| Base    | (none) | 8                | 4                |
| Medium  | m      | 16               | 8                |
| High    | h      | 32               | 16               |
| Very    | v      | 64               | 32               |
| Extreme | e      | 128              | 64               |
| Xenith  | x      | 256              | 128              |

Examples: `%ba` (Base), `%mba` (Medium), `%hba` (High), `%vba` (Very), `%eba` (Extreme), `%xba` (Xenith). Same scheme for `%ka`, `%ib`, `%da`, `%si`, `%di`.

Rules: operations between different sizes are rejected, and overflow is a compile-time error.

## Installation of the thot compiler

```bash
cargo install thot
```

### 2. Write your first Maât Law

Create a file named `os.maat`. Here is an example.

```text
wab                             ; Purifie l'écran (Clear Screen)

; --- 1. GRAVURE DU DOGME (Zone sécurisée 4000+) ---
; On inscrit les vérités fondamentales dans la mémoire vive
duat "AMENTYS OS V5 - THEOREM:", 4000
duat "Infinite function in a finite world.", 4100
duat "Iteration progress: ", 4200
duat "Finite world reached. Stability achieved.", 4300

; --- 2. CÉRÉMONIE D'OUVERTURE ---
henek %ba, 4000                 ; Charge l'adresse du titre
jena imprimer_ligne             ; Appelle le scribe d'affichage

henek %ba, 4100                 ; Charge l'adresse du théorème
jena imprimer_ligne

; --- 3. L'EXPANSION INFINIE (Boucle de calcul) ---
henek %ib, 0                    ; %ib sert de compteur d'itération
henek %da, 10                   ; %da définit la limite du "monde fini"

boucle_expansion:
henek %ba, 4200             ; "Iteration progress: "
jena imprimer_mots          ; Affiche sans retour à la ligne

; On affiche un symbole '*' pour chaque itération
henek %ka, 42               ; Code ASCII pour '*'
per %ka

henek %ka, 13               ; Retour à la ligne pour la clarté
per %ka

sema %ib, 1                 ; Incrémente l'itérateur
wdj %ib, %da                ; Compare l'itérateur à la limite du monde fini
kher boucle_expansion       ; Tant que %ib < 10, l'expansion continue


; --- 4. LA STABILITÉ FINALE ---
henek %ba, 4300
jena imprimer_ligne

arret_eternel:
neheh arret_eternel         ; Le système reste en contemplation infinie

; --- RITUEL : IMPRIMER LIGNE (Texte + Saut) ---
imprimer_ligne:
jena imprimer_mots
henek %ka, 13               ; Ajoute un Carriage Return (13)
per %ka
return 0

; --- RITUEL : IMPRIMER MOTS (Lecture RAM) ---
imprimer_mots:
sena %ka, [%ba]             ; Lit l'octet pointé par %ba dans la RAM
wdj %ka, 0                  ; Est-ce le Signe du Silence (fin de chaîne) ?
ankh fin_rituel
per %ka                     ; Envoie le caractère au BIOS
sema %ba, 1                 ; Avance le pointeur vers la lettre suivante
neheh imprimer_mots
fin_rituel:
return 0
```

### 3. Invoke the Scribe (Compilation)

The Thot compiler uses positional arguments:

`thot <source_file> <output_file> <bootloader_mode> <keyboard_layout>`

**To compile as a Bare-Metal Bootloader (Naos):**

```bash
thot os.maat os.bin true qwerty
```

**To compile as an Amentys ELF Executable (Sarcophage):**

```bash
thot os.maat os.elf false qwerty
```

### 4. Run the Universe

To boot your newly created OS image in a virtual machine:

```bash
qemu-system-x86_64 -drive format=raw,file=os.bin
```
