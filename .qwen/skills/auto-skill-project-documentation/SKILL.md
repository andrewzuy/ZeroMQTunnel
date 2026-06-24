---
name: project-documentation-generation
description: Generate comprehensive QWEN.md context file with architecture, conventions, and build instructions for future agent sessions
source: auto-skill
extracted_at: '2026-06-21T12:18:53.237Z'
---

## Skill Purpose

Generate a comprehensive `QWEN.md` file that serves as instructional context for future agent interactions with any code project. This documentation captures architecture, file structure, build instructions, development conventions, and current implementation state.

## When to Use

Trigger this skill when the user requests:
- "generate QWEN.md" or "create QWEN.md file"
- "context file for future agents" or "project context documentation"
- Any request documenting a codebase for team onboarding or handoff

## Pre-generation Checklist

Before creating QWEN.md, read and gather:
1. **README.md** — Project overview, user-facing docs, quick start info
2. **plan.md** — Development phases, milestones, target architecture
3. **Project root directory** — Main file structure, config files
4. **Key header files** (`include/*.h`) — Data structures, API declarations
5. **Key source files** (`src/*.c`) — Implementation details (or note if stubbed)
6. **Makefile/CMakeLists.txt** — Build system, dependencies, targets

## Generation Approach

### Step 1: Gather Context
```
Access these files in order:
- README.md → User-facing overview, architecture diagram reference
- plan.md → Phase milestones, completed/future work
- Project root listing → Complete directory structure
- include/headers/ → Data structures, enums, API declarations  
- src/implementation/*.c → Core logic (or note stub status)
- Makefile/CMakeLists.txt → Build commands, dependencies
```

### Step 2: Structure QWEN.md Content
Create these mandatory sections:

**Header:** Project title + "Quick Start Guide for New Agents"

**1. Quick Start Guide** — Top of file priority content:
- What has been implemented (completed vs stubbed)
- Build commands with full paths
- Run/daemonize instructions
- Command reference tables (if CLI exists)

**2. Architecture Overview** — High-level design:
- Core diagrams (ASCII art acceptable)
- Security model if applicable
- Hybrid encryption layers or data flow
- Transport protocol details

**3. File Structure Reference** — Directory tree + purpose summary:
```
root/
├── QWEN.md              # This context file
├── README.md             # User docs
├── plan.md               # Development phases
├── include/              # Public headers with mapping
│   ├── header1.h        # Purpose, key structs/enums
│   └── ...
├── src/                  # Implementation files
│   ├── file1.c          # Lines count + status (complete/stubbed)
│   └── ...
├── keys/                 # Config/key directories
└── tests/                # Test suite stubs if present
```

**4. Key Data Structures** — Copy from headers with inline comments:
- Transport/connection structs
- Crypto key contexts
- Payload envelopes
- Status flags and enums

**5. OpenSSL EVP Integration State** (if applicable):
- Current stub status
- Stub-to-production migration path table
- OpenSSL version notes and deprecation warnings

**6. Client Command Protocol** (if stdin/stdout interface exists):
- Input stream format
- Output stream format
- Example sessions with actual commands

**7. Server Relay Protocol** (if server-side forwarding):
- Message framing structure
- Whitelist loading logic
- Forwarding flow diagram

**8. Building & Testing Checklist** — Pre-flight + verification:
- Dependency installation commands
- Clean build verification
- Runtime test commands
- Memory safety check commands (valgrind)

**9. Development Conventions**:
- C code style rules (indentation, line length, naming)
- File modification pattern (5-step change process)
- Commit message format example

**10. Phase Status Reference**:
- Completed phases with ✅ marker
- Near-complete phases with 🟡 + notes
- Target milestones for future work
- Mapping to plan.md sections

**11. Known Limitations & Open Issues**:
- Current implementation gaps
- Security considerations (permissions, RNG, etc.)

**12. Quick Reference: Common Agent Tasks** — "Askable" Q&A pairs:
- "I need to understand how X is forwarded" → read file Y, lines Z
- "I need to migrate crypto stubs" → study phase 8 section
- etc.

**13. Appendices**:
- Error handling patterns
- Makefile target reference
- CMake placeholder if Phase 8+ targets exist

**Footer:** Last updated date, project author, license notes

### Step 3: Document Implementation States Clearly
For every component, state one of these status markers:

| Marker | Meaning                            | Example                                      |
|--------|------------------------------------|----------------------------------------------|
| ✅     | Complete / fully tested            | "P0 Project setup headers — ✅ Complete"    |
| 🟡     | Partial / stubbed                  | "P3 Auth handshake — 🟡 Partial (EVP stub)"  |
| ⚠️     | Needs work                         | "P5 Server relay — ⚠️ Table exists, logic partial" |
| ⬜     | Not yet implemented                | "P8 CI/CD audit — Deferred to Phase 8 milestone" |

**Critical:** Always distinguish between:
- Protocol design complete vs implementation complete
- Stub implementations vs production code
- TCP working vs ZeroMQ deferred (per project's architecture)

### Step 4: Document Build & Run Commands Precisely
Include full paths and environment setup:

```bash
# From project root (adjust path as needed)
cd /home/andrew/Development/ZeroMQTunnel/chat

# Required: openssl-dev build-essential libzmq3-dev
sudo apt-get install libzmq3-dev libssl-dev build-essential

# Build all targets
make all              # Creates: chat-server, chat-client, keygen
make keys             # RSA-2048 test keys in chat/keys/*.pem

# Run server (daemonize with &)
./chat-server 5558 &

# Test client via stdin
echo "/to bob Hello" | ./chat-client keys/priv.pem keys/pub.pem 127.0.0.1:5558
```

### Step 5: Create ASCII Architecture Diagrams
Use box-drawing characters or simple lines for clarity:

```
┌─────────────────────────────────────────────┐
│              ZERO-MQ RELAY CHAIN            │
│                                           │
│   Client A → Server → Client B             │
│                                           │
│   Message: [timestamp][RSA_enc(ES_AES)]    │
│         → [IV] [Ciphertext]               │
│         → [signature]                     │
└─────────────────────────────────────────────┘
```

**Keep diagrams under 20 lines wide for QWEN.md display limits.**

### Step 6: Create Protocol Reference Tables
Use markdown tables for commands, enums, and data structures:

| Command        | Description            | Example            |
|----------------|------------------------|--------------------|
| `/quit`        | Exit immediately       |                    |
| `/to <fp> MSG` | Send to fingerprint    | `/to alice Hello!` |

### Step 7: Add "Askable" Quick Reference Section
Create task→file mappings for common agent questions:

```markdown
## Quick Reference: Common Agent Tasks

**"I need to understand how messages are forwarded"**
→ Read `chat/src/server.c`, lines 50-150: `handle_msg_frame()` implementation  
→ See `include/transport.h` for multipart frame structures

**"How do I migrate crypto stubs to OpenSSL EVP"**
→ Study `plan.md` Phase 8 section for migration checklist  
→ Reference OpenSSL 3.x EVP documentation for modern API patterns
```

### Step 8: Verify Against Original Project Documentation
After drafting:
1. Cross-check all facts against README.md and plan.md
2. Confirm build commands actually work (test mentally)
3. Ensure phase status matches plan.md milestones
4. Validate file paths are correct from project root

## Post-generation Quality Checklist

- [ ] Quick start guide is first section after header
- [ ] All data structures documented with field descriptions
- [ ] Build commands include full paths and dependencies
- [ ] Architecture diagrams fit within ~80 line width
- [ ] Phase status markers (✅/🟡/⚠️) used consistently
- [ ] Known limitations section includes security considerations
- [ ] Quick reference has at least 3 common agent tasks mapped to files
- [ ] Appendices include error handling patterns and makefile targets

## Example Output Structure

```markdown
# Project Name - Context for Agents

## Quick Start Guide for New Agents

### Understanding What Has Been Implemented
[Implementation summary]

## Architecture Overview
[ASCII diagram + protocol description]

## File Structure Reference
[dir tree with purposes]

## Key Data Structures
[code blocks from headers with comments]

## Building & Testing Checklist
[detailed commands]

## Development Conventions
[code style, modification pattern, commit format]

## Phase Status Reference
[table with status markers]

## Known Limitations & Open Issues
[current gaps + security notes]

## Quick Reference: Common Agent Tasks
[task→file mappings]

## Appendices
[error patterns, makefile targets, CMake placeholder]
```

## Anti-patterns to Avoid

- **Don't** document code that's been deleted or renamed without noting the change
- **Don't** include debug-only code in production documentation
- **Don't** forget to distinguish between stubbed and complete implementations
- **Don't** assume agents will read plan.md — summarize phase status inline
- **Don't** omit OpenSSL version notes if API compatibility matters
- **Don't** create diagrams wider than 80 characters

## Common Variations

### CLI-less projects
Omit client command protocol section; instead document:
- Input/output streams (stdio, sockets, etc.)
- Signal handlers and graceful shutdown patterns
- Configuration file formats (INI, JSON, YAML)

### Microservice projects
Add inter-service communication diagrams:
```
Service A ─────→ API Gateway → Service B
     │              │                    │
   DB1            AuthZ             DB2
```

### Rust/C++/Go projects
Replace C-style data structure sections with:
- Struct definitions from language headers
- Trait/bound documentation where applicable
- Error type hierarchies (if documented)

---

**Key Takeaway:** The QWEN.md file should be self-contained. An agent reading only this file should understand project architecture, build/run procedures, and development patterns without needing to immediately consult other files.