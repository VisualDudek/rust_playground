// =============================================================================
// Rust Learning Playground: Demystifying Rc<T> and RefCell<T>
//
// Topic:
//   Solving multiple ownership and function-return lifetime issues using
//   `std::rc::Rc`, followed by interior mutability with `std::cell::RefCell`.
//
// How to use this file:
//   1. Run `cargo run` immediately. It compiles and runs in its baseline state.
//   2. Work through the numbered exercises sequentially (Step 1 -> Step 8).
//   3. Each step is tagged as [BUILD], [BREAK-IT], or [FIX-IT].
//   4. Follow the commented instructions to replace `todo!()`, uncomment code
//      to trigger deliberate compiler/runtime errors, observe why Rust stops
//      you, and then apply the guided fix.
// =============================================================================

use std::cell::RefCell;
use std::rc::Rc;

// -----------------------------------------------------------------------------
// Domain Struct: Shared Server Configuration
// -----------------------------------------------------------------------------
#[derive(Debug)]
struct ServerConfig {
    host: String,
    port: u16,
    max_connections: usize,
}

// Custom Drop implementation so you can visually verify on the console
// exactly when heap deallocation takes place.
impl Drop for ServerConfig {
    fn drop(&mut self) {
        println!("  [DROP NOTIFICATION] ServerConfig for '{}:{}' deallocated from heap!", self.host, self.port);
    }
}

// -----------------------------------------------------------------------------
// Consumers of ServerConfig
// -----------------------------------------------------------------------------
struct WebService {
    name: String,
    config: Rc<ServerConfig>,
}


struct MetricsWorker {
    #[allow(dead_code)]
    worker_id: u32,
    config: Rc<ServerConfig>,
}

// =============================================================================
// STEP 1 [BUILD]: Returning Owned Heap Data From a Factory Function
// =============================================================================
// Why this matters:
//   A standard reference `&ServerConfig` cannot be returned from a function if
//   the data is created inside that function—the stack frame is destroyed upon
//   return. `Rc::new(...)` places the value onto the heap, producing an owned
//   handle that can safely escape any stack frame without lifetime annotations.
//
// Task:
//   Replace `todo!()` by constructing an `Rc<ServerConfig>` with:
//   - host: "127.0.0.1".to_string()
//   - port: 8080
//   - max_connections: 1000
fn make_default_config() -> Rc<ServerConfig> {
    Rc::new(ServerConfig {
        host: String::from("127.0.0.1"),
        port: 8080,
        max_connections: 1000,
    })
}

// =============================================================================
// STEP 2 [BREAK-IT]: The "Flawed Alternative" — Why Not Just Return `&'a ServerConfig`?
// =============================================================================
// Why people try this:
//   "I don't want heap allocation overhead, so I'll just return a reference!"
//
// Uncomment the function below and run `cargo check`:
//
// fn create_dangling_config<'a>() -> &'a ServerConfig {
//     let local_cfg = ServerConfig {
//         host: String::from("0.0.0.0"),
//         port: 3000,
//         max_connections: 50,
//     };
//     &local_cfg
// }
//
// What the compiler reports:
//   error[E0515]: cannot return reference to local variable `local_cfg`
//   returns a value referencing data owned by the current function
//
// Why:
//   `local_cfg` is allocated on this function's stack frame. As soon as
//   `create_dangling_config` exits, that memory is invalidated. Allowing this
//   would cause undefined behavior / memory corruption.
//   Keep this commented out and proceed to Step 3.

// =============================================================================
// STEP 3 [BUILD & OBSERVATION]: Shared Ownership & Reference Counts
// =============================================================================
// Demonstrates that `Rc::clone` does NOT deep-copy heap data; it only increments
// an internal reference counter.
fn step_3_shared_ownership() {
    println!("\n=== Step 3: Shared Ownership & Reference Counting ===");

    let config = make_default_config();
    println!("1. After creation, strong count: {}", Rc::strong_count(&config)); // Expected: 1

    let web = WebService {
        name: String::from("AuthService"),
        config: Rc::clone(&config),
    };
    println!("2. After WebService takes an Rc, strong count: {}", Rc::strong_count(&config)); // Expected: 2

    let metrics = MetricsWorker {
        worker_id: 1,
        config: Rc::clone(&config),
    };
    println!("3. After MetricsWorker takes an Rc, strong count: {}", Rc::strong_count(&config)); // Expected: 3

    println!("   Both services reading identical config: {}:{}", web.config.host, metrics.config.port);
}

// =============================================================================
// STEP 4 [BUILD]: Verifying Dynamic Deallocation (Last Owner Cleans Up)
// =============================================================================
// Demonstrates decentralized ownership: no single variable is the "master".
// Heap cleanup occurs only when the counter drops to 0.
fn step_4_lifecycle_and_drop() {
    todo!("Implement step 4 to observe dynamic deallocation when the last Rc goes out of scope");
}

// =============================================================================
// STEP 5 [BREAK-IT]: Mutating Behind `Rc<T>` is Prohibited
// =============================================================================
// Why:
//   `Rc<T>` implements `Deref<Target = T>`, but explicitly DOES NOT implement
//   `DerefMut`. Rust's core aliasing model forbids shared mutable references
//   (`&mut T` while other `&T` or `&mut T` exist).
fn step_5_try_mutate_rc() {
    println!("\n=== Step 5: The Mutability Limitation of Rc<T> ===");
    let config = make_default_config();

    // UNCOMMENT the following line and run `cargo check`:
    // config.port = 9090;

    // What the compiler reports:
    //   error[E0594]: cannot assign to `config.port`, which is behind a `Rc` reference
    //   `Rc` is an immutable container, it cannot be used with `mut`
    //
    // Explanation:
    //   Because multiple pointers can point to this memory, permitting direct
    //   mutation would introduce data races in your application logic.
    //   Leave it commented and continue to Step 6.
    let _ = config;
}


// -----------------------------------------------------------------------------
// Driver: Runs All Steps
// -----------------------------------------------------------------------------
fn main() {
    println!(">>> Starting Rust Rc / RefCell Playground <<<\n");

    // Exercise 3
    step_3_shared_ownership();

    // Exercise 4
    step_4_lifecycle_and_drop();

    // Exercise 5 (Explanation & Compiler test)
    step_5_try_mutate_rc();

    println!("\n>>> Playground completed successfully! <<<");
}