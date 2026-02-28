

# Thot

An assembler with mythological semantics allowing you to compile boot loaders (Naos) and ELF (Sarcophagus) executables. 

> Summon the sacred registers, manipulate the size of the Propeller and bring silicon to life.

## Verbs

| Verb      | desc     | action                                                       |
| --------- | -------- | ------------------------------------------------------------ |
| ankh      | JE       | Life: Conditional jump to a label if Libra is in balance (Tie). |
| duat      | DB / MOV | Burn: Writes a sentence in RAM with the automatic Sign of Silence (null terminator). |
| dja       | MUL      | Multiply: Multiplies the force contained in the ship. *(Suggestion)* |
| dema      | DIV      | Divide: Divides the force contained in the ship. *(Suggestion)* |
| her       | JG       | Avoid: Conditional jump if the ship is strictly greater than the value. |
| henek     | MOV      | Give: Assigns an immediate value or register to a ship (register). |
| in        | IN       | Receive: Reads a value from a hardware port into %ka.        |
| kherp     | XOR      | Shift/Control: Logical operation XOR (Bitwise XOR). *(Suggestion)* |
| her_ankh  | JGE      | Peak: Conditional jump if the ship is greater than or equal to the value. |
| henet     | AND      | Assemble: Logical operation AND (Bitwise AND).               |
| isfet     | JNE      | Chaos: Conditional jump to a label if Libra is broken (Difference). |
| jena      | CALL     | Summon: Calls a ritual (function) and prepares for the return of the code soul. |
| kheb      | SUB      | Reduce: Subtracts a value from the force contained in a ship. |
| kher      | JL       | Depth: Conditional jump if the ship is strictly less than the value. |
| kheper    | MOV      | Embody: Saves the contents of %ka to a RAM address or a [%ba] pointer. |
| kher_ankh | JLE      | Base: Conditional jump if the ship is less than or equal to the value. |
| mer       | OR       | Link: Logical operation OR (Bitwise OR).                     |
| nama      | INC      | Step: Increments the value of a ship by one. *(Suggestion)*  |
| neheh     | JMP      | Eternity: Unconditional jump (infinite loop) to a label.     |
| out       | OUT      | Emit: Writes a value from %ka to a hardware port.            |
| push      | PUSH     | Bury: Pushes a value onto the sacred Stack.                  |
| pop       | POP      | Exhume: Retrieves a value from the sacred Stack.             |
| shesa     | NOT      | Invert: Bitwise NOT operation (Inverts all bits). *(Suggestion)* |
| sena      | MOV      | Collect: Loads data into %ka from a RAM address or a [%ba] pointer. |
| per       | INT 10h  | Manifest: Displays a character, register or phrase on the screen. |
| return    | RET      | Return: Leaves a ritual to resume the thread of the previous existence. |
| rdtsc     | RDTSC    | Time: Reads the processor's Time Stamp Counter to measure the cycles of Ra. |
| sedjem    | INT 16h  | Listen: Interrupts time and waits for a keyboard pulse (stored in %ka). |
| sema      | ADD      | Unite: Adds a value to the force contained in a ship.        |
| sokh      | DEC      | Strike: Decrements the value of a ship by one. *(Suggestion)* |
| smen      | LEA      | Establish: Loads the effective memory address into a ship. *(Suggestion)* |
| wdj       | CMP      | Weigh: Compares (weighs) a ship against a value on the Balance of Maat. |
| wab       | INT 10h  | Purify: Clears the screen and resets the sacred void (Clear Screen). |

## Register

| register | description                                         |
| -------- | --------------------------------------------------- |
| %ka      | Accumulator (EAX) – The seat of mathematical power. |
| %ib      | Counter (ECX) – The master of time and cycles.      |
| %da      | Data (EDX) – The servant of inputs/outputs.         |
| %ba      | Base (EBX) – The finger that points to memory.      |
| %si      | Source (ESI) – The origin of data flows.            |
| %di      | Destination (EDI) – The arrival of data flows.      |

### **Register Levels**

The register name encodes the Helix size. Sizes are total Helix width (Ra + Apophis).

Each channel is half.

| **level** | **prefix** | **total Helix size** | **per-channel size** | **Max Unsigned Value (0 to N)** | **Signed Range (Complément à 2)** |
| --------- | ---------- | -------------------- | -------------------- | ------------------------------- | --------------------------------- |
| Base      | (none)     | 8                    | 4                    | 255                             | -128 to 127                       |
| Medium    | m          | 16                   | 8                    | 65,535                          | -32,768 to 32,767                 |
| High      | h          | 32                   | 16                   | 4,294,967,295                   | -2.14 × 10⁹ to 2.14 × 10⁹         |
| Very      | v          | 64                   | 32                   | ≈ 1.844674407 × 10¹⁹            | ± 9.22 × 10¹⁸                     |
| Extreme   | e          | 128                  | 64                   | ≈ 3.402823669 × 10³⁸            | ± 1.70 × 10³⁸                     |
| Zenith    | x          | 256                  | 128                  | ≈ 1.157920892 × 10⁷⁷            | ± 5.78 × 10⁷⁶                     |

Examples: `%ba` (Base), `%mba` (Medium), `%hba` (High), `%vba` (Very), `%eba` (Extreme), `%xba` (Zenith). Same scheme for `%ka`, `%ib`, `%da`, `%si`, `%di`.

Rules: operations between different sizes are rejected, and overflow is a compile-time error.

## Installation

```bash
cargo install thot
```

### Compilation

The Thot compiler uses positional arguments:

`thot <main_maat_file> <output_file> <bootloader_mode> <keyboard_layout>`

**To compile as a Bare-Metal Boot loader (Naos):**

```bash
thot os.maat os.bin true qwerty
```

**To compile as an Amentys ELF Executable (Sarcophagus):**

```bash
thot os.maat os.elf false qwerty
```

### Run the Universe

To boot your newly created OS image in a virtual machine:

```bash
qemu-system-x86_64 -drive format=raw,file=os.bin
```
