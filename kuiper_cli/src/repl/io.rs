macro_rules! printerr {
    ( $description:expr, $error:expr ) => {
        eprintln!("{} {} {}", "Error:".red(), $description, $error)
    };
}
pub(crate) use printerr;
