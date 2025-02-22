pub trait LogResult<I: Interactor> {
    async fn log(self, int: &I) -> Self;
}

impl<I: Sync + Interactor, T: Send, E: Send + ToString> LogResult<I> for Result<T, E> {
    async fn log(self, int: &I) -> Self {
        match &self {
            Ok(_) => {}
            Err(e) => {
                int.log(&e.to_string()).await;
            }
        }
        self
    }
}

pub trait LogFutResult<I: Interactor> {
    type Result;
    async fn log(self, int: &I) -> Self::Result;
}

impl<
    I: Sync + Interactor,
    Fut: Send + std::future::Future<Output = Result<T, E>>,
    T: Send,
    E: Send + ToString,
> LogFutResult<I> for Fut
{
    type Result = Result<T, E>;
    async fn log(self, int: &I) -> Self::Result {
        self.await.log(int).await
    }
}

macro_rules! value2 {
    ($t:ty, $v:expr) => {
        rune::from_value::<$t>($v)
    };
    ($v:expr) => {
        rune::from_value::<String>($v)
    };
}

use dv_api::process::Interactor;
pub(crate) use value2;

macro_rules! obj_take2 {
    ($o:ident, $k:expr, $v:expr, $t:ty) => {
        $o.remove($k)
            .map(|v| value2!($t, v))
            .transpose()
            .map(|v| v.unwrap_or($v.into()))
    };
    ($o:ident, $k:expr, $v:expr, $parse:expr) => {
        $o.remove($k)
            .map(|v| value2!(v))
            .transpose()
            .map(|v| $parse(v.unwrap_or($v.into())))
    };
}

pub(crate) use obj_take2;

//TODO:crate doc
//unwrap_or_default
//(default,type,parse)
//(default,parse)
//(,parse)
//(default,type)
//(,type)
//(type)
//must have
//(type,parse)
//type
//parse
//
//value
//(value)

macro_rules! field {
    ($ctx:expr, $o:ident, $k:ident($v:expr, $p:expr)) => {
        obj_take2!($o, stringify!($k), $v, $p).log($ctx.interactor)
    };
    ($ctx:expr, $o:ident, $k:ident($t:ty)) => {
        obj_take2!($o, stringify!($k), <$t>::default(), $t).log($ctx.interactor)
    };
    ($ctx:expr, $o:ident, $k:ident($v:expr,)) => {
        obj_take2!($o, stringify!($k), $v, String).log($ctx.interactor)
    };
}

pub(crate) use field;

macro_rules! obj2 {
    ($st:ident, $ctx:expr, $o:ident, $($k:ident$(@$t:tt)?),+ $(, @$d:ident)?) => {
        $st {
            $(
                $k: field!($ctx, $o, $k$($t)?).await?.into(),
            )+
            $(
                ..Default::$d()
            )?
        }
    };
}

pub(crate) use obj2;
