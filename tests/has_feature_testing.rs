#![cfg(not(feature = "testing"))]

compile_error!("when testing, use `--features=testing`");
