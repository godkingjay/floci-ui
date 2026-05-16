# Rust Skills - Agent Instructions

> For OpenAI Codex and compatible agents

## Default Project Settings

When creating Rust projects or Cargo.toml files, ALWAYS use:

```toml
[package]
edition = "2024"
rust-version = "1.85"

[lints.rust]
unsafe_code = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
```

## Agent-Only Files

Put all plan, memory, scratch, and agent-workflow artifacts under `./local`.

Use `./local` for files that help agentic development but are not part of the
actual project deliverable, such as:

- implementation plans
- design notes and specs created for agent coordination
- temporary memory summaries or research notes
- task checklists, work logs, and agent handoff notes
- prompt drafts, tool outputs, and other scratch artifacts

Do not place agent-only artifacts in project docs, source, assets, tests, or
runtime configuration paths unless the user explicitly asks for a project-facing
artifact there. Project-facing documentation that is meant to ship with the UI
can live outside `./local`.

## Core Capabilities

### 1. Question Routing

Route Rust questions to appropriate skills:

- Ownership/borrowing → m01-ownership
- Smart pointers → m02-resource
- Error handling → m06-error-handling
- Concurrency → m07-concurrency
- Unsafe code → unsafe-checker

### 2. Code Style

Follow Rust coding guidelines:

- Use snake_case for variables and functions
- Use PascalCase for types and traits
- Use SCREAMING_SNAKE_CASE for constants
- Max line length: 100 characters
- Use `?` operator instead of `unwrap()` in library code

### 3. Error Handling

```rust
// Good: Use Result with context
fn read_config() -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string("config.toml")
        .map_err(|e| ConfigError::Io(e))?;
    toml::from_str(&content)
        .map_err(|e| ConfigError::Parse(e))
}

// Avoid: unwrap() in library code
fn read_config() -> Config {
    let content = std::fs::read_to_string("config.toml").unwrap(); // Bad
    toml::from_str(&content).unwrap() // Bad
}
```

### 4. Unsafe Code

Every `unsafe` block MUST have a `// SAFETY:` comment:

```rust
// SAFETY: We checked that index < len above, so this is in bounds
unsafe { slice.get_unchecked(index) }
```

### 5. Common Error Fixes

| Error | Cause                | Fix                                |
| ----- | -------------------- | ---------------------------------- |
| E0382 | Use of moved value   | Clone, borrow, or use reference    |
| E0597 | Lifetime too short   | Extend lifetime or restructure     |
| E0502 | Borrow conflict      | Split borrows or use RefCell       |
| E0499 | Multiple mut borrows | Restructure to single mut borrow   |
| E0277 | Missing trait impl   | Add trait bound or implement trait |

## Quick Reference

### Ownership

- Each value has one owner
- Borrowing: `&T` (shared) or `&mut T` (exclusive)
- Lifetimes: `'a` annotations for references

### Smart Pointers

- `Box<T>`: Heap allocation
- `Rc<T>`: Reference counting (single-threaded)
- `Arc<T>`: Atomic reference counting (thread-safe)
- `RefCell<T>`: Interior mutability

### Concurrency

- `Send`: Safe to transfer between threads
- `Sync`: Safe to share references between threads
- `Mutex<T>`: Mutual exclusion
- `RwLock<T>`: Reader-writer lock

### Async

```rust
#[tokio::main]
async fn main() {
    let handle = tokio::spawn(async {
        // async work
    });
    handle.await.unwrap();
}
```

## Skill Files

For detailed guidance, see these top-level skill docs in `.rust-skills`:

### Routing and Core Guidance

- `.rust-skills/rust-router/SKILL.md` - Route Rust questions to specific skills
- `.rust-skills/coding-guidelines/SKILL.md` - Rust code style and best practices
- `.rust-skills/unsafe-checker/SKILL.md` - Unsafe Rust and FFI review
- `.rust-skills/rust-learner/SKILL.md` - Rust versions, crate investigation, and learning support
- `.rust-skills/rust-skill-creator/SKILL.md` - Create Rust-focused skills for crates or APIs

### Rust Concept Modules

- `.rust-skills/m01-ownership/SKILL.md` - Ownership, borrowing, and lifetimes
- `.rust-skills/m02-resource/SKILL.md` - Smart pointers and resource management
- `.rust-skills/m03-mutability/SKILL.md` - Mutability and interior mutability
- `.rust-skills/m04-zero-cost/SKILL.md` - Generics, traits, and zero-cost abstractions
- `.rust-skills/m05-type-driven/SKILL.md` - Type-driven design
- `.rust-skills/m06-error-handling/SKILL.md` - Error handling patterns
- `.rust-skills/m07-concurrency/SKILL.md` - Concurrency, async, and thread safety
- `.rust-skills/m09-domain/SKILL.md` - Domain modeling
- `.rust-skills/m10-performance/SKILL.md` - Performance optimization
- `.rust-skills/m11-ecosystem/SKILL.md` - Crate ecosystem and dependency integration
- `.rust-skills/m12-lifecycle/SKILL.md` - Resource lifecycle design
- `.rust-skills/m13-domain-error/SKILL.md` - Domain error design
- `.rust-skills/m14-mental-model/SKILL.md` - Rust mental models and concept learning
- `.rust-skills/m15-anti-pattern/SKILL.md` - Rust anti-pattern review

### Domain Skills

- `.rust-skills/domain-cli/SKILL.md` - CLI tools
- `.rust-skills/domain-cloud-native/SKILL.md` - Cloud-native applications
- `.rust-skills/domain-embedded/SKILL.md` - Embedded and `no_std` Rust
- `.rust-skills/domain-fintech/SKILL.md` - Fintech applications
- `.rust-skills/domain-iot/SKILL.md` - IoT applications
- `.rust-skills/domain-ml/SKILL.md` - ML and AI applications in Rust
- `.rust-skills/domain-web/SKILL.md` - Web services

### Codebase Analysis and Refactoring

- `.rust-skills/rust-call-graph/SKILL.md` - Function call graph visualization
- `.rust-skills/rust-code-navigator/SKILL.md` - LSP-based code navigation
- `.rust-skills/rust-deps-visualizer/SKILL.md` - Dependency visualization
- `.rust-skills/rust-refactor-helper/SKILL.md` - Safe Rust refactoring support
- `.rust-skills/rust-symbol-analyzer/SKILL.md` - Rust project symbol analysis
- `.rust-skills/rust-trait-explorer/SKILL.md` - Trait implementation exploration

### Automation, Reports, and Internal Support

- `.rust-skills/rust-daily/SKILL.md` - Rust news and periodic reports
- `.rust-skills/meta-cognition-parallel/SKILL.md` - Experimental parallel meta-cognition workflow
- `.rust-skills/core-actionbook/SKILL.md` - Internal actionbook MCP support
- `.rust-skills/core-agent-browser/SKILL.md` - Internal agent-browser CLI support
- `.rust-skills/core-dynamic-skills/SKILL.md` - Internal dynamic Rust crate skill support
- `.rust-skills/core-fix-skill-docs/SKILL.md` - Internal skill documentation maintenance
