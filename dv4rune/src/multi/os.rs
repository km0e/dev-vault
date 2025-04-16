use dv_api::util::Os;

#[rune::function(instance)]
fn compat(this: &Os, os: &str) -> bool {
    let os = Os::from(os);
    this.compatible(&os)
}

#[rune::function(instance)]
fn as_str(this: &Os) -> String {
    this.to_string()
}
pub fn register(m: &mut rune::module::Module) -> Result<(), rune::ContextError> {
    m.ty::<Os>()?;
    m.function_meta(compat)?;
    m.function_meta(as_str)?;
    Ok(())
}
