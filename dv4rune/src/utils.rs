pub trait LogResult<I: Interactor> {
    async fn log(self, int: &I) -> Self;
}

impl<I: Sync + Interactor, T, E: ToString> LogResult<I> for Result<T, E> {
    async fn log(self, int: &I) -> Self {
        match &self {
            Ok(_) => {}
            Err(e) => {
                int.log(e.to_string()).await;
            }
        }
        self
    }
}

pub trait LogFutResult<I: Interactor> {
    type Result;
    async fn log(self, int: &I) -> Self::Result;
}

impl<I: Sync + Interactor, Fut: Future<Output = Result<T, E>>, T, E: ToString> LogFutResult<I>
    for Fut
{
    type Result = Result<T, E>;
    async fn log(self, int: &I) -> Self::Result {
        self.await.log(int).await
    }
}

use dv_api::process::Interactor;
use rune::{
    ast, compile,
    macros::{MacroContext, TokenStream, quote},
    parse::Parser,
};

/// Implementation of the `stringy_math!` macro.
#[rune::macro_]
pub fn stringy_math(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream, cx.input_span());

    let mut output = quote!(0);

    while !parser.is_eof()? {
        let op = parser.parse::<ast::Ident>()?;
        let arg = parser.parse::<ast::Expr>()?;

        output = match cx.resolve(op)? {
            "add" => quote!((#output) + #arg),
            "sub" => quote!((#output) - #arg),
            "div" => quote!((#output) / #arg),
            "mul" => quote!((#output) * #arg),
            _ => return Err(compile::Error::msg(op, "unsupported operation")),
        }
    }

    parser.eof()?;
    Ok(output.into_token_stream(cx)?)
}
