#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

mod server;

fn main() {
    server::run(
        "127.0.0.1",
        6379,
        3,
        128,
        std::env::current_dir().expect("Couldn't get current directory"),
    );
}
