# Amentys OS & The Maât Language

Amentys is not just a standard operating system; it is a bare-metal philosophy. Built without traditional binaries, it aims to eliminate chaos (Isfet) through strict hardware purity. It is written in **Maât**, a custom symbolic, typeless, and event-driven assembly language.

The compiler, **Thot**, acts as the scribe. It reads the Sacred Verbs (Maât source code) and translates them directly into pure x86_64 machine code, packaging them either as a Linux executable (Sarcophage) or a raw 512-byte bootloader (Naos).

## Key Features

* **Zero-Overhead Bare Metal:** Talk directly to the silicon without BIOS interruptions (in pure mode).
* **Dual-Output Compiler:** Generate standalone 64-bit ELF binaries or raw bootable disk images.
* **Sacred Registers:** Hardware registers are abstracted into metaphysical vessels (`%ka` for the Accumulator, `%ib` for the Counter, etc.).

## Prerequisites

To forge the OS and compile the Thot compiler, you will need:

* **Rust & Cargo** (to build the compiler)
* **QEMU** (to emulate the bare-metal bootloader)

## Getting Started

### 1. Build the Compiler (Thot)

Clone the repository and build the Rust project:

```bash
cargo install thot
```

### 2. Write your first Maât Law

Create a file named `os.maat`. Here is an example of an infinite echo terminal:

```text
wab             ; Purify the screen (Clear)
per "Terminal Amentys Pret."

boucle:
    sedjem %ka  ; Wait for a keystroke and store in %ka
    per %ka     ; Print the keystroke to the screen
    neheh boucle; Infinite loop back to 'boucle'
```

### 3. Invoke the Scribe (Compilation)

The Thot compiler uses positional arguments:

`thot <source_file> <output_file> <bootloader_mode> <keyboard_layout>`

**To compile as a Bare-Metal Bootloader (Naos):**

```bash
thot os.maat os.bin true qwerty
```

**To compile as a Linux ELF Executable (Sarcophage):**

```bash
thot os.maat os.elf false qwerty
```

*(Note: You can replace `qwerty` with `azerty` if needed).*

### 4. Run the Universe

To boot your newly created OS image in a virtual machine:

```bash
qemu-system-x86_64 -drive format=raw,file=os.bin
```

## The Sacred Verbs (Language Reference)

| Verb     | Equivalent | Description                                                  |
|----------|------------|--------------------------------------------------------------|
| `henek`  | `MOV`      | Assigns a value to a register (e.g., `henek %ka, 10`).       |
| `sema`   | `ADD`      | Adds a value to a register (e.g., `sema %ka, 5`).            |
| `wdj`    | `CMP`      | Weighs (compares) a register against a value.                |
| `ankh`   | `JE`       | Conditional jump to a label if the previous `wdj` was equal. |
| `neheh`  | `JMP`      | Unconditional jump to a label (Eternal loop).                |
| `per`    | `PRINT`    | Prints a string or the content of a register to the screen.  |
| `sedjem` | `LISTEN`   | Halts the CPU and waits for hardware input (keyboard).       |
| `wab`    | `CLEAR`    | Purifies the screen (clears all text).                       |
| `;`      | `//`       | A Whisper (Comment). Ignored by the compiler.                |
