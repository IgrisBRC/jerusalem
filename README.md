# Jerusalem ⚔️

This is under development, if you wanna know about the progress, keep reading. 

## What it supports right now

| Command | Category | Status | Notes |
| :--- | :--- | :--- | :--- |
| SET | String | Supported | Now supports EX (seconds) for TTL |
| GET | String | Supported | Functional |
| DEL | Any type | Supported | Can delete any available data type |
| APPEND | String | Supported | Working |
| INCR | String | Supported | Atomic increment for numeric strings |
| DECR | String | Supported | Atomic decrement for numeric strings |
| PING | Connection | Supported | Functional, but does not support incorrect pipelining (more on that further) | 
| HSET | Hashmap | Supported | Supports single setting and multiple values |
| HGET | Hashmap | Supported | Supports single values |
| HMGET | Hashmap | Supported | Supports single and multiple values |

## My order of operations for this project

Currently working on operations for data types like lists and sets. And also for more operations of Hashmap.

As for the PING command pipelining, for some reason when you pipeline it, the PING command array is sent without the *. And I honestly have no clue why they have done it like this, surely there must be good reason. The reason why I don't support it, because (main reason is that I don't it makes sense because in the real wrold no one's gonna do that (I don't think)) the code would get a bit messy.

## Maybe plans

Sharding

## todo

Match through all the possible errors in egress.rs and give appropriate error message.

Support list, and sets.

Slay the drain monster in wish.rs.

## Issues

The drain monster still lives. (The drain monster is the usage of .drain() in the parsing logic. It makes it easier to parse but is O(n), which makes things slightly slower).

