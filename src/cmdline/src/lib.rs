//! Helper library for parsing command line arguments

#![deny(clippy::panic)]
#![warn(missing_docs)]

pub use lexopt;

/// An argument that can go alongside a command line option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Value<'a> {
    /// should be uppercase
    pub name: &'a str,
    /// `[NAME]` instead of `<NAME>`
    pub optional: bool,
    /// `...`
    pub variadic: bool,
}

impl std::fmt::Display for Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name;
        let (open, close) = if self.optional {
            ('[', ']')
        } else {
            ('<', '>')
        };
        let trail = if self.variadic { "..." } else { "" };
        write!(f, "{open}{name}{close}{trail}")
    }
}

impl<'a> Value<'a> {
    /// Construct a new [`Value`] with a name, setting all other fields to their defaults.
    ///
    /// # Convention
    ///
    /// `name` should be all uppercase and preferably one word.
    pub const fn new(name: &'a str) -> Self {
        Self {
            name,
            optional: false,
            variadic: false,
        }
    }

    /// Mark the value as optional (wrap with `[]` instead of `<>`).
    pub const fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Mark the value as variadic (append with `...`).
    pub const fn variadic(mut self) -> Self {
        self.variadic = true;
        self
    }

    /// The length of the display string in bytes.
    #[allow(
        clippy::len_without_is_empty,
        reason = "`Value` will never be empty, because it always contains either `[]` or `<>`"
    )]
    pub const fn len(&self) -> usize {
        self.name.len()
            + if self.optional { "[]" } else { "<>" }.len()
            + if self.variadic { "..." } else { "" }.len()
    }
}

/// An optional argument.
#[derive(Debug, Default)]
pub struct RunOption<'l, 'v, 'm> {
    /// The [`Short`] option.
    pub short: Option<char>,

    /// The [`Long`] option.
    pub long: Option<&'l str>,

    /// An optional or required [`Value`](`lexopt::prelude::Value`)
    /// expected to follow [`long`](Self::long) or [`short`](Self::short)
    pub val: Option<Value<'v>>,

    /// The option's brief help description
    pub msg: &'m str,
}

impl<'l, 'v, 'm> RunOption<'l, 'v, 'm> {
    /// Construct a new [`RunOption`] from its help description.
    pub const fn new(msg: &'m str) -> Self {
        Self {
            short: None,
            long: None,
            val: None,
            msg,
        }
    }

    /// Add a [`Short`] option.
    pub const fn with_short(mut self, ch: char) -> Self {
        self.short = Some(ch);
        self
    }

    /// Add a [`Long`] option.
    pub const fn with_long(mut self, s: &'l str) -> Self {
        self.long = Some(s);
        self
    }

    /// Add a [`Value`](lexopt::prelude::Value) option.
    pub const fn with_value(mut self, val: Value<'v>) -> Self {
        self.val = Some(val);
        self
    }
}

/// Write the help message to a [`Write`](std::io::Write) implementor.
///
/// `usages[i][j].0`: If true, style as literal text. Otherwise, style as a placeholder.
///
/// **See also:** [`print_help`]
pub fn write_help<W>(
    mut w: W,
    bin_name: &str,
    usages: &[&[(bool, &str)]],
    options: &[RunOption<'_, '_, '_>],
) -> std::io::Result<()>
where
    W: std::io::Write,
{
    const RESET_STYLE: &str = "\x1B[0m";
    const NAME_STYLE: &str = "\x1B[36m";
    const LIT_STYLE: &str = "\x1B[1;96m";
    const HEADER_STYLE: &str = "\x1B[1;92m";

    let longest_short = if options.iter().any(|opt| opt.short.is_some()) {
        "-*".len()
    } else {
        0
    };

    let longest_long = options
        .iter()
        .filter_map(|opt| opt.long)
        .map(|x| "--".len() + x.len())
        .max()
        .unwrap_or(0);

    let longest_val = options
        .iter()
        .filter_map(|opt| opt.val)
        .map(|x| x.len())
        .max()
        .unwrap_or(0);

    write!(w, "{HEADER_STYLE}Usage: {RESET_STYLE}")?;
    for usage in usages {
        write!(w, "{LIT_STYLE}{bin_name}{RESET_STYLE}")?;
        for (bold, text) in *usage {
            write!(
                w,
                " {}{text}{RESET_STYLE}",
                if *bold { LIT_STYLE } else { NAME_STYLE }
            )?;
        }
        writeln!(w)?;
        write!(w, "{:indent$}", "", indent = "Usage: ".len())?;
    }
    writeln!(w)?;

    writeln!(w, "{HEADER_STYLE}Options:{RESET_STYLE}")?;
    for option in options {
        let comma = if option.short.is_some() { ',' } else { ' ' };
        let short = option.short.map(|ch| format!("-{ch}")).unwrap_or_default();
        let long = option.long.map(|s| format!("--{s}")).unwrap_or_default();
        let val = option.val.map(|v| v.to_string()).unwrap_or_default();
        let msg = option.msg;
        writeln!(
            w,
            "  {LIT_STYLE}{short:>short_width$}{RESET_STYLE}{comma} {LIT_STYLE}{long:<long_width$}{RESET_STYLE} {NAME_STYLE}{val:<val_width$}{RESET_STYLE}  {msg}",
            short_width = longest_short,
            long_width = longest_long,
            val_width = longest_val,
        )?;
    }

    Ok(())
}

/// [`print`] version of [`write_help`].
pub fn print_help(
    bin_name: &str,
    usages: &[&[(bool, &str)]],
    options: &[RunOption<'_, '_, '_>],
) -> std::io::Result<()> {
    write_help(std::io::stdout().lock(), bin_name, usages, options)
}

/// Parse an argument with [`lexopt`], automatically generating help text for [`write_help`]/[`print_help`].
#[macro_export]
macro_rules! parse_arg {
    // yes, these do have to be repetitive.
    // making all the arguments optional would allow users of the macro to provide invalid combinations.
    (@[$msg:expr] -$short:literal) => {
        $crate::RunOption::new($msg)
            .with_short($short)
    };
    (@[$msg:expr] -$short:literal, --$long:literal) => {
        $crate::RunOption::new($msg)
            .with_short($short)
            .with_long($long)
    };
    (@[$msg:expr] --$long:literal) => {
        $crate::RunOption::new($msg)
            .with_short($short)
            .with_long($long)
    };
    (@[$msg:expr] -$short:literal <$val:ident>) => {
        $crate::RunOption::new($msg)
            .with_short($short)
            .with_value($crate::Value::new(stringify!($val)))
    };
    (@[$msg:expr] -$short:literal, --$long:literal <$val:ident>) => {
        $crate::RunOption::new($msg)
            .with_short($short)
            .with_long($long)
            .with_value($crate::Value::new(stringify!($val)))
    };
    (@[$msg:expr] --$long:literal <$val:ident>) => {
        $crate::RunOption::new($msg)
            .with_short($short)
            .with_long($long)
            .with_value($crate::Value::new(stringify!($val)))
    };
    (@[$msg:expr] -$short:literal [$opt_val:ident]) => {
        $crate::RunOption::new($msg)
            .with_short($short)
            .with_value($crate::Value::new(stringify!($opt_val)).optional())
    };
    (@[$msg:expr] -$short:literal, --$long:literal [$opt_val:ident]) => {
        $crate::RunOption::new($msg)
            .with_short($short)
            .with_long($long)
            .with_value($crate::Value::new(stringify!($opt_val)).optional())
    };
    (@[$msg:expr] --$long:literal [$opt_val:ident]) => {
        $crate::RunOption::new($msg)
            .with_short($short)
            .with_long($long)
            .with_value($crate::Value::new(stringify!($opt_val)).optional())
    };

    (# -$short:literal, --$long:literal $(<$val:ident>)? $([$opt_val:ident])?) => {
        $crate::lexopt::prelude::Short($short) | $crate::lexopt::prelude::Long($long)
    };
    (# -$short:literal $(<$val:ident>)? $([$opt_val:ident])?) => {
        $crate::lexopt::prelude::Short($short)
    };
    (# --$long:literal $(<$val:ident>)? $([$opt_val:ident])?) => {
        $crate::lexopt::prelude::Long($long)
    };
    (# $value:ident) => {
        $crate::lexopt::prelude::Value($value)
    };

    (%[$parser:ident, $expr:expr] $(-$short:literal$(,)?)? $(--$long:literal)? <$val:ident>) => {
        {
            #[allow(non_snake_case)]
            let $val = $parser.value();
            $expr
        }
    };
    (%[$parser:ident, $expr:expr] $(-$short:literal$(,)?)? $(--$long:literal)? [$opt_val:ident]) => {
        {
            #[allow(non_snake_case)]
            let $opt_val = $parser.optional_value();
            $expr
        }
    };
    (%[$parser:ident, $expr:expr] $(-$short:literal$(,)?)? $(--$long:literal)?) => {
        $expr
    };

    (
        options = $OPTIONS:ident;
        parser = $parser:ident;
        match $arg:ident {
            $(
                $(#[help = $msg:expr])?
                ($($pattern:tt)*) => $expr:expr,
            )*
            _ => $rest:expr $(,)?
        }
    ) => {
        static $OPTIONS: &[$crate::RunOption] = &[
            $($crate::parse_arg!(@[$($msg)?] $($pattern)*)),*
        ];
        match $arg {
            $($crate::parse_arg!(# $($pattern)*) => $crate::parse_arg!(%[$parser, $expr] $($pattern)*),)*
            _ => $rest
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test0() {
        let mut res = Vec::new();
        let mut parser = lexopt::Parser::from_args(["-t", "--test=squeak", "-h"]);
        while let Some(arg) = parser.next().unwrap() {
            parse_arg! {
                options = OPTIONS;
                parser = parser;
                match arg {
                    #[help = "test"]
                    ( -'t', --"test" [OPT] ) => res.push(format!("test: {OPT:?}")),

                    #[help = "help"]
                    ( -'h' ) => write_help(std::io::stdout().lock(), "cmdline", &[&[(false, "[OPTIONS]")]], OPTIONS).unwrap(),

                    _ => todo!()
                }
            }
        }
        assert_eq!(res.as_slice(), &["test: None", "test: Some(\"squeak\")"]);
    }
}
