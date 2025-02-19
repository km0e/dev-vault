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
        $ctx.assert_result(obj_take2!($o, stringify!($k), $v, $p))
    };
    ($ctx:expr, $o:ident, $k:ident($t:ty)) => {
        $ctx.assert_result(obj_take2!($o, stringify!($k), <$t>::default(), $t))
    };
    ($ctx:expr, $o:ident, $k:ident($v:expr,)) => {
        $ctx.assert_result(obj_take2!($o, stringify!($k), $v, String))
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
