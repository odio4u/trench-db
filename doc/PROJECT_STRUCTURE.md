# trench-db — Project Structure, Libraries & Build System

---

## Table of Contents

1. [Full Folder Tree](#1-full-folder-tree)
2. [What Each Folder Is For](#2-what-each-folder-is-for)
3. [What Each File Is For](#3-what-each-file-is-for)
4. [External Libraries — What They Are & Why We Use Them](#4-external-libraries)
5. [How C Libraries Are "Imported"](#5-how-c-libraries-are-imported)
6. [How Binaries Are Generated — The Full Compilation Pipeline](#6-how-binaries-are-generated)
7. [The Makefile Explained Line by Line](#7-the-makefile-explained)
8. [Build Commands Reference](#8-build-commands-reference)
9. [Dependency Diagram](#9-dependency-diagram)

---

## 1. Full Folder Tree

```
trench-db/
│
├── doc/                          ← written documentation
│   ├── PLAN.md                   ← phase-by-phase build plan
│   └── PROJECT_STRUCTURE.md      ← this file
│
├── include/                      ← public API headers (.h files)
│   ├── trench_types.h
│   ├── schema.h
│   ├── hasher.h
│   ├── crypto.h
│   ├── hashmap.h
│   ├── store.h
│   ├── wal.h
│   └── snapshot.h
│
├── src/                          ← implementation source files (.c files)
│   ├── schema.c
│   ├── hasher.c
│   ├── crypto.c
│   ├── hashmap.c
│   ├── store.c
│   ├── wal.c
│   └── snapshot.c
│
├── tests/                        ← one test binary per module
│   ├── test_schema.c
│   ├── test_hasher.c
│   ├── test_crypto.c
│   ├── test_store.c
│   ├── test_concurrent.c
│   └── test_wal.c
│
├── vendor/                       ← third-party source code, bundled in repo
│   ├── blake3/
│   │   ├── blake3.h
│   │   ├── blake3.c
│   │   ├── blake3_impl.h
│   │   ├── blake3_portable.c
│   │   └── blake3_dispatch.c
│   └── cjson/
│       ├── cJSON.h
│       └── cJSON.c
│
├── schemas/                      ← JSON files that describe data shapes
│   └── example_user.json
│
└── Makefile                      ← build instructions for GNU Make
```

---

## 2. What Each Folder Is For

### `doc/`
Plain documentation that explains the project to humans — not to the compiler.
Nothing in here affects the build. Put design notes, plans, and diagrams here.

---

### `include/`
**Header files only.** A header (`.h`) is a contract — it declares:
- What functions exist (their names, parameters, return types)
- What structs and enums look like
- What constants are available

It contains **no actual logic**. Other `.c` files `#include` headers to know
what they are allowed to call. Think of a header as the table of contents and
a `.c` file as the chapter with the actual content.

Rule: **one header per module**, matching its `.c` file.

---

### `src/`
**Implementation files.** Each `.c` file is one module. It `#include`s its own
header (to confirm the implementation matches the declaration) and any other
headers it needs, then provides the actual function bodies.

The compiler turns each `.c` file into an **object file** (`.o`) independently.
Object files are later linked together to form a binary.

---

### `tests/`
Each `.c` file here is a standalone program with its own `main()`. It calls
functions from `src/` and asserts that results are correct. Every test file
produces one **test binary** when compiled. The Makefile runs all of them.

Tests are separate executables — not part of the library — so a crashing test
cannot affect the others.

---

### `vendor/`
Third-party source code checked directly into our repository. We do not use a
package manager; instead we copy the exact source files we need. This means:

- The build works offline with no extra install steps
- The version is pinned forever — no surprise breakage from upstream changes
- The compiler sees vendor code exactly like our own code

Sub-folders group each library: `vendor/blake3/` and `vendor/cjson/`.

---

### `schemas/`
JSON text files that describe the shape of the data we want to store — column
names, types, whether a field is required, etc. These are **data files**, not
code. At runtime, `src/schema.c` reads them with cJSON and builds an in-memory
`SchemaRegistry`. Adding a new data shape means adding a new `.json` file here;
no recompilation needed.

---

## 3. What Each File Is For

### `include/trench_types.h`
The shared vocabulary of the entire project. Every other header includes this
one. Defines:
- `TrenchStatus` — the set of all possible return codes (`TRENCH_OK`,
  `TRENCH_ERR_CRYPTO`, etc.)
- `FieldType` — the 6 allowed column types (`FIELD_TEXT`, `FIELD_INT`, …)
- `Value` — a tagged union that holds one value of any `FieldType`
- `Record` — an array of `Value`s bound to a named schema

Because it is included everywhere, it must contain **no function definitions**
— only type declarations.

---

### `include/schema.h` + `src/schema.c`
Responsible for loading `.json` files from `schemas/` and validating records.

`schema.h` declares:
```
schema_load_dir()    — load every *.json file from a directory
schema_load_json()   — load one file
schema_find()        — look up a Schema by name
schema_validate()    — check a Record matches its schema
schema_registry_destroy()
```

`schema.c` implements all of the above. It uses `vendor/cjson/cJSON.h` to
parse the JSON text.

---

### `include/hasher.h` + `src/hasher.c`
Responsible for converting raw string keys into secure hash digests.

`hasher.h` declares:
```
StoreSecret   — typedef for uint8_t[32], the per-store secret
KeyHash       — typedef for uint8_t[32], the stored digest of a key
hash_key()    — BLAKE3-keyed hash: secret + raw_key → KeyHash
```

`hasher.c` calls `vendor/blake3/blake3.h` to do the actual hashing.
The raw key string never leaves this function — only the `KeyHash` is returned.

---

### `include/crypto.h` + `src/crypto.c`
Responsible for encrypting and decrypting record bytes in RAM.

`crypto.h` declares:
```
ValueKey        — typedef for uint8_t[32], a per-entry encryption key
EncryptedBlob   — struct { ciphertext, ciphertext_len, nonce[12], tag[16] }
crypto_encrypt()
crypto_decrypt()
blob_free_secure()
```

`crypto.c` uses OpenSSL's EVP API (`#include <openssl/evp.h>`) for
AES-256-GCM. It also uses `RAND_bytes` for the nonce and `OPENSSL_cleanse`
for secure memory wiping.

---

### `include/hashmap.h` + `src/hashmap.c`
The internal key-value table. Custom open-addressing hash map.

`hashmap.h` declares:
```
TrenchEntry   — one slot: KeyHash + EncryptedBlob + ValueKey + schema_name
HashMap       — the table: slots array + capacity + count
hashmap_create / insert / get / delete / contains / destroy
```

`hashmap.c` uses `OPENSSL_cleanse` (from `<openssl/crypto.h>`) to wipe slots
on delete. It does not depend on any other trench module — only on
`hasher.h` and `crypto.h` for the types it stores.

---

### `include/store.h` + `src/store.c`
The main public API. Ties every other module together.

`store.h` declares:
```
TrenchStore   — { secret, registry, table, pthread_rwlock_t, wal }
store_create / insert / get / delete / contains / destroy
record_free
```

`store.c` orchestrates: hash the key (hasher), validate (schema), serialize,
encrypt (crypto), insert (hashmap), append (wal). It uses `pthread_rwlock_t`
from `<pthread.h>` for thread safety and `RAND_bytes` to generate the store
secret on creation.

---

### `include/wal.h` + `src/wal.c`
Write-Ahead Log — crash recovery mechanism.

On every insert/delete, a frame is appended to a binary file on disk.
Each frame is AES-256-GCM encrypted with the store secret so the WAL file
is unreadable without that secret.

`wal_replay()` re-drives the log into a fresh store to restore state
after a restart or crash.

---

### `include/snapshot.h` + `src/snapshot.c`
Full store snapshot — planned-shutdown persistence.

`snapshot_write()` encrypts and serializes the entire in-memory table to
one file. `snapshot_load()` decrypts and rebuilds the table. Faster than
replaying a long WAL after a normal shutdown.

---

### `vendor/blake3/`
The official C reference implementation of the BLAKE3 hash function.
We use it in **keyed mode**: `blake3_hasher_init_keyed(secret)` — this binds
the hash to our secret so an outsider cannot predict or control hash outputs.

Files:
| File | Role |
|---|---|
| `blake3.h` | Public API header — we include this in `hasher.c` |
| `blake3.c` | Top-level logic — init, update, finalize |
| `blake3_impl.h` | Internal constants and structs (included by blake3.c) |
| `blake3_portable.c` | Pure C compression — works on any CPU |
| `blake3_dispatch.c` | CPU feature detection, picks fastest implementation |

---

### `vendor/cjson/`
A single-file JSON parser (cJSON). We use it only in `src/schema.c` to parse
the `schemas/*.json` files.

| File | Role |
|---|---|
| `cJSON.h` | Public API — we include this in `schema.c` |
| `cJSON.c` | Full implementation |

---

### `schemas/example_user.json`
An example schema file. Describes a "user" record with 5 typed fields.
At runtime `schema_load_dir("schemas/")` reads this file and registers the
schema. To add a new data shape, add a new `.json` file here.

---

### `Makefile`
Tells GNU Make how to compile and link everything. Explained in full in
[Section 7](#7-the-makefile-explained).

---

## 4. External Libraries

### OpenSSL (`libcrypto`)
**What it is:** The most widely deployed open-source cryptography library.
`libcrypto` is the sub-library inside OpenSSL that provides low-level
cryptographic primitives.

**Why we use it:**
- `EVP_aes_256_gcm()` — AES-256 in GCM mode (authenticated encryption)
- `RAND_bytes(buf, n)` — cryptographically secure random bytes (CSPRNG)
- `OPENSSL_cleanse(ptr, len)` — secure memory wipe that the compiler cannot
  optimize away (unlike `memset`)

**How it is installed:**
```sh
# Linux (Debian/Ubuntu)
sudo apt install libssl-dev

# Linux (Fedora/RHEL)
sudo dnf install openssl-devel

# macOS
brew install openssl

# Windows (MSYS2/MinGW)
pacman -S mingw-w64-x86_64-openssl
```

**How the compiler finds it:**
- Header search: `-I/usr/include/openssl` (usually automatic)
- Link flag: `-lssl -lcrypto` — tells the linker to link `libssl.so` and
  `libcrypto.so` (or their `.a` static equivalents)

---

### BLAKE3 (vendored)
**What it is:** A modern, extremely fast, cryptographically secure hash
function. Supports keyed mode — the hash output depends on both the input
AND a 32-byte secret key.

**Why we use it:** Standard `malloc`-based hash maps are vulnerable to
hash-flooding attacks (an attacker crafts keys that all collide). BLAKE3 keyed
mode makes this impossible — without the secret, the attacker cannot predict
which bucket any key lands in.

**How it is installed:** Not installed — the source files live in
`vendor/blake3/`. The Makefile compiles them as regular `.c` files alongside
our own code. No install step needed.

**How the compiler finds it:**
- We pass `-Ivendor/blake3` so `#include "blake3.h"` resolves to
  `vendor/blake3/blake3.h`

---

### cJSON (vendored)
**What it is:** A lightweight, single-file JSON parser written in C.

**Why we use it:** We need to read the `schemas/*.json` files at runtime.
cJSON handles all the parsing; we just traverse the resulting tree to build
our `Schema` structs.

**How it is installed:** Not installed — source lives in `vendor/cjson/`.
Compiled alongside our code. No install step needed.

**How the compiler finds it:**
- We pass `-Ivendor/cjson` so `#include "cJSON.h"` resolves to
  `vendor/cjson/cJSON.h`

---

### pthreads (`libpthread`)
**What it is:** POSIX Threads — the standard C threading and synchronization
API on Linux and macOS.

**Why we use it:** We use `pthread_rwlock_t` in `TrenchStore` — a
readers-writer lock that allows many concurrent readers but serializes writers,
which is exactly the right trade-off for a read-heavy key-value store.

**How it is installed:** Ships with the OS on Linux/macOS. On Windows use
MSYS2/MinGW which bundles a pthreads implementation.

**How the compiler finds it:**
- Header: `#include <pthread.h>` — this is a system header, always found
  automatically
- Link flag: `-lpthread`

---

## 5. How C Libraries Are "Imported"

Unlike Python (`import`) or JavaScript (`require`), C has no import system.
Instead there are two separate steps:

### Step 1 — Tell the compiler about the API: `#include`

```c
// In src/crypto.c
#include <openssl/evp.h>    // system library  — angle brackets <...>
#include "crypto.h"         // our own header  — quotes "..."
#include "../vendor/blake3/blake3.h"  // vendored — relative path
```

`#include` is a **preprocessor directive**. Before compiling, the preprocessor
literally copies the text of the named header file into the source file.
This is how the compiler learns the function signatures it is allowed to call.

`#include` **does not link any code**. It only provides declarations.

### Step 2 — Tell the linker where the actual code is: `-l` flags

When the compiler links all the object files into a binary, it must resolve
every function call to an actual address. For system libraries this is done
with linker flags:

```
-lssl     →  links libssl.so   (OpenSSL TLS layer)
-lcrypto  →  links libcrypto.so (OpenSSL crypto primitives)
-lpthread →  links libpthread.so (POSIX threads)
```

The `-l` prefix is shorthand: `-lcrypto` tells the linker to find a file
named `libcrypto.so` (Linux) or `libcrypto.dylib` (macOS) somewhere in the
library search path.

For **vendored libraries** (BLAKE3, cJSON) there is no `-l` flag — we just
compile their `.c` files directly and hand the resulting `.o` files to the
linker ourselves.

### Summary

| Library | `#include` | Linker flag |
|---|---|---|
| OpenSSL | `<openssl/evp.h>` etc. | `-lssl -lcrypto` |
| BLAKE3 | `"blake3.h"` (via `-Ivendor/blake3`) | none — compiled directly |
| cJSON | `"cJSON.h"` (via `-Ivendor/cjson`) | none — compiled directly |
| pthreads | `<pthread.h>` (system header) | `-lpthread` |

---

## 6. How Binaries Are Generated — The Full Compilation Pipeline

C compilation happens in four stages. Understanding these makes the Makefile
and any error messages much easier to read.

### Stage 1 — Preprocessing
```
gcc -E src/crypto.c → preprocessed source (all #includes expanded)
```
The preprocessor expands every `#include`, `#define`, and `#ifdef`. The output
is a single large `.c`-like text file. You rarely see this stage directly, but
it runs invisibly inside every `gcc` invocation.

### Stage 2 — Compilation (C source → assembly)
```
gcc -S src/crypto.c → src/crypto.s  (assembly text)
```
The C compiler parses the preprocessed source and generates CPU assembly
instructions. Again, this step is usually invisible.

### Stage 3 — Assembly (assembly → object file)
```
gcc -c src/crypto.c → src/crypto.o  (binary object file)
```
The assembler converts assembly into machine code and produces a `.o` (object)
file. A `.o` file contains compiled machine code but with **unresolved
symbols** — every function call that refers to code in another file is still
a placeholder.

This is the stage the Makefile uses explicitly:
```makefile
%.o: %.c
    gcc -std=c11 -Wall -Wextra -O2 -Iinclude -c $< -o $@
```

Each `.c` file compiles **independently** into its own `.o`. This is why
changes to one file do not require recompiling everything.

### Stage 4 — Linking (object files → executable binary)
```
gcc src/crypto.o src/hasher.o ... -lssl -lcrypto -lpthread -o tests/test_crypto
```
The linker takes all the `.o` files, resolves every unresolved symbol (function
call) between them, and resolves the remaining symbols against system libraries
(`-lssl`, etc.). The output is a self-contained executable binary.

### Visual summary

```
  src/crypto.c  ──┐
  src/hasher.c  ──┤   gcc -c       ┌── crypto.o ──┐
  src/store.c   ──┤  ──────────►   ├── hasher.o  ──┤
  vendor/       ──┤                ├── store.o   ──┤   gcc (link)
  blake3.c      ──┘                └── blake3.o ──┘  ──────────► test_store  (binary)
                                                     + -lssl -lcrypto -lpthread
```

---

## 7. The Makefile Explained

A `Makefile` is a set of rules that tell `make` how to build targets. Each
rule has the form:

```makefile
target: dependencies
    command to run
```

`make` only re-runs a command if a dependency is newer than the target — this
avoids recompiling unchanged files.

Here is the complete Makefile with every line explained:

```makefile
CC      = gcc                        # compiler to use
STD     = -std=c11                   # enforce C11 standard
WARN    = -Wall -Wextra -Wpedantic   # turn on all common warnings
OPT     = -O2                        # optimization level 2 (fast, debuggable)
CFLAGS  = $(STD) $(WARN) $(OPT) $(EXTRA_CFLAGS)
         # EXTRA_CFLAGS lets you inject -fsanitize=... from the command line
LDFLAGS = -lssl -lcrypto -lpthread   # libraries to link against

INC     = -Iinclude -Ivendor/blake3 -Ivendor/cjson
         # -Idir tells the compiler to search dir when resolving #include "..."
         # Without -Iinclude, #include "store.h" would fail to find include/store.h

# All .c files that make up the core library
LIB_SRCS = src/schema.c src/hasher.c src/crypto.c src/hashmap.c \
           src/store.c src/wal.c src/snapshot.c

# Vendored .c files compiled the same way as our own code
VENDOR_SRCS = vendor/blake3/blake3.c vendor/blake3/blake3_portable.c \
              vendor/blake3/blake3_dispatch.c vendor/cjson/cJSON.c

# Convert .c paths to .o paths  (src/store.c → src/store.o)
LIB_OBJS = $(LIB_SRCS:.c=.o) $(VENDOR_SRCS:.c=.o)

# The six test binaries we want to build
TESTS = tests/test_schema tests/test_hasher tests/test_crypto \
        tests/test_store tests/test_concurrent tests/test_wal

# .PHONY means these targets are not real files — always run them
.PHONY: all test clean

# Default target: build all test binaries
all: $(TESTS)

# Pattern rule: how to build any test binary
#   $^ = all dependencies (test .c + all .o files)
#   $@ = the target name  (e.g. tests/test_store)
tests/%: tests/%.c $(LIB_OBJS)
    $(CC) $(CFLAGS) $(INC) $^ -o $@ $(LDFLAGS)

# Pattern rule: how to compile any .c into a .o
#   $< = first dependency (the .c file)
#   $@ = the target (the .o file)
%.o: %.c
    $(CC) $(CFLAGS) $(INC) -c $< -o $@

# Run every test binary in sequence; stop on first failure
test: $(TESTS)
    @for t in $(TESTS); do \
        echo "Running $$t ..."; \
        ./$$t || exit 1; \
    done
    @echo "All tests passed."

# Remove all generated files
clean:
    rm -f $(LIB_OBJS) $(TESTS)
```

---

## 8. Build Commands Reference

### First-time setup — fetch vendored sources

```sh
# BLAKE3 (run from vendor/blake3/)
curl -LO https://raw.githubusercontent.com/BLAKE3-team/BLAKE3/master/c/blake3.h
curl -LO https://raw.githubusercontent.com/BLAKE3-team/BLAKE3/master/c/blake3.c
curl -LO https://raw.githubusercontent.com/BLAKE3-team/BLAKE3/master/c/blake3_impl.h
curl -LO https://raw.githubusercontent.com/BLAKE3-team/BLAKE3/master/c/blake3_portable.c
curl -LO https://raw.githubusercontent.com/BLAKE3-team/BLAKE3/master/c/blake3_dispatch.c

# cJSON (run from vendor/cjson/)
curl -LO https://raw.githubusercontent.com/DaveGamble/cJSON/master/cJSON.h
curl -LO https://raw.githubusercontent.com/DaveGamble/cJSON/master/cJSON.c
```

### Install system dependencies

```sh
# Ubuntu / Debian
sudo apt install build-essential libssl-dev

# Fedora / RHEL
sudo dnf install gcc openssl-devel

# macOS
xcode-select --install
brew install openssl
```

### Common make commands

| Command | What it does |
|---|---|
| `make` | Compile everything; build all test binaries |
| `make test` | Build + run all tests, stop on first failure |
| `make clean` | Delete all `.o` files and test binaries |
| `make test EXTRA_CFLAGS="-fsanitize=address,undefined"` | Build + test with AddressSanitizer and UndefinedBehaviorSanitizer |
| `make test EXTRA_CFLAGS="-fsanitize=thread"` | Build + test with ThreadSanitizer (detects data races) |
| `make test EXTRA_CFLAGS="-g -O0"` | Debug build (symbols on, no optimization) |

### Run a single test

```sh
./tests/test_store
./tests/test_crypto
```

### Leak check with Valgrind (Linux only)

```sh
make test EXTRA_CFLAGS="-g -O0"
valgrind --leak-check=full --error-exitcode=1 ./tests/test_store
```

---

## 9. Dependency Diagram

This shows which files include which headers. An arrow means "depends on /
includes".

```
trench_types.h   (no dependencies — root node)
      │
      ├──► schema.h    ──► cJSON.h  (vendor)
      │
      ├──► hasher.h    ──► blake3.h (vendor)
      │
      ├──► crypto.h    ──► openssl/evp.h   (system)
      │                ──► openssl/rand.h  (system)
      │
      ├──► hashmap.h   ──► hasher.h
      │                ──► crypto.h
      │
      ├──► wal.h       ──► hashmap.h
      │
      ├──► snapshot.h  ──► hasher.h
      │
      └──► store.h     ──► schema.h
                       ──► hasher.h
                       ──► hashmap.h
                       ──► wal.h
                       ──► pthread.h  (system)
                       ──► openssl/rand.h (system)
```

Each `src/*.c` file includes its own matching `.h` plus any `.h` it calls
into. The `tests/*.c` files include only the header of the module they test.
