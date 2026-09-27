pub mod board;

const _: () = assert!(app::is_version(env!("CARGO_PKG_VERSION")), "the firmware package's version is not app::VERSION");
