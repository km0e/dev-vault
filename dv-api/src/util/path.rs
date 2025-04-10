use std::{
    borrow::{Borrow, Cow},
    cmp,
    ops::Deref,
    path::{Components, Path},
};

pub struct XPath(str);

impl<'a> From<&'a str> for &'a XPath {
    fn from(path: &'a str) -> Self {
        XPath::new(path)
    }
}

impl std::fmt::Display for XPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<Path> for &XPath {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl<'a> From<&'a XPath> for Cow<'a, XPath> {
    fn from(path: &'a XPath) -> Self {
        Cow::Borrowed(path)
    }
}

impl XPath {
    pub fn metadata(&self) -> std::io::Result<std::fs::Metadata> {
        Path::new(&self.0).metadata()
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn has_root(&self) -> bool {
        Path::new(&self.0).has_root()
    }
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }
    pub fn new<S: AsRef<str> + ?Sized>(s: &S) -> &XPath {
        unsafe { &*(s.as_ref() as *const str as *const XPath) }
    }
    pub fn components(&self) -> Components {
        Path::new(&self.0).components()
    }
}

#[derive(Debug, Clone)]
pub struct XPathBuf(String);

impl XPathBuf {
    /// just push a path to the end of the string
    pub fn push<T: AsRef<str>>(&mut self, path: T) {
        let path = path.as_ref();
        if !self.0.is_empty() && !self.0.ends_with('/') {
            self.0.push('/');
        }
        self.0.push_str(path);
    }
}

impl AsRef<str> for XPathBuf {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for XPathBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Deref for XPathBuf {
    type Target = XPath;

    fn deref(&self) -> &Self::Target {
        XPath::new(&self.0)
    }
}

impl Borrow<XPath> for XPathBuf {
    fn borrow(&self) -> &XPath {
        self.deref()
    }
}

impl ToOwned for XPath {
    type Owned = XPathBuf;

    fn to_owned(&self) -> Self::Owned {
        XPathBuf(self.0.to_string())
    }
}

impl From<&str> for XPathBuf {
    fn from(path: &str) -> Self {
        Self(path.to_string())
    }
}

impl From<&XPath> for XPathBuf {
    fn from(path: &XPath) -> Self {
        Self::from(&path.0)
    }
}

impl From<String> for XPathBuf {
    fn from(path: String) -> Self {
        Self(path)
    }
}

impl From<XPathBuf> for Cow<'_, XPath> {
    fn from(path: XPathBuf) -> Self {
        Cow::Owned(path)
    }
}

impl PartialEq for XPath {
    #[inline]
    fn eq(&self, other: &XPath) -> bool {
        Path::new(&self.0) == Path::new(&other.0)
    }
}

impl PartialOrd for XPath {
    #[inline]
    fn partial_cmp(&self, other: &XPath) -> Option<cmp::Ordering> {
        Path::new(&self.0).partial_cmp(Path::new(&other.0))
    }
}
