use crate::{Os, Pm};

#[derive(Debug, Default)]
pub struct Dev {
    pub pm: Pm,
    pub os: Os,
}
