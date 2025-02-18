macro_rules! assert_option {
    ($o:expr, $interactor:expr, $m:expr) => {
        match $o {
            Some(v) => v,
            None => {
                fn _f<S: Into<String>>(f: impl FnOnce() -> S) -> String {
                    f().into()
                }
                let _s = _f($m);
                $interactor.log(&_s).await;
                return None;
            }
        }
    };
}

pub(crate) use assert_option;

macro_rules! assert_bool {
    ($b:expr, $interactor:expr, $m:expr) => {
        assert_option!(($b).then_some(()), $interactor, $m)
    };
}

pub(crate) use assert_bool;

macro_rules! assert_result {
    ($r:expr, $interactor:expr, $m:expr) => {
        match $r {
            Ok(v) => v,
            Err(e) => {
                fn _f(e: String, f: impl FnOnce(String) -> impl Into<String>) -> String {
                    f(e).into()
                }
                $interactor.log(&_f(e.to_string(), $m)).await;
                return None;
            }
        }
    };
    ($r:expr, $interactor:expr) => {
        match $r {
            Ok(v) => v,
            Err(e) => {
                let _s = e.to_string();
                $interactor.log(&_s).await;
                return None;
            }
        }
    };
}

pub(crate) use assert_result;

macro_rules! value2 {
    ($t:ty, $v:expr) => {
        rune::from_value::<$t>($v).map_err(|e| e.to_string())
    };
    ($v:expr) => {
        rune::from_value::<String>($v).map_err(|e| e.to_string())
    };
}

pub(crate) use value2;

macro_rules! obj_take2 {
    // or default
    ($o:ident@($k:expr, $v:expr, $t:ty)) => {
        $o.remove($k)
            .map(|v| value2!($t, v))
            .transpose()
            .map(|v| v.unwrap_or($v.into()))
    };
    ($o:ident@($k:expr,, $t:ty)) => {
        obj_take2!($o@($k, <$t>::default(), $t))
    };
    ($o:ident@($k:expr, $v:expr,)) => {
        obj_take2!($o@($k, $v, String))
    };
    ($o:ident@($k:expr,,)) => {
        obj_take2!($o@($k, "", String))
    };
    // must have
    ($o:ident@($k:expr, $t:ty)) => {
        $o.remove($k)
            .ok_or_else(|| format!("{} not found", $k))
            .and_then(|v| value2!($t, v))
    };
    ($o:ident@$k:expr) => {
        obj_take2!($o@($k, String))
    };
}

pub(crate) use obj_take2;

macro_rules! field {
    ($interactor:expr, $o:ident, $k:ident@) => {
        assert_result!(obj_take2!($o@stringify!($k)), $interactor)
    };
    ($interactor:expr, $o:ident, $k:ident@($v:expr, $t:ty)) => {
        assert_result!(obj_take2!($o@(stringify!($k), $v, $t)), $interactor)
    };
    ($interactor:expr, $o:ident, $k:ident@(, $t:ty)) => {
        assert_result!(obj_take2!($o@(stringify!($k),, $t)), $interactor)
    };
    ($interactor:expr, $o:ident, $k:ident@($v:expr)) => {
        assert_result!(obj_take2!($o@(stringify!($k), $v,)), $interactor)
    };
    ($interactor:expr, $o:ident, $k:ident@()) => {
        assert_result!(obj_take2!($o@(stringify!($k),,)), $interactor)
    };
}
pub(crate) use field;

macro_rules! obj2 {
    ($st:ident, $interactor:expr, $o:ident, $($k:ident$(@$t:tt)?),+ $(, @$d:ident)?) => {
        $st {
            $(
                $k: field!($interactor, $o, $k@$($t)?).into(),
            )+
            $(
                ..Default::$d()
            )?
        }
    };
}

pub(crate) use obj2;
