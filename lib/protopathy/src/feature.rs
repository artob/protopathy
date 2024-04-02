// This is free and unencumbered software released into the public domain.

#[allow(unused)]
pub static FEATURES: &[&str] = &[
    #[cfg(feature = "tcp")]
    "tcp",
    #[cfg(feature = "tracing")]
    "tracing",
];
