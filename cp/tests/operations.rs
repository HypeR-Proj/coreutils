// SPDX-FileCopyrightText: 2026 roolrz
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn recursive_link_copy_replaces_regular_entry_without_modifying_its_inode() -> io::Result<()> {
    let d = Directory::new()?;
    fs::write(d.path("target"), b"old contents")?;
    fs::hard_link(d.path("target"), d.path("old-alias"))?;
    fs::write(d.path("referent"), b"untouched referent")?;
    symlink("referent", d.path("source"))?;
    let options = CopyOptions {
        recursive: true,
        verbose: true,
        ..CopyOptions::default()
    };
    let mut output = Vec::new();
    copy(&d.path("source"), &d.path("target"), &options, &mut output)?;
    assert_eq!(fs::read_link(d.path("target"))?, Path::new("referent"));
    assert_eq!(fs::read(d.path("old-alias"))?, b"old contents");
    assert_eq!(fs::read(d.path("referent"))?, b"untouched referent");
    assert_eq!(output.iter().filter(|byte| **byte == b'\n').count(), 1);
    Ok(())
}

#[test]
fn recursive_link_overwrite_preserves_skip_and_refusal_policies() -> io::Result<()> {
    let d = Directory::new()?;
    symlink("missing", d.path("source"))?;
    fs::write(d.path("file"), b"keep")?;
    fs::create_dir(d.path("directory"))?;
    fs::write(d.path("directory/child"), b"keep child")?;
    symlink("file", d.path("link"))?;
    symlink("absent", d.path("dangling"))?;
    let mut options = CopyOptions {
        recursive: true,
        no_clobber: true,
        verbose: true,
    };
    let mut output = Vec::new();
    for target in ["file", "directory", "link", "dangling", "source"] {
        copy(&d.path("source"), &d.path(target), &options, &mut output)?;
    }
    assert!(output.is_empty());
    assert_eq!(fs::read(d.path("file"))?, b"keep");
    options.no_clobber = false;
    for target in ["directory", "link", "dangling", "source"] {
        assert!(copy(&d.path("source"), &d.path(target), &options, &mut output).is_err());
    }
    assert_eq!(fs::read(d.path("directory/child"))?, b"keep child");
    assert_eq!(fs::read_link(d.path("link"))?, Path::new("file"));
    assert_eq!(fs::read_link(d.path("dangling"))?, Path::new("absent"));
    assert_eq!(fs::read_link(d.path("source"))?, Path::new("missing"));
    assert_eq!(fs::read(d.path("file"))?, b"keep");
    assert!(!d.path("missing").exists());
    assert!(!d.path("absent").exists());
    copy(&d.path("source"), &d.path("file"), &options, &mut output)?;
    assert_eq!(fs::read_link(d.path("file"))?, Path::new("missing"));
    Ok(())
}

#[test]
fn link_preparation_and_publication_failures_preserve_destination() -> io::Result<()> {
    let d = Directory::new()?;
    fs::write(d.path("file"), b"keep")?;
    fs::create_dir(d.path("directory"))?;
    fs::write(d.path("directory/child"), b"keep child")?;
    let oversized = "x".repeat(65_536);
    assert!(replace_with_link(Path::new(&oversized), &d.path("file")).is_err());
    assert!(replace_with_link(Path::new("missing"), &d.path("directory")).is_err());
    assert_eq!(fs::read(d.path("file"))?, b"keep");
    assert_eq!(fs::read(d.path("directory/child"))?, b"keep child");
    assert_eq!(fs::read_dir(&d.0)?.count(), 2);
    Ok(())
}

#[test]
fn no_clobber_preserves_files_and_dangling_links_and_merges_directories() -> io::Result<()> {
    let d = Directory::new()?;
    fs::create_dir(d.path("source"))?;
    fs::create_dir(d.path("target"))?;
    fs::write(d.path("source/existing"), b"new")?;
    fs::write(d.path("source/link"), b"new")?;
    fs::write(d.path("source/fresh"), b"fresh")?;
    fs::write(d.path("target/existing"), b"keep")?;
    symlink("absent", d.path("target/link"))?;
    let options = CopyOptions {
        recursive: true,
        no_clobber: true,
        verbose: true,
    };
    let mut output = Vec::new();
    copy(&d.path("source"), &d.path("target"), &options, &mut output)?;
    assert_eq!(fs::read(d.path("target/existing"))?, b"keep");
    assert_eq!(fs::read(d.path("target/fresh"))?, b"fresh");
    assert_eq!(fs::read_link(d.path("target/link"))?, Path::new("absent"));
    assert!(!d.path("target/absent").exists());
    assert_eq!(output.iter().filter(|byte| **byte == b'\n').count(), 1);
    // Also exercise the exclusive-create operation itself, bypassing the
    // earlier metadata check as if another creator had won the race.
    assert!(!copy_regular(
        &d.path("source/existing"),
        &d.path("target/existing"),
        fs::metadata(d.path("source/existing"))?.permissions(),
        true
    )?);
    assert_eq!(fs::read(d.path("target/existing"))?, b"keep");
    Ok(())
}

struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> std::io::Result<Self> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("hyper-cp-test-{}-{id}", std::process::id()));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
    fn path(&self, name: &str) -> std::path::PathBuf {
        self.0.join(name)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn copies_data_and_rejects_aliases_without_truncating() -> io::Result<()> {
    let d = Directory::new()?;
    fs::write(d.path("source"), b"keep me")?;
    fs::write(d.path("copy"), b"long old contents")?;
    copy(
        &d.path("source"),
        &d.path("copy"),
        &CopyOptions {
            recursive: false,
            ..CopyOptions::default()
        },
        &mut io::sink(),
    )?;
    assert_eq!(fs::read(d.path("copy"))?, b"keep me");
    fs::hard_link(d.path("source"), d.path("hard"))?;
    symlink("source", d.path("soft"))?;
    for name in ["source", "hard", "soft"] {
        assert!(
            copy(
                &d.path("source"),
                &d.path(name),
                &CopyOptions {
                    recursive: false,
                    ..CopyOptions::default()
                },
                &mut io::sink()
            )
            .is_err()
        );
        assert_eq!(fs::read(d.path("source"))?, b"keep me");
    }
    symlink("absent", d.path("dangling"))?;
    assert!(
        copy(
            &d.path("source"),
            &d.path("dangling"),
            &CopyOptions {
                recursive: false,
                ..CopyOptions::default()
            },
            &mut io::sink()
        )
        .is_err()
    );
    assert!(!d.path("absent").exists());
    Ok(())
}

#[test]
fn recursive_copy_preserves_links_and_rejects_descendants() -> io::Result<()> {
    let d = Directory::new()?;
    fs::create_dir_all(d.path("source/sub"))?;
    fs::write(d.path("source/sub/file"), b"data")?;
    symlink("sub/file", d.path("source/link"))?;
    symlink("missing", d.path("source/dangling"))?;
    assert!(
        copy(
            &d.path("source"),
            &d.path("plain"),
            &CopyOptions {
                recursive: false,
                ..CopyOptions::default()
            },
            &mut io::sink()
        )
        .is_err()
    );
    assert!(
        copy(
            &d.path("source"),
            &d.path("source/new"),
            &CopyOptions {
                recursive: true,
                ..CopyOptions::default()
            },
            &mut io::sink()
        )
        .is_err()
    );
    symlink("source/sub", d.path("alias"))?;
    assert!(
        copy(
            &d.path("source"),
            &d.path("alias/new"),
            &CopyOptions {
                recursive: true,
                ..CopyOptions::default()
            },
            &mut io::sink()
        )
        .is_err()
    );
    copy(
        &d.path("source"),
        &d.path("copy"),
        &CopyOptions {
            recursive: true,
            ..CopyOptions::default()
        },
        &mut io::sink(),
    )?;
    assert_eq!(fs::read(d.path("copy/sub/file"))?, b"data");
    assert_eq!(fs::read_link(d.path("copy/link"))?, Path::new("sub/file"));
    assert_eq!(
        fs::read_link(d.path("copy/dangling"))?,
        Path::new("missing")
    );
    Ok(())
}

#[test]
fn cli_rejects_missing_destination_and_continues_after_errors()
-> Result<(), Box<dyn std::error::Error>> {
    assert!(Args::try_parse_from(["cp", "one"]).is_err());
    let d = Directory::new()?;
    fs::create_dir(d.path("out"))?;
    fs::write(d.path("file"), b"data")?;
    assert!(!run(Args {
        options: CopyOptions {
            recursive: false,
            ..CopyOptions::default()
        },
        paths: vec![d.path("missing"), d.path("file"), d.path("out")],
        ..Args::default()
    }));
    assert_eq!(fs::read(d.path("out/file"))?, b"data");
    Ok(())
}

#[test]
fn trailing_dot_copies_contents_into_existing_directory() -> io::Result<()> {
    let d = Directory::new()?;
    fs::create_dir(d.path("source"))?;
    fs::create_dir(d.path("out"))?;
    fs::write(d.path("source/file"), b"data")?;
    assert!(run(Args {
        options: CopyOptions {
            recursive: true,
            ..CopyOptions::default()
        },
        paths: vec![d.path("source/."), d.path("out")],
        ..Args::default()
    }));
    assert_eq!(fs::read(d.path("out/file"))?, b"data");
    assert!(!d.path("out/source").exists());
    Ok(())
}

#[test]
fn copies_multiple_transfer_windows_and_the_final_tail() -> io::Result<()> {
    let d = Directory::new()?;
    let bytes: Vec<u8> = (0..(2 * 1024 * 1024 + 17))
        .map(|i| (i % 251) as u8)
        .collect();
    fs::write(d.path("source"), &bytes)?;
    assert!(copy_regular(
        &d.path("source"),
        &d.path("copy"),
        fs::metadata(d.path("source"))?.permissions(),
        false
    )?);
    assert_eq!(fs::read(d.path("copy"))?, bytes);
    Ok(())
}
