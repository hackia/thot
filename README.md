# Thot

An assembler with mythological semantics allowing you to compile boot loaders (Naos) and ELF (Sarcophagus) executables.

> Summon the sacred registers, manipulate the size of the Propeller and bring silicon to life.

## Verbs

| Verb      | x86 OpCode | Action in Maât                                                                                                                |
|-----------|------------|-------------------------------------------------------------------------------------------------------------------------------|
| ankh      | JE         | Life: Conditional jump to a label if Libra is in balance (Tie).                                                               |
| dema      | (Merge)    | Weave: Includes/merges another Maât tablet (file) into the current code.                                                      |
| dja       | CALL FAR   | Project: Performs a Far Call to a specific segment and label target.                                                          |
| duat      | MOV (Mem)  | Burn: Writes a string in RAM with the automatic Sign of Silence (null term.).                                                 |
| henek     | MOV        | Give: Now capable of transmitting immediate (constant) numbers directly into 32-bit and 128-bit registers.                    |
| henet     | AND        | Assemble: Logical operation AND (Bitwise AND).                                                                                |
| her       | JG         | Peak: Conditional jump if the ship is strictly greater than the value.                                                        |
| her_ankh  | JGE        | Peak Life: Conditional jump if the ship is greater than or equal to the value.                                                |
| in        | IN         | Receive: Reads a hardware port (using %da) into AL.                                                                           |
| isfet     | JNE        | Chaos: Conditional jump to a label if Libra is broken (Difference).                                                           |
| jena      | CALL       | Summon: Calls a ritual (function) and prepares for the return of the soul.                                                    |
| kheb      | SUB        | Reduce: Subtracts a value from the force contained in a ship.                                                                 |
| kheper    | MOV [mem]  | Embody: Writes the contents of a register into the RAM.                                                                       |
| kher      | JL         | Depth: Conditional jump if the ship is strictly less than the value.                                                          |
| kher_ankh | JLE        | Depth Life: Conditional jump if the ship is less than or equal to the value.                                                  |
| kherp     | INT 13h    | waken: Its power has been increased tenfold to load 64 sectors (32 KB) from disk to RAM.                                      |
| mer       | OR         | Link: Logical operation OR (Bitwise OR).                                                                                      |
| nama      | ALLOC      | Create: Now able to allocate pure numbers (in addition to helices and phrases) in sacred memory (the Noun).                   |
| neheh     | JMP        | Eternity: Unconditional jump (infinite loop) to a target label.                                                               |
| out       | OUT        | Emit: Writes a value to a hardware port (using %da).                                                                          |
| per       | INT/VGA    | st: Now hybrid. It uses 16-bit BIOS and 32-bit direct VGA memory writing to display text.                                     |
| pop       | POP        | Exhume: Retrieves a value from the sacred Stack into a register.                                                              |
| push      | PUSH       | Bury: Pushes a value or register onto the sacred Stack.                                                                       |
| rdtsc     | RDTSC      | Time: Reads the processor's Time Stamp Counter to measure the cycles of Ra.                                                   |
| return    | RET        | Return: Leaves a ritual to resume the thread of the previous existence.                                                       |
| sedjem    | INT 16h    | isten: Improved for Protected Mode. It now listens directly to the hardware (Port 0x60) without depending on the 32-bit BIOS. |
| sema      | ADD        | Unite: Adds a value to the force contained in a ship.                                                                         |
| sena      | MOV reg    | Collect: Reads data from the RAM into a register.                                                                             |
| shesa     | IMUL       | Multiply: Multiplies the force contained in a ship.                                                                           |
| smen      | (None)     | *Currently unimplemented in the Emitter.*                                                                                     |
| sokh      | DEC        | Strike: The new verb that reduces the strength of a register by 1 (Decrement). Ideal for time loops.                          |
| wab       | INT 10h    | Purify: Clears the screen and resets the sacred void (Clear Screen).                                                          |
| wdj       | CMP        | Weigh: Compares (weighs) a ship against a value on the Balance of Maat.                                                       |

## Register

| register | description                                         |
|----------|-----------------------------------------------------|
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
|-----------|------------|----------------------|----------------------|---------------------------------|-----------------------------------|
| Base      | (none)     | 8                    | 4                    | 255                             | -128 to 127                       |
| Medium    | m          | 16                   | 8                    | 65,535                          | -32,768 to 32,767                 |
| High      | h          | 32                   | 16                   | 4,294,967,295                   | -2.14 × 10⁹ to 2.14 × 10⁹         |
| Very      | v          | 64                   | 32                   | ≈ 1.844674407 × 10¹⁹            | ± 9.22 × 10¹⁸                     |
| Extreme   | e          | 128                  | 64                   | ≈ 3.402823669 × 10³⁸            | ± 1.70 × 10³⁸                     |
| Zenith    | x          | 256                  | 128                  | ≈ 1.157920892 × 10⁷⁷            | ± 5.78 × 10⁷⁶                     |

Examples: `%ba` (Base), `%mba` (Medium), `%hba` (High), `%vba` (Very), `%eba` (Extreme), `%xba` (Zenith). Same scheme
for `%ka`, `%ib`, `%da`, `%si`, `%di`.

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
