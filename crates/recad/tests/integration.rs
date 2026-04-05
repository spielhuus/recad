mod common;
mod integration {
    #[cfg(feature = "svg")]
    mod builder;
    mod erc;
    mod pcb;
    #[cfg(feature = "svg")]
    mod plot;
    mod rewrite;
    #[cfg(feature = "svg")]
    mod schema;
}

