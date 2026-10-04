# CLAUDE.md — Rust Playground Exercise Rules

This repo is a personal Rust learning playground. When asked to "create exercises for
<topic>", follow the rules below. The reference implementation of this style is
`rc_and_refcell/rc_and_refcell_main/src/main.rs` — read it before writing a new topic.

---

## 1. Repo layout

Cargo workspace (`Cargo.toml` at root, `members = ["*/*"]`), so every crate two levels deep
is picked up automatically — never edit the root `Cargo.toml` to add a topic.

```
<topic>/
  <topic>_main/      # ONE fully solved, richly commented reference file (deliverable #1)
  <topic>_00/        # incremental exercise crates (deliverable #2, ONLY when asked later)
  <topic>_01/
  ...
```

- Topic name: `snake_case`, e.g. `threads`, `rc_and_refcell`, `channels`, `traits_dyn`.
- New crate `Cargo.toml`:
  ```toml
  [package]
  name = "<topic>_main"
  version = "0.1.0"
  edition = "2024"

  [dependencies]
  ```
  Add dependencies only if the topic genuinely requires them (e.g. `tokio` for async).
- Run with: `cargo run -p <topic>_main`.

---

## 2. Deliverable for a new topic

Produce **exactly one file**: `<topic>/<topic>_main/src/main.rs` (+ its `Cargo.toml`).

- It is **fully solved** — no `todo!()`, no unimplemented parts.
- It **compiles and runs cleanly** in its default state (`cargo run`, `cargo clippy` with
  no warnings). Everything that breaks the build or panics is **commented out**.
- All learning material lives in comments: explanations, tips, broken variants,
  anti-patterns, debugger hints.
- Do NOT create the `_00.._NN` incremental crates in this step. That is a separate,
  explicit follow-up request (see §10).

---

## 3. Incremental knowledge — the core rule

Order steps so that each one introduces **exactly one new idea** and relies only on what
earlier steps taught. The usual arc:

1. Simplest working usage of the feature.
2. The first caveat / limitation you hit with it (BREAK-IT).
3. The tool or idiom that fixes the caveat (FIX-IT) — and that tool's own caveats.
4. A more powerful / ergonomic alternative.
5. Getting results back out / composing many instances.
6. Failure modes at runtime (panics, deadlocks, poisoning, leaks).
7. Anti-patterns that compile but are wrong or slow.
8. A final CHALLENGE combining everything.

Aim for **6–10 steps** per topic.

### Worked example: `threads`

| Step | Tag | Concept |
|------|-----|---------|
| 1 | [BUILD] | `thread::spawn` + `JoinHandle::join`; what happens if you don't join (main exits early) |
| 2 | [BREAK-IT] | closure borrows a local → `E0373: closure may outlive the current function` |
| 3 | [FIX-IT] | `move` closures; caveat: value is moved, can't use it afterwards (`E0382`); clone before moving |
| 4 | [BUILD] | `thread::scope` — borrow locals without `move`, auto-join at scope end |
| 5 | [BUILD] | returning a value from a thread: `JoinHandle<T>` → `join().unwrap()` |
| 6 | [BUILD] | spawn N workers, collect `Vec<JoinHandle<T>>`, join and collect into `Vec<T>` |
| 7 | [BREAK-IT] / [ANTI-PATTERN] | panic inside a thread → `join()` returns `Err`; joining inside the spawn loop serializes work |
| 8 | [CHALLENGE] | parallel sum / processing over the shared domain data |

The same arc applies to any topic (e.g. `Rc` → count → drop → no mutation → `RefCell` →
runtime panic → challenge, as in `rc_and_refcell_main`).

---

## 4. One shared domain example

- Define **one realistic domain type** at the top of the file (e.g. `ServerConfig`,
  `Order`, `SensorReading`) and **reuse it in every step**. The learner should focus on the
  new concept, never on new data shapes.
- Small helper constructors (e.g. `make_default_config()`) are fine and should be reused.
- When ownership, moves, or lifetimes matter, add `impl Drop` with a `println!` so the
  learner can *see* when values are destroyed (and in which thread).

---

## 5. File structure template

```rust
// =============================================================================
// Rust Learning Playground: <Topic Title>
//
// Topic:
//   <1–3 lines: what is learned and why it matters>
//
// How to use this file:
//   1. `cargo run -p <topic>_main` — it compiles and runs in its baseline state.
//   2. Read the steps in order (Step 1 -> Step N); each builds on the previous one.
//   3. Tags: [BUILD] [OBSERVE] [BREAK-IT] [FIX-IT] [ANTI-PATTERN] [DEBUG] [CHALLENGE]
//   4. Uncomment the marked blocks to see compiler errors / runtime panics,
//      read the explanation, then comment them back.
// =============================================================================

use std::...;

// -----------------------------------------------------------------------------
// Domain types
// -----------------------------------------------------------------------------
#[derive(Debug)]
struct ServerConfig { ... }

impl Drop for ServerConfig { ... }   // when drop timing is relevant

// =============================================================================
// STEP 1 [BUILD]: <Short title>
// =============================================================================
// Why this matters:
//   <the problem this step solves>
//
// TIP: <key API call 1 and what it does>
// TIP: <key API call 2 and what it does>
fn step_1_<name>() {
    println!("\n=== Step 1: <Short title> ===");
    // solved code with inline `// Expected: ...` comments
}

// ... more steps ...

// -----------------------------------------------------------------------------
// Driver: runs all steps in order
// -----------------------------------------------------------------------------
fn main() {
    println!(">>> Starting <Topic> Playground <<<");
    step_1_<name>();
    // ...
    println!("\n>>> Playground completed successfully! <<<");
}
```

Rules:
- One `fn step_N_<name>()` per step; each prints its own `=== Step N: ... ===` header.
- `main()` calls every step in order (pure-comment steps like a BREAK-IT on a free
  function may have no runtime function — then say so in a comment in `main`).
- Steps can share tags, e.g. `[BUILD & OBSERVE]`, `[BREAK-IT & FIX-IT]`.

---

## 6. BREAK-IT blocks (compile-time and runtime)

Every broken example is commented out so the file always compiles. Use this exact shape:

```rust
    // UNCOMMENT the lines below and run `cargo check`:
    //
    // let handle = thread::spawn(|| {
    //     println!("{}", config.host);
    // });
    //
    // What the compiler reports:
    //   error[E0373]: closure may outlive the current function, but it borrows `config`
    //
    // Why:
    //   <explain the underlying rule — ownership, lifetimes, Send/Sync, aliasing>
    //
    // Fix:
    //   <point to the solved code below / next step>
```

- For runtime failures use `// What you'll see at runtime:` with the real panic message
  (e.g. `already borrowed: BorrowMutError`, `called Result::unwrap() on an Err value`).
- For deadlocks / hangs, warn explicitly: `// WARNING: this will hang — Ctrl+C to stop`.
- **Error text must be real**: before finishing, temporarily uncomment each block, run
  `cargo check` / `cargo run`, copy the actual error code + first line, then re-comment.
- Prefer `/* ... */` for multi-line blocks that are meant to be uncommented together.

---

## 7. DEBUG blocks

Help the learner watch behaviour in a debugger (rust-analyzer / CodeLLDB / gdb):

```rust
    // [DEBUG] Set a breakpoint on the next line and inspect:
    //   - `handles.len()` — how many threads are alive
    //   - the thread list in the debugger — note each thread's name / id
    //   - step over `join()` and watch the Drop message appear
```

- Say **exactly what to observe** (ref counts, thread ids, value before/after move,
  drop order, lock state).
- Also print observable state (`Rc::strong_count`, `thread::current().id()`,
  `Arc::strong_count`, `{:p}` addresses) so the lesson works without a debugger.
- Optionally provide commented variants: `// UNCOMMENT to slow the thread down and see
  interleaving: thread::sleep(Duration::from_millis(100));`

---

## 8. ANTI-PATTERN blocks

For each step with a common misuse, add a commented block:

```rust
// [ANTI-PATTERN] Joining inside the spawn loop
//
// for i in 0..4 {
//     let h = thread::spawn(move || work(i));
//     h.join().unwrap();          // ❌ waits for each thread before spawning the next
// }
//
// Why it's bad:
//   Compiles and "works", but runs sequentially — zero parallelism.
// Idiomatic:
//   Collect handles first, join afterwards (see Step 6).
```

- Clearly label **"won't compile"** vs **"compiles but wrong"** — the latter category
  (silent perf bugs, deadlocks, leaks via `Rc` cycles, `.clone()` everywhere,
  `unwrap()` on poisoned locks, `Arc<Mutex<_>>` where a channel fits) is the most valuable.
- Always give the idiomatic alternative.

---

## 9. Commenting style

- Rich comments: explain **why** the compiler/runtime behaves this way, not just what the
  code does. Connect back to ownership/borrowing rules.
- Inline expected output: `println!("{}", Rc::strong_count(&a)); // Expected: 2`.
- `// TIP:` lines list the key APIs of the step with a one-line explanation each.
- ASCII banners exactly as in the template (`// ===` for steps, `// ---` for sections).
- Keep lines ≤ 100 chars. English only. Emojis only ❌ / ✅ to mark wrong/right lines.
- Use the std library only, unless the topic requires a crate.

---

## 10. Later step: splitting into incremental exercises (only when explicitly asked)

- Create `<topic>_00`, `<topic>_01`, ... from `<topic>_main`.
- Crate `_NN` contains steps up to the current one; the **current step's body** is replaced
  with `todo!("<what to implement>")` plus a `// Task:` comment; previous steps stay solved
  (or are trimmed if not needed); BREAK-IT / DEBUG / ANTI-PATTERN comments of the current
  step are kept.
- Each crate must still compile (`todo!()` is fine; it panics only when run).
- **Never overwrite** existing `_main` or `_NN` crates without asking — they may contain
  the user's own in-progress solutions (check `git status` first).

---

## 11. Checklist before finishing a topic

1. `cargo run -p <topic>_main` — runs to `Playground completed successfully!`.
2. `cargo clippy -p <topic>_main` — no warnings.
3. Every BREAK-IT block was uncommented once and its documented error/panic verified,
   then commented back.
4. Steps follow the incremental order (§3) and reuse the one domain type (§4).
5. Report to the user: list of steps (number, tag, concept) and the run command.
