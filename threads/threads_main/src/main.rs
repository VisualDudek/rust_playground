// =============================================================================
// Rust Learning Playground: OS Threads with std::thread
//
// Topic:
//   Running work in parallel with `std::thread`: spawning and joining, why closures
//   need `move`, borrowing safely with `thread::scope`, getting results back out,
//   and what happens when a thread panics.
//
// How to use this file:
//   1. `cargo run -p threads_main` — it compiles and runs in its baseline state.
//   2. Read the steps in order (Step 1 -> Step 8); each builds on the previous one.
//   3. Tags: [BUILD] [OBSERVE] [BREAK-IT] [FIX-IT] [ANTI-PATTERN] [DEBUG] [CHALLENGE]
//   4. Uncomment the marked blocks to see compiler errors / runtime panics,
//      read the explanation, then comment them back.
// =============================================================================

use std::thread;
use std::time::{Duration, Instant};

// -----------------------------------------------------------------------------
// Domain types
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
struct ServerConfig {
    host: String,
    port: u16,
    max_connections: usize,
}

// Custom Drop so you can SEE when a config is destroyed — and, more importantly
// for this topic, IN WHICH THREAD. A value moved into a thread is dropped there.
impl Drop for ServerConfig {
    fn drop(&mut self) {
        let current = thread::current();
        println!(
            "  [DROP] ServerConfig '{}:{}' dropped in thread '{}' ({:?})",
            self.host,
            self.port,
            current.name().unwrap_or("<unnamed>"),
            current.id()
        );
    }
}

// -----------------------------------------------------------------------------
// Helpers reused by every step
// -----------------------------------------------------------------------------
fn make_config(i: u16) -> ServerConfig {
    ServerConfig {
        host: format!("10.0.0.{i}"),
        port: 8000 + i,
        max_connections: 100 * (i as usize + 1),
    }
}

fn make_fleet(n: u16) -> Vec<ServerConfig> {
    (0..n).map(make_config).collect()
}

// Simulated network health check: takes ~20 ms and returns a fake latency in ms.
// Deterministic, so parallel and sequential results can be compared.
const CHECK_DURATION: Duration = Duration::from_millis(20);

fn health_check(cfg: &ServerConfig) -> u64 {
    thread::sleep(CHECK_DURATION);
    (cfg.port % 100) as u64 + cfg.host.len() as u64
}

// =============================================================================
// STEP 1 [BUILD & OBSERVE]: Spawning a Thread and Joining It
// =============================================================================
// Why this matters:
//   `main` runs on the main thread. To do work in parallel you start another OS
//   thread. The new thread runs independently — the main thread does NOT wait for
//   it unless you explicitly ask it to.
//
// TIP: `thread::spawn(closure)` starts a new OS thread running `closure`, returns
//      a `JoinHandle<T>` immediately (T = the closure's return type).
// TIP: `handle.join()` blocks the current thread until that thread finishes.
// TIP: `thread::current().id()` / `.name()` identify the thread you are on.
fn step_1_spawn_and_join() {
    println!("\n=== Step 1: Spawning a Thread and Joining It ===");
    println!("main thread id: {:?}", thread::current().id()); // Expected: ThreadId(1)

    let handle = thread::spawn(|| {
        // [DEBUG] Set a breakpoint on the next line and inspect:
        //   - the debugger's thread list: there are now (at least) 2 threads
        //   - the call stack of this thread: it does NOT contain `main`, it starts
        //     in std's thread-start machinery — a separate stack entirely
        let cfg = make_config(1);
        // Expected: ThreadId(2)
        println!("  worker {:?} checks {}", thread::current().id(), cfg.host);
        // `cfg` is created AND dropped in this worker thread — watch the [DROP] line.
    });

    println!("main keeps going while the worker runs (lines may interleave)");
    handle.join().unwrap(); // wait for the worker to finish
    println!("worker joined — from here on its output is guaranteed to be printed");

    // [OBSERVE] Detached thread: what if you never call `join()`?
    //
    // UNCOMMENT the lines below and run `cargo run -p threads_main`:
    /*
    thread::spawn(|| {
        thread::sleep(Duration::from_secs(5));
        println!("  !!! detached worker finished"); // ❌ never printed
    });
    */
    //
    // What you'll see at runtime:
    //   Nothing from the detached thread. The whole playground finishes in < 1 s,
    //   and when `main` returns the process exits, killing every other thread
    //   mid-sleep. Dropping a `JoinHandle` does NOT stop or wait for the thread —
    //   it just "detaches" it. If you need its work done, `join()` it.
}

// =============================================================================
// STEP 2 [BREAK-IT]: A Closure That Borrows a Local Variable
// =============================================================================
// Why this matters:
//   The natural first attempt: create a config on main's stack and read it from a
//   thread. The compiler refuses.
fn step_2_borrow_fails() {
    println!("\n=== Step 2: Borrowing a Local From a Thread (compile error) ===");
    let config = make_config(2);

    // UNCOMMENT the lines below and run `cargo check`:
    //
    // let handle = thread::spawn(|| {
    //     println!("{}", config.host);
    // });
    // handle.join().unwrap();
    //
    // What the compiler reports:
    //   error[E0713]: borrow may still be in use when destructor runs
    //   ... argument requires that `config.host` is borrowed for `'static`
    //   ... drop of `config` needs exclusive access to `config.host`, because the
    //       type `ServerConfig` implements the `Drop` trait
    //
    //   (Without our `impl Drop`, the classic error is:
    //    error[E0373]: closure may outlive the current function, but it borrows
    //    `config.host`, which is owned by the current function. Same root cause.)
    //
    // Why:
    //   `thread::spawn` requires `F: FnOnce() -> T + Send + 'static`. `'static`
    //   means the closure may not hold references to anything that can die before
    //   the program ends. The compiler cannot prove the thread finishes before
    //   `config` is dropped at the end of this function — even though we `join()`
    //   right away, `join()` is just a function call; the type system does not
    //   know it ends the borrow. If you forgot `join()`, the thread could read
    //   freed memory. Our `Drop` impl makes it explicit: dropping `config` needs
    //   exclusive access, but the thread may still hold a `&` to it.
    //   (Edition 2021+ closures capture disjoint fields, hence `config.host`.)
    //
    // Fix:
    //   Step 3 — give the thread OWNERSHIP of the data with `move`.
    //   Step 4 — or use `thread::scope`, which DOES prove the thread ends in time.
    println!("config still owned by main: {}", config.host);
}

// =============================================================================
// STEP 3 [FIX-IT]: `move` Closures — Transferring Ownership to the Thread
// =============================================================================
// Why this matters:
//   If the thread owns its data, there is nothing that can dangle — the closure
//   becomes `'static`. The price: the data is gone from the spawning thread.
//
// TIP: `move || { ... }` captures every used variable BY VALUE (moves it in).
// TIP: `.clone()` BEFORE the move if the spawning thread still needs a copy.
fn step_3_move_closure() {
    println!("\n=== Step 3: `move` Closures ===");
    let config = make_config(3);

    let handle = thread::spawn(move || {
        println!("  worker owns {}:{}", config.host, config.port);
        // `config` is dropped HERE, at the end of the closure, in the worker thread.
    });
    handle.join().unwrap();
    // Expected: the [DROP] line names an <unnamed> worker thread, not 'main'.

    // [BREAK-IT] Using the value after it was moved.
    //
    // UNCOMMENT the lines below and run `cargo check`:
    //
    // let config = make_config(3);
    // let handle = thread::spawn(move || println!("{}", config.host));
    // println!("{}", config.host); // ❌ `config.host` lives in the other thread now
    // handle.join().unwrap();
    //
    // What the compiler reports:
    //   error[E0382]: borrow of moved value: `config`
    //
    // Why:
    //   (It says `config`, not `config.host`: a type with `impl Drop` can't be
    //   split into fields, so `move` takes the WHOLE struct.)
    //   Ownership has a single owner. After the move, main has nothing left to
    //   read — and allowing it would mean two threads touching the same String
    //   without any synchronization.
    //
    // Fix: clone before moving — each thread gets its own independent copy.
    let config = make_config(3);
    let for_worker = config.clone();
    let handle = thread::spawn(move || {
        println!("  worker uses its clone: {}", for_worker.host);
    });
    handle.join().unwrap();
    println!("main still has the original: {}", config.host);
    // Expected: two [DROP] lines for 10.0.0.3 — one in the worker, one in 'main'.

    // [ANTI-PATTERN] (compiles but wasteful) `.clone()` everything "just in case"
    //   Cloning a big config per thread copies all its heap data. Fine for small
    //   data; for large shared read-only data use `Arc<T>` (shared ownership
    //   across threads — the thread-safe `Rc`, its own topic) or `thread::scope`
    //   (next step) to borrow without copying.
}

// =============================================================================
// STEP 4 [BUILD]: `thread::scope` — Borrowing Locals Safely
// =============================================================================
// Why this matters:
//   Step 2 failed because the compiler could not prove the thread ends before
//   the data dies. `thread::scope` makes that a guarantee: every thread spawned
//   in the scope is joined automatically before `scope` returns. So threads may
//   BORROW local data — no `move`, no clones, no `'static`.
//
// TIP: `thread::scope(|s| { s.spawn(|| ...); })` — `s` is the scope handle.
// TIP: `s.spawn` returns a `ScopedJoinHandle`; joining it is optional — the scope
//      joins everything left over at its end.
// TIP: normal borrow rules apply ACROSS threads: many `&T` OR one `&mut T`.
fn step_4_scoped_threads() {
    println!("\n=== Step 4: Scoped Threads ===");
    let config = make_config(4);
    let mut checks_done = 0;

    thread::scope(|s| {
        // Two threads share `&config` — many shared borrows are fine.
        s.spawn(|| println!("  reader A sees host {}", config.host));
        s.spawn(|| println!("  reader B sees port {}", config.port));
        // One thread takes `&mut checks_done` — exclusive, nobody else touches it.
        s.spawn(|| checks_done += 1);
    }); // <- all three threads are joined HERE, so the borrows end here

    println!("after the scope: checks_done = {checks_done}"); // Expected: 1
    println!("config is still owned by main: {}", config.host);
    // Expected: no [DROP] inside the scope — `config` dies at the end of this
    // function, in thread 'main'.

    // [BREAK-IT] Two threads mutating the same variable.
    //
    // UNCOMMENT the lines below and run `cargo check`:
    /*
    let mut checks = 0;
    thread::scope(|s| {
        s.spawn(|| checks += 1);
        s.spawn(|| checks += 1); // ❌ second `&mut checks` while the first is alive
    });
    */
    //
    // What the compiler reports:
    //   error[E0499]: cannot borrow `checks` as mutable more than once at a time
    //
    // Why:
    //   This is exactly a data race: two threads doing read-modify-write on the
    //   same memory. The borrow checker's "one `&mut` at a time" rule is what
    //   makes Rust threads data-race-free at compile time.
    //
    // Fix:
    //   Let each thread produce its own value and combine afterwards (Steps 5–6),
    //   or use a synchronization primitive (`AtomicUsize`, `Mutex` — later topics).
}

// =============================================================================
// STEP 5 [BUILD]: Returning a Value From a Thread
// =============================================================================
// Why this matters:
//   Threads usually compute something. Instead of writing into shared state, the
//   cleanest way is to RETURN the result — it travels back through the handle.
//
// TIP: `thread::spawn(|| -> T { ... })` gives a `JoinHandle<T>`.
// TIP: `handle.join()` returns `thread::Result<T>` = `Result<T, Box<dyn Any + Send>>`
//      — `Err` if the thread panicked (Step 7). `.unwrap()` it for now.
// TIP: `thread::Builder::new().name(..).spawn(..)` names the thread (shows up in
//      panics, debuggers, `htop`). It returns `io::Result<JoinHandle<T>>` because
//      the OS can refuse to create a thread.
fn step_5_return_values() {
    println!("\n=== Step 5: Returning Values From Threads ===");
    let config = make_config(5);

    let handle = thread::spawn(move || health_check(&config));
    let latency: u64 = handle.join().unwrap();
    println!("latency of 10.0.0.5: {latency} ms"); // Expected: 13

    let named = thread::Builder::new()
        .name("health-worker-5".to_string())
        .spawn(|| {
            let cfg = make_config(5);
            let name = thread::current().name().map(String::from);
            (name, health_check(&cfg))
        })
        .expect("OS failed to spawn a thread");

    // Expected: Some("health-worker-5")
    println!("handle refers to thread: {:?}", named.thread().name());
    let (name, latency) = named.join().unwrap();
    // Expected: Some("health-worker-5") reported 13 ms
    println!("thread {name:?} reported {latency} ms");
    // Expected: the [DROP] line for this config names 'health-worker-5'.

    // [DEBUG] Set a breakpoint on `named.join()` and look at the thread list:
    //   the worker shows up as "health-worker-5" instead of an anonymous id.
}

// =============================================================================
// STEP 6 [BUILD]: Fan-Out / Fan-In — N Workers, Collect All Results
// =============================================================================
// Why this matters:
//   Real parallelism: start one thread per task FIRST, then join them all. Total
//   time ≈ the slowest task, not the sum of all tasks.
//
// TIP: collect handles into `Vec<JoinHandle<T>>` with `.map(...).collect()`.
// TIP: `.into_iter().map(|h| h.join().unwrap()).collect::<Vec<T>>()` — results
//      come back in spawn order, regardless of which thread finished first.
// TIP: `Instant::now()` / `.elapsed()` to measure the speedup.
fn step_6_many_workers() {
    println!("\n=== Step 6: Many Workers, Collect Results ===");
    let fleet = make_fleet(4);
    let start = Instant::now();

    let handles: Vec<thread::JoinHandle<(String, u64)>> = fleet
        .into_iter() // each config is MOVED into its own thread
        .map(|cfg| {
            thread::spawn(move || {
                let latency = health_check(&cfg);
                (cfg.host.clone(), latency)
            })
        })
        .collect(); // ← all 4 threads are running before we join any of them

    let results: Vec<(String, u64)> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // [DEBUG] Set a breakpoint on the next line and inspect:
    //   - `results` — same order as `fleet`, although threads finished in any order
    //   - the [DROP] lines above: each config died in a different worker thread id
    println!("results: {results:?}");
    // Expected: [("10.0.0.0", 8), ("10.0.0.1", 9), ("10.0.0.2", 10), ("10.0.0.3", 11)]
    println!(
        "4 checks took {:?} (one check = {:?})",
        start.elapsed(),
        CHECK_DURATION
    ); // Expected: ~20 ms, not ~80 ms

    // UNCOMMENT to slow down one worker and see out-of-order completion:
    //   inside the closure add: `if cfg.port == 8000 { thread::sleep(CHECK_DURATION * 5); }`
    //   The [DROP] lines reorder, but `results` stays in spawn order.
}

// =============================================================================
// STEP 7 [BREAK-IT & ANTI-PATTERN]: Panics and Accidental Serialization
// =============================================================================
// Why this matters:
//   A panic in a spawned thread does NOT crash the process — it unwinds only that
//   thread, and the panic is delivered to whoever calls `join()` as an `Err`.
//   Calling `.unwrap()` on that turns one failed worker into a crashed program.
//
// TIP: `join()` -> `Err(payload)`; `payload` is `Box<dyn Any + Send>`.
// TIP: `payload.downcast_ref::<&str>()` for `panic!("literal")`,
//      `payload.downcast_ref::<String>()` for `panic!("{}", formatted)`.
fn validate_and_check(cfg: &ServerConfig) -> u64 {
    if cfg.port == 0 {
        panic!("port 0 is not a valid port");
    }
    health_check(cfg)
}

fn step_7_panics_and_anti_patterns() {
    println!("\n=== Step 7: Thread Panics & Anti-Patterns ===");

    let mut broken = make_config(7);
    broken.port = 0;

    let handle = thread::Builder::new()
        .name("health-worker-bad".to_string())
        .spawn(move || validate_and_check(&broken))
        .expect("OS failed to spawn a thread");

    // Expected on stderr (printed by the default panic hook, NOT an error of ours):
    //   thread 'health-worker-bad' (<id>) panicked at threads/threads_main/src/main.rs:..
    //   port 0 is not a valid port
    // Also: `broken` is still dropped — unwinding runs destructors in that thread.
    match handle.join() {
        Ok(latency) => println!("unexpected success: {latency}"),
        Err(payload) => {
            let msg = payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("<non-string panic payload>");
            println!("worker panicked, main survived. reason: {msg}");
        }
    }

    // [BREAK-IT] Blindly unwrapping the join result.
    //
    // UNCOMMENT the lines below and run `cargo run -p threads_main`:
    /*
    let mut broken = make_config(7);
    broken.port = 0;
    let latency = thread::spawn(move || validate_and_check(&broken)).join().unwrap();
    println!("{latency}");
    */
    //
    // What you'll see at runtime:
    //   thread '<unnamed>' (<id>) panicked at threads/threads_main/src/main.rs:..:
    //   port 0 is not a valid port
    //   thread 'main' (<id>) panicked at threads/threads_main/src/main.rs:..:
    //   called `Result::unwrap()` on an `Err` value: Any { .. }
    //
    // Why:
    //   The worker's panic is re-raised in main by `unwrap()`. `Any { .. }` is the
    //   opaque payload — the useful message is only in the FIRST line. Handle `Err`
    //   (as above) when one worker failing must not take the whole program down.

    // [ANTI-PATTERN] (compiles but wrong) Joining inside the spawn loop.
    // This one is LIVE so you can see the timing:
    let start = Instant::now();
    for cfg in make_fleet(4) {
        let h = thread::spawn(move || health_check(&cfg));
        h.join().unwrap(); // ❌ waits for each thread before spawning the next
    }
    println!(
        "join-in-loop: 4 checks took {:?} ❌ (compare with Step 6)",
        start.elapsed()
    ); // Expected: ~80 ms — fully sequential, plus thread-creation overhead
    //
    // Why it's bad:
    //   Compiles and "works", but there is zero parallelism — you paid for 4 thread
    //   creations and got sequential execution.
    // Idiomatic ✅:
    //   Collect all handles first, join afterwards (Step 6).

    // [ANTI-PATTERN] (compiles but wrong) One OS thread per item.
    //
    // let fleet = make_fleet(10_000);
    // let handles: Vec<_> = fleet
    //     .into_iter()
    //     .map(|cfg| thread::spawn(move || health_check(&cfg))) // ❌ 10_000 OS threads
    //     .collect();
    //
    // Why it's bad:
    //   Each OS thread costs a stack (~2 MiB reserved by default on Linux) plus
    //   kernel scheduling. Past a few × CPU cores you only add overhead, and
    //   `thread::spawn` can even panic when the OS refuses more threads.
    // Idiomatic ✅:
    //   Spawn ~`thread::available_parallelism()` threads and give each a CHUNK of
    //   the work (Step 8), or use a thread pool (e.g. the `rayon` crate).
}

// =============================================================================
// STEP 8 [CHALLENGE]: Parallel Fleet Health Check
// =============================================================================
// Task:
//   Health-check a fleet of 12 servers in parallel and return
//   `(total_latency_ms, total_max_connections)`.
//
// Rules:
//   1. Don't move or clone the fleet — BORROW it with `thread::scope` (Step 4).
//   2. Use at most `thread::available_parallelism()` threads; split the fleet with
//      `.chunks(chunk_size)` so each thread handles a slice (Step 7 anti-pattern).
//   3. Each thread RETURNS its partial sums (Step 5) — no shared mutable state.
//   4. Join all handles after spawning all of them (Step 6) and combine.
//   5. Must equal the sequential result (checked with `assert_eq!` below).
fn parallel_fleet_report(fleet: &[ServerConfig]) -> (u64, usize) {
    let n_threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(fleet.len())
        .max(1);
    let chunk_size = fleet.len().div_ceil(n_threads).max(1);

    thread::scope(|s| {
        let handles: Vec<_> = fleet
            .chunks(chunk_size)
            .map(|chunk| {
                s.spawn(move || {
                    // `move` here moves only the `chunk` slice REFERENCE (a cheap
                    // `&[ServerConfig]`), not the configs themselves.
                    chunk.iter().fold((0u64, 0usize), |(lat, conns), cfg| {
                        (lat + health_check(cfg), conns + cfg.max_connections)
                    })
                })
            })
            .collect();

        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .fold((0, 0), |(lat, conns), (l, c)| (lat + l, conns + c))
    })
}

fn sequential_fleet_report(fleet: &[ServerConfig]) -> (u64, usize) {
    fleet
        .iter()
        .map(|cfg| (health_check(cfg), cfg.max_connections))
        .fold((0, 0), |(lat, conns), (l, c)| (lat + l, conns + c))
}

fn step_8_challenge() {
    println!("\n=== Step 8: Challenge — Parallel Fleet Health Check ===");
    let fleet = make_fleet(12);

    let start = Instant::now();
    let parallel = parallel_fleet_report(&fleet);
    let parallel_time = start.elapsed();

    let start = Instant::now();
    let sequential = sequential_fleet_report(&fleet);
    let sequential_time = start.elapsed();

    // Expected: (164, 7800) in ~20 ms × ceil(12 / CPU cores), e.g. ~60 ms on 4 cores
    println!("parallel:   {parallel:?} in {parallel_time:?}");
    println!("sequential: {sequential:?} in {sequential_time:?}"); // Expected: ~240 ms (12 × 20 ms)
    assert_eq!(parallel, sequential, "parallel result must match sequential");
    println!("✅ results match; the fleet is still owned by main ({} configs)", fleet.len());
    // Expected: 12 [DROP] lines in thread 'main' when `fleet` goes out of scope.
}

// -----------------------------------------------------------------------------
// Driver: runs all steps in order
// -----------------------------------------------------------------------------
fn main() {
    println!(">>> Starting Threads Playground <<<");
    step_1_spawn_and_join();
    step_2_borrow_fails();
    step_3_move_closure();
    step_4_scoped_threads();
    step_5_return_values();
    step_6_many_workers();
    step_7_panics_and_anti_patterns();
    step_8_challenge();
    println!("\n>>> Playground completed successfully! <<<");
}
