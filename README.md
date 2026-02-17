# IgrisDB ⚔️

A high-performance, in-memory data store implemented in **Rust**. IgrisDB utilizes the **RESP (Redis Serialization Protocol)** to provide a fast, thread-safe, and reliable key-value storage engine.

## 🚀 Architectural Overview
IgrisDB is designed around a **Single-Threaded Worker (Actor Model)**. Instead of using heavy global locks (`Arc<Mutex<Database>>`) that cause contention, IgrisDB routes requests through a synchronized queue to a dedicated database thread. This ensures:
- **Lock-Free Data Access:** The core HashMap is only ever touched by one thread.
- **Predictable Performance:** No thundering herd problems or deadlocks.
- **Memory Safety:** Leverages Rust's ownership model to manage state transitions across thread boundaries.

## ✨ Features
- **Full RESP Implementation:** Handles Simple Strings, Errors, Integers, Bulk Strings, and Arrays.
- **Atomic Counters:** `INCR` support with integrated string-to-integer parsing and validation.
- **Lazy Expiration (The Reaper):** Custom TTL logic that validates and purges expired keys during access to maximize throughput.
- **Variadic Commands:** Efficient multi-key handling for `DEL` and `EXISTS`.
- **Sass-Enhanced Errors:** Descriptive (and occasionally opinionated) error messages for protocol violations.

## 🛠 Commands Supported
| Command | Description | Variadic |
| :--- | :--- | :--- |
| `GET` | Retrieve the value of a key | No |
| `SET` | Set key to hold string value | No |
| `DEL` | Removes the specified keys | **Yes** |
| `PING` | Returns `PONG` to test connection | No |

