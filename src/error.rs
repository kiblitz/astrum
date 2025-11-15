use crate::import::*;

pub type Result<T> = color_eyre::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    PrefixTree {
        source: prefix_tree::Error,
    },
    IoError {
        #[snafu(source(from(io::Error, Rc::new)))]
        source: Rc<io::Error>,
    },
    ReportError {
        #[snafu(source(from(eyre::ErrReport, Rc::new)))]
        source: Rc<eyre::ErrReport>,
    },
}
