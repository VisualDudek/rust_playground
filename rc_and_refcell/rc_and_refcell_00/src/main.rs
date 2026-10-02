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
todo!("Implement Drop for ServerConfig to observe heap deallocation");

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
    todo!("Construct an Rc<ServerConfig> with the default values specified in the comments");
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

// TIP: Use `Rc::clone(&rc_instance)` to create a new reference to the same heap allocation without deep-copying.
// Use `Rc::strong_count(&rc_instance)` to check the current reference count.
fn step_3_shared_ownership() {
    todo!();
}


// -----------------------------------------------------------------------------
// Driver: Runs All Steps
// -----------------------------------------------------------------------------
fn main() {
    println!(">>> Starting Rust Rc / RefCell Playground <<<\n");

    // Exercise 3
    step_3_shared_ownership();

    println!("\n>>> Playground completed successfully! <<<");
}