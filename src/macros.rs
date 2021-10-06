#[macro_export]
macro_rules! exit_impl {
    ($exit_code:literal, $format_str:expr $(,$args:expr)*) => {
        {
            log::error!($format_str $(,$args)*);
            std::process::exit($exit_code);
        }
    }
}

#[macro_export]
macro_rules! exit {
    (1, $arg1:expr) => {
        $crate::exit_impl!(1, "invalid port: `{}`", $arg1);
    };
    (2, $arg1:expr) => {
        $crate::exit_impl!(2, "port `{}` is already in use", $arg1);
    };
    (3) => {
        $crate::exit_impl!(3, "no available ports");
    };
    (4, $arg1:expr) => {
        $crate::exit_impl!(4, "invalid number of threads: `{}`", $arg1);
    };
    (5, $arg1:expr) => {
        $crate::exit_impl!(5, "failed to connect to peer: `{}`", $arg1);
    };
    (6, $arg1:expr) => {
        $crate::exit_impl!(6, "failed to convert into SocketAddr: `{}`", $arg1);
    };
    (7, $arg1:expr) => {
        $crate::exit_impl!(7, "output path is not a directory: `{}`", $arg1);
    };

    ($invalid_exit_code:literal) => {
        compile_error!(concat!("No such exit code: ", $invalid_exit_code));
    };
}
