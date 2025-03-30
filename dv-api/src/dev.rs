use crate::{Os, Pm};

#[derive(Debug)]
pub struct Dev {
    pub pm: Pm,
    pub os: Os,
}
