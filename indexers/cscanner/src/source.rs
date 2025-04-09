use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{PathBuf, Path};
use std::env::{set_current_dir, current_dir};
use std::sync::Mutex;

use nix::unistd::{Uid, Gid, setuid, setgid};
use clang::Entity;
use lazy_static::lazy_static;

use territory_core::{
    AbsolutePath,
    Location,
    TokenKind,
};
use crate::ast::TransportID;

// use crate::args::get_debug_cfg;

pub fn is_ignored_entity_kind(k: clang::EntityKind) -> bool {
    match k {
        clang::EntityKind::NotImplemented |
        clang::EntityKind::Namespace |
        clang::EntityKind::LinkageSpec => {
            true
        },
        _ => {
            false
        }
    }
}

pub fn find_root<'tu>(
    mut cur: clang::Entity<'tu>,
    under: clang::Entity<'tu>,
) -> clang::Entity<'tu> {
    let mut result = cur;

    while let Some(parent) = cur.get_lexical_parent() {
        if parent == under { break; }

        cur = parent;
        if is_ignored_entity_kind(cur.get_kind()) {
            continue;
        } else {
            result = cur;
        }
    }

    result
}

pub fn cur_path(cur: &clang::Entity) -> Option<AbsolutePath> {
    let loc = cur.get_range()?.get_start().get_spelling_location();
    clang_loc_path(&loc)
}

pub fn clang_loc_path(loc: &clang::source::Location) -> Option<AbsolutePath> {
    Some(clang_file_path(&loc.file?))
}

pub fn clang_file_path(file: &clang::source::File) -> AbsolutePath {
    normalize_path(&file.get_path()).into()
}

pub fn from_clang_token_kind(t: clang::token::TokenKind) -> TokenKind {
    match t {
        clang::token::TokenKind::Comment => TokenKind::Comment,
        clang::token::TokenKind::Identifier => TokenKind::Identifier,
        clang::token::TokenKind::Keyword => TokenKind::Keyword,
        clang::token::TokenKind::Literal => TokenKind::Literal,
        clang::token::TokenKind::Punctuation => TokenKind::Punctuation,
    }
}


pub fn from_clang_location(l: &clang::source::Location) -> Location {
    Location { line: l.line, col: l.column, off: l.offset }
}

pub trait RangeLocations {
    fn start(&self, expected_file: &clang::source::File) -> Option<Location>;
    fn end(&self, expected_path: &clang::source::File) -> Option<Location>;
}

impl RangeLocations for clang::token::Token<'_> {
    /// if token starts in the expected file, return the start location
    fn start(&self, expected_file: &clang::source::File) -> Option<Location> {
        let tok_rng = self.get_range();
        let tok_start = tok_rng.get_start().get_spelling_location();
        let file = tok_start.file?;
        if file != *expected_file { return None; }
        Some(from_clang_location(&tok_start))
    }

    /// if token ends in the expected file, return the end location
    fn end(&self, expected_file: &clang::source::File) -> Option<Location> {
        let tok_rng = self.get_range();
        let tok_end = tok_rng.get_end().get_spelling_location();
        let file = tok_end.file?;
        if file != *expected_file { return None; }
        Some(from_clang_location(&tok_end))
    }
}

impl RangeLocations for clang::Entity<'_> {
    /// if cursor starts in the expected file, return the start location
    fn start(&self, expected_file: &clang::source::File) -> Option<Location> {
        let rng = self.get_range()?;
        let start = rng.get_start().get_spelling_location();
        let file = start.file?;
        if file != *expected_file { return None; }
        Some(from_clang_location(&start))
    }

    /// if cursor ends in the expected file, return the end location
    fn end(&self, expected_file: &clang::source::File) -> Option<Location> {
        let rng = self.get_range()?;
        let end = rng.get_end().get_spelling_location();
        let file = end.file?;
        if file != *expected_file { return None; }
        Some(from_clang_location(&end))
    }
}

pub fn curloc(_repo_path: &Path, cur: &clang::Entity) -> String {
    let path = cur_path(cur)
        .map(|p| p.to_string())
        .unwrap_or("???".to_string());

    format!(
        "{}:{} \"{}\"",
        path,
        cur.get_location()
            .map(|l| l.get_spelling_location().line.to_string())
            .unwrap_or("???".to_string()),
        cur.get_display_name().unwrap_or("???".to_string()))
}

lazy_static! {
    pub static ref CANON_PATH_CACHE: Mutex<HashMap<PathBuf, PathBuf>> = Mutex::new(HashMap::new());
    pub static ref WOKRDIR: Mutex<Option<PathBuf>> = Mutex::new(None);
}


pub fn normalize_path(path: &Path) -> PathBuf {
    let mut cache = CANON_PATH_CACHE.lock().unwrap();

    let mut wd_lock = WOKRDIR.lock().unwrap();
    let workdir = wd_lock.get_or_insert_with(|| current_dir().unwrap());

    if let Some(value) = cache.get(path) {
        value.clone()
    } else {
        let key = workdir.join(path);
        let Ok(value) = key.canonicalize() else {
            println!("failed to canonicalize: {:?} (workdir {:?}", key, workdir);
            return key;
        };

        cache.insert(key, value.clone());
        value
    }
}

pub fn cur_hash(cur: &Entity) -> TransportID {
    let mut hasher = DefaultHasher::new();
    cur.hash(&mut hasher);
    hasher.finish()
}

// pub fn dump_cur(dent: &str, e: clang::Entity) {
//     let rs = match e.get_range() {
//         Some(rng) => {
//             let start = rng.get_start().get_spelling_location();
//             let end = rng.get_end().get_spelling_location();
//             format!("{}:{} - {}:{}", start.line, start.column, end.line, end.column)
//         }
//         None      => "???".to_string()
//     };

//     println!("{}[{}] {:?} {}", dent, rs, e.get_kind(), e.get_name().unwrap_or("???".to_string()));
//     if get_debug_cfg().definition_links {
//         if let Some(rfr) = e.get_reference() {
//             println!("{}        --> {} {:?}", dent, curloc(&current_dir().unwrap(), &rfr), rfr.get_kind());
//         }
//         if let Some(def) = e.get_definition() {
//             println!("{}        DEF: {} {:?}", dent, curloc(&current_dir().unwrap(), &def), def.get_kind());
//         }
//     }
//     if get_debug_cfg().types {
//         if let Some(typ) = e.get_type() {
//             let tds = if let Some(td) = typ.get_declaration() {
//                 format!(" declared at {}", curloc(&current_dir().unwrap(), &td))
//             } else {
//                 "".to_string()
//             };
//             println!("{}        TYP: {}{}", dent, typ.get_display_name(), tds);
//         }
//     }
//     if get_debug_cfg().usrs {
//         if let Some(clang::Usr(usr)) = e.get_usr() {
//             println!("{}        USR: {}", dent, usr);
//         }
//     }
// }

// #[allow(dead_code)]
// pub fn dump_tree(dent: &str, e: clang::Entity) {
//     let mut dent = dent.to_string();

//     fn go(dent: &mut String, e: clang::Entity) {
//         dump_cur(dent, e);

//         dent.push_str("    ");
//         for c in e.get_children() {
//             go(dent, c);
//         }
//         dent.truncate(dent.len() - 4);
//     }

//     go(&mut dent, e);
// }

pub fn set_process_root(
    chroot: &Option<PathBuf>,
) {
    if let Some(chroot_path) = chroot {
        std::os::unix::fs::chroot(chroot_path)
            .expect(&format!("failed to chroot to {:?}", chroot_path));
        std::env::set_current_dir("/")
            .expect(&format!("failed to cd to / (chrooted in {:?})", chroot_path));
    }

}

pub fn set_process_identity(
    setuid_: Option<u32>,
    setgid_: Option<u32>,
) {
    if let Some(gid) = setgid_ {
        setgid(Gid::from_raw(gid)).expect(&format!("failed to set group ID {}", gid));
    }
    if let Some(uid) = setuid_ {
        setuid(Uid::from_raw(uid)).expect(&format!("failed to set user ID {}", uid));
    }
}

static CLANG: Mutex<()> = Mutex::new(());  // Clang can not be instantiated in multiple threads

pub fn with_clang<R>(
    workdir: &PathBuf,
    chroot: &Option<PathBuf>,
    setuid_: Option<u32>,
    setgid_: Option<u32>,
    f: impl FnOnce(clang::Index) -> R
) -> R {
    let _lock = CLANG.lock().unwrap();
    let clang = clang::Clang::new().unwrap();

    set_process_root(chroot);

    let original_directory = current_dir().unwrap();
    set_current_dir(workdir).expect(&format!("nonexistent workdir: {}", workdir.to_string_lossy()));

    let idx = clang::Index::new(&clang, false, false);

    set_process_identity(setuid_, setgid_);

    let r = f(idx);

    set_current_dir(original_directory).unwrap();  // don't break tests

    r
}

pub fn parse<'tu>(
    idx: &'tu clang::Index,
    path: &Path,
    args: &[String],
) -> Result<clang::TranslationUnit<'tu>, clang::SourceError> {
    let mut p = idx.parser(path);
    p.detailed_preprocessing_record(true);
    // p.retain_excluded_conditional_blocks(true);
    p.arguments(args);
    p.parse()
}


#[cfg(test)]
mod test {
    use std::os::unix::fs::symlink;
    use std::fs::{create_dir, write};
    use std::path::PathBuf;

    use testdir::testdir;

    use super::normalize_path;

    #[test]
    fn normalize_path_with_symlinks() {
        let d = testdir!().canonicalize().unwrap();
        let src = d.join("src");
        create_dir(&src).unwrap();
        let src_file = src.join("x.c");
        write(&src_file, "").unwrap();

        let lnk = d.join("lnk");
        symlink(&src, &lnk).unwrap();

        assert_eq!(normalize_path(&lnk.join("x.c")), src_file);
    }

    #[test]
    fn normalize_path_with_excessive_parent_elements() {
        let d = PathBuf::from("/../usr");

        assert_eq!(normalize_path(&d), PathBuf::from("/usr"));
    }
}
