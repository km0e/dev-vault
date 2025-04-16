use dv_api::user::Config;

#[rune::function(free,path = Config::cur)]
fn current_user_config() -> Config {
    let mut cfg = Config::default();
    cfg.insert("HID", "local");
    cfg.insert("MOUNT", "~/.local/share/dv");
    cfg.insert("OS", "linux");
    cfg
}

#[rune::function(free, path = Config::ssh)]
fn ssh_user_config(host: &str) -> Config {
    let mut cfg = Config::default();
    cfg.insert("HOST", host);
    cfg.insert("MOUNT", "~/.local/share/dv");
    cfg.insert("OS", "linux");
    cfg
}

#[rune::function(instance, protocol = INDEX_SET)]
fn config_index_set(this: &mut Config, key: String, value: String) {
    this.insert(key, value);
}

pub fn register(m: &mut rune::module::Module) -> Result<(), rune::ContextError> {
    m.ty::<Config>()?;
    m.function_meta(current_user_config)?;
    m.function_meta(ssh_user_config)?;
    m.function_meta(config_index_set)?;
    Ok(())
}
