use crate::util::{Os, Pm};

#[derive(Debug)]
pub struct Dev {
    pub pm: Pm,
    pub os: Os,
}
