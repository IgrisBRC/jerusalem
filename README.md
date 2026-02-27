# Jerusalem ⚔️

This is under development, if you wanna know about the progress, keep reading. 

## What it supports right now

Note: Every command is supposed to work just like how it would in Redis.

| Command | Category | Usage | Description |
| :--- | :--- | :--- | :--- |
| **SET** | String | `SET key value [EX seconds]` | Stores a string; supports optional TTL |
| **GET** | String | `GET key` | Retrieves a value; handles passive expiry |
| **APPEND** | String | `APPEND key value` | Appends to a key; acts like SET |
| **INCR / DECR** | String | `INCR key` | Atomic integer math on string values |
| **STRLEN** | String | `STRLEN key` | Retrieves length of string |
| **LPUSH / RPUSH**| List | `LPUSH key val [val...]` | Pushes to the Front (Left) or Back (Right) |
| **LPOP / RPOP** | List | `LPOP key [count]` | Pops element(s) from the Front or Back |
| **LRANGE** | List | `LRANGE key start stop` | Returns a slice of the list |
| **LREM** | List | `LREM key count element` | Removes elements based on directional search |
| **LINDEX** | List | `LINDEX key index` | Retrieves one element at given index in a list |
| **LLEN** | List | `LLEN key` | Retrieves number of elements in a list|
| **LSET** | List | `LSET key index element` | Replaces an element at a specific index |
| **HSET / HMGET** | Hash | `HSET key field val...` | Sets or gets fields within a hash map |
| **HGET** | Hash | `HGET key field` | Gets fields within a hash map |
| **HDEL** | Hash | `HDEL key field...` | Removes one or more fields from a hashmap |
| **HEXISTS** | Hash | `HEXISTS key field` | Checks for one field in a hashmap |
| **HLEN** | Hash | `HLEN key` | Retrieves the hashmap's size |
| **EXISTS** | Generic | `EXISTS key [key...]` | Checks for the presence of keys |
| **DEL** | Generic | `DEL key [key...]` | Removes keys of any data type |
| **PING** | System | `PING` | Returns `PONG` |

### A Note on PING
Currently, Jerusalem requires the standard RESP array protocol format for all commands. Some clients may attempt to send a "naked" PING during pipelining without the array marker (`*`). This is currently not supported to keep the parser logic clean and focused on standard protocol adherence.

## Usage

```bash
cargo r --release
```

Note: If you want to handle more than 508 concurrent connections, you may have to set ulimit to a higher number than 1024.

## Crates used

https://crates.io/crates/mio

## My order of operations for this project

Sets, and expiry controls.

Make the error messages.

Make relevant optimizations.

## Maybe plans

Sharding

## todo

Support sets and expiry.

Match through all the possible errors in egress.rs and give appropriate error message.

Slay utf8 monster throughout the codebase.

Restrict the number of arguments accepted for all the commands.

Slay the drain monster in wish.rs.

This is a maybe cuz it's hard, but slay the mutex monster in choir.rs.

