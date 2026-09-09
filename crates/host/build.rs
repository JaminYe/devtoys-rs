fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let _ = embed_resource::compile("assets/devtoys.rc", embed_resource::NONE);
    }
}
