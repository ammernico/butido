use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Error;
use anyhow::anyhow;
use anyhow::Result;
use pom::*;
use pom::parser::Parser as PomParser;

use crate::package::PackageName;
use crate::package::PackageVersion;

pub struct Artifact {
    path: PathBuf,

    name: PackageName,
    version: PackageVersion,
}

impl Artifact {
    pub fn load(path: &Path) -> Result<Self> {
        if path.is_file() {
            let (name, version) = Self::parse_path(path)?;

            Ok(Artifact {
                path: path.to_path_buf(),

                name,
                version
            })
        } else {
            Err(anyhow!("Path does not exist: {}", path.display()))
        }
    }

    fn parse_path(path: &Path) -> Result<(PackageName, PackageVersion)> {
        path.file_name()
            .ok_or_else(|| anyhow!("Cannot get filename from {}", path.display()))?
            .to_owned()
            .into_string()
            .map_err(|_| anyhow!("Internal conversion of '{}' to UTF-8", path.display()))
            .and_then(|s| Self::parser().parse(s.as_bytes()).map_err(Error::from))
    }

    /// Construct a parser that parses a Vec<u8> into (PackageName, PackageVersion)
    fn parser<'a>() -> PomParser<'a, u8, (PackageName, PackageVersion)> {
        use pom::parser::*;
        use pom::char_class::hex_digit;

        let numbers = || one_of(b"0123456789").repeat(1..);
        let letters = || pom::parser::is_a(pom::char_class::alpha).repeat(1..);
        let dash    = || sym(b'-').map(|b| vec![b]);
        let under   = || sym(b'_').map(|b| vec![b]);
        let dot     = || sym(b'.').map(|b| vec![b]);

        let package_name = (letters() + ((letters() | numbers()).repeat(0..)))
            .collect()
            .convert(|b| String::from_utf8(b.to_vec()));

        let package_version = (
                numbers() +
                ((dash() | under() | dot() | letters() | numbers()).repeat(0..))
            )
            .collect()
            .convert(|b| String::from_utf8(b.to_vec()));

        (package_name + dash() + package_version)
            .map(|((name, _), version)| (name, version))
            .map(|(name, version)| {
                (PackageName::from(name), PackageVersion::from(version))
            })
    }

    pub fn create(root: &Path, name: PackageName, version: PackageVersion) -> Result<Self> {
        let path = Self::create_path(root, &name, &version)?;
        if !path.exists() {
            Ok(Artifact {
                path,
                name,
                version
            })
        } else {
            Err(anyhow!("Path exists: {}", path.display()))
        }
    }

    fn create_path(root: &Path, name: &PackageName, version: &PackageVersion) -> Result<PathBuf> {
        if !root.is_dir() {
            return Err(anyhow!("Cannot create file path for {}-{} when root is file path: {}",
                    name, version, root.display()))
        }

        Ok(root.join(format!("{}-{}", name, version)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::tests::pname;
    use crate::package::tests::pversion;

    #[test]
    fn test_parser_one_letter_name() {
        let p = PathBuf::from("a-1");
        let r = Artifact::parse_path(&p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("a"));
        assert_eq!(version, pversion("1"));
    }

    #[test]
    fn test_parser_multi_letter_name() {
        let p = PathBuf::from("foo-1");
        let r = Artifact::parse_path(&p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("foo"));
        assert_eq!(version, pversion("1"));
    }

    #[test]
    fn test_parser_multi_char_version() {
        let p = PathBuf::from("foo-1123");
        let r = Artifact::parse_path(&p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("foo"));
        assert_eq!(version, pversion("1123"));
    }

    #[test]
    fn test_parser_multi_char_version_dashed() {
        let p = PathBuf::from("foo-1-1-2-3");
        let r = Artifact::parse_path(&p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("foo"));
        assert_eq!(version, pversion("1-1-2-3"));
    }

    #[test]
    fn test_parser_multi_char_version_dashed_and_dotted() {
        let p = PathBuf::from("foo-1-1.2-3");
        let r = Artifact::parse_path(&p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("foo"));
        assert_eq!(version, pversion("1-1.2-3"));
    }

    #[test]
    fn test_parser_alnum_version() {
        let p = PathBuf::from("foo-1-1.2a3");
        let r = Artifact::parse_path(&p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("foo"));
        assert_eq!(version, pversion("1-1.2a3"));
    }

    #[test]
    fn test_parser_package_name_with_number() {
        let p = PathBuf::from("foo2-1-1.2a3");
        let r = Artifact::parse_path(&p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("foo2"));
        assert_eq!(version, pversion("1-1.2a3"));
    }
}
