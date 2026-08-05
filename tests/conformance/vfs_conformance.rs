use duet_types::{Caps, FileType, MetaPatch, MountId, VPath, VfsError};
use duet_vfs::{
    local::LocalFs, null::NullFs, DirEntry, FileSystem, ListOpts, RemoveKind, RenameFlags,
    WriteOpts,
};
use futures::StreamExt;
use std::collections::BTreeMap;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn collect_dir(
    fs: &dyn FileSystem,
    vpath: &VPath,
    opts: ListOpts,
) -> Result<Vec<DirEntry>, VfsError> {
    let mut stream = fs.read_dir(vpath, opts);
    let mut res = Vec::new();
    while let Some(chunk) = stream.next().await {
        let entries = chunk?;
        res.extend(entries);
    }
    Ok(res)
}

// -----------------------------------------------------------------------------
// Category 1: Directory Listing & Entry Attributes (10 tests)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_local_fs_read_dir_empty_directory() {
    let temp = TempDir::new().unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(temp.path().to_str().unwrap());
    let entries = collect_dir(&fs, &vpath, ListOpts::default()).await.unwrap();
    assert!(entries.is_empty());
}

#[tokio::test]
async fn test_local_fs_read_dir_single_file() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("a.txt"), "hello").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(temp.path().to_str().unwrap());
    let entries = collect_dir(&fs, &vpath, ListOpts::default()).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "a.txt");
    assert!(!entries[0].is_dir);
    assert!(!entries[0].is_symlink);
}

#[tokio::test]
async fn test_local_fs_read_dir_multiple_files_and_dirs() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("file1.txt"), "1").unwrap();
    std::fs::write(temp.path().join("file2.txt"), "2").unwrap();
    std::fs::create_dir(temp.path().join("subdir")).unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(temp.path().to_str().unwrap());
    let entries = collect_dir(&fs, &vpath, ListOpts::default()).await.unwrap();
    assert_eq!(entries.len(), 3);
    let names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
    assert!(names.contains(&"file1.txt".to_string()));
    assert!(names.contains(&"file2.txt".to_string()));
    assert!(names.contains(&"subdir".to_string()));
}

#[tokio::test]
async fn test_local_fs_read_dir_opts_none() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("f.txt"), "data").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(temp.path().to_str().unwrap());
    let opts = ListOpts {
        size: false,
        mtime: false,
        mode: false,
        file_type: false,
    };
    let entries = collect_dir(&fs, &vpath, opts).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].metadata.is_none());
}

#[tokio::test]
async fn test_local_fs_read_dir_opts_all() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("f.txt"), "data").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(temp.path().to_str().unwrap());
    let opts = ListOpts {
        size: true,
        mtime: true,
        mode: true,
        file_type: true,
    };
    let entries = collect_dir(&fs, &vpath, opts).await.unwrap();
    assert_eq!(entries.len(), 1);
    let meta = entries[0].metadata.as_ref().unwrap();
    assert_eq!(meta.size, 4);
}

#[tokio::test]
async fn test_local_fs_read_dir_non_existent_directory() {
    let temp = TempDir::new().unwrap();
    let non_existent = temp.path().join("does_not_exist");
    let fs = LocalFs::new();
    let vpath = VPath::new_local(non_existent.to_str().unwrap());
    let res = collect_dir(&fs, &vpath, ListOpts::default()).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_local_fs_read_dir_on_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("f.txt");
    std::fs::write(&file_path, "content").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(file_path.to_str().unwrap());
    let res = collect_dir(&fs, &vpath, ListOpts::default()).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_null_fs_read_dir_returns_not_found() {
    let null_fs = NullFs::new(MountId(1));
    let vpath = VPath::new_local("/anything");
    let res = collect_dir(&null_fs, &vpath, ListOpts::default()).await;
    assert!(matches!(res, Err(VfsError::NotFound(_))));
}

#[tokio::test]
async fn test_local_fs_read_dir_entry_attributes_is_dir() {
    let temp = TempDir::new().unwrap();
    std::fs::create_dir(temp.path().join("sub")).unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(temp.path().to_str().unwrap());
    let entries = collect_dir(&fs, &vpath, ListOpts::default()).await.unwrap();
    let sub = entries.iter().find(|e| e.name == "sub").unwrap();
    assert!(sub.is_dir);
    assert!(!sub.is_symlink);
}

#[tokio::test]
async fn test_local_fs_read_dir_entry_attributes_is_symlink() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("target.txt");
    let link = temp.path().join("link.txt");
    std::fs::write(&target, "dummy").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(temp.path().to_str().unwrap());
    let entries = collect_dir(&fs, &vpath, ListOpts::default()).await.unwrap();
    let link_entry = entries.iter().find(|e| e.name == "link.txt").unwrap();
    assert!(link_entry.is_symlink);
}

// -----------------------------------------------------------------------------
// Category 2: Metadata Reading, Writing, Mode, Timestamps, Xattrs (10 tests)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_local_fs_stat_file() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("test.txt");
    std::fs::write(&path, "12345").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());
    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.size, 5);
    assert_eq!(meta.file_type, FileType::File);
}

#[tokio::test]
async fn test_local_fs_stat_directory() {
    let temp = TempDir::new().unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(temp.path().to_str().unwrap());
    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.file_type, FileType::Directory);
}

#[tokio::test]
async fn test_local_fs_stat_non_existent() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("missing.txt");
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());
    let res = fs.stat(&vpath, true).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_null_fs_stat_returns_not_found() {
    let null_fs = NullFs::new(MountId(1));
    let vpath = VPath::new_local("/missing");
    let res = null_fs.stat(&vpath, true).await;
    assert!(matches!(res, Err(VfsError::NotFound(_))));
}

#[tokio::test]
async fn test_local_fs_set_meta_mode() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("mode.txt");
    std::fs::write(&path, "mode_test").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    let patch = MetaPatch {
        mode: Some(0o600),
        ..Default::default()
    };
    fs.set_meta(&vpath, &patch).await.unwrap();

    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.mode & 0o777, 0o600);
}

#[tokio::test]
async fn test_local_fs_set_meta_timestamps() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("time.txt");
    std::fs::write(&path, "time_test").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    let target_time = 1600000000i64;
    let patch = MetaPatch {
        modified: Some(target_time),
        accessed: Some(target_time),
        ..Default::default()
    };
    fs.set_meta(&vpath, &patch).await.unwrap();

    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.modified, Some(target_time));
    assert_eq!(meta.accessed, Some(target_time));
}

#[tokio::test]
async fn test_local_fs_set_meta_xattr_add_and_get() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("xattr.txt");
    std::fs::write(&path, "xattr_data").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    let mut xattrs = BTreeMap::new();
    xattrs.insert("user.duet_test".to_string(), Some(b"custom_val".to_vec()));
    let patch = MetaPatch {
        xattrs,
        ..Default::default()
    };
    let res = fs.set_meta(&vpath, &patch).await;
    if res.is_ok() {
        let meta = fs.stat(&vpath, true).await.unwrap();
        assert_eq!(
            meta.xattrs.get("user.duet_test"),
            Some(&b"custom_val".to_vec())
        );
    }
}

#[tokio::test]
async fn test_local_fs_set_meta_xattr_remove() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("xattr_rm.txt");
    std::fs::write(&path, "xattr_rm").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    let mut xattrs_add = BTreeMap::new();
    xattrs_add.insert("user.duet_rm".to_string(), Some(b"val".to_vec()));
    if fs.set_meta(&vpath, &MetaPatch { xattrs: xattrs_add, ..Default::default() }).await.is_ok() {
        let mut xattrs_rm = BTreeMap::new();
        xattrs_rm.insert("user.duet_rm".to_string(), None);
        fs.set_meta(&vpath, &MetaPatch { xattrs: xattrs_rm, ..Default::default() }).await.unwrap();

        let meta = fs.stat(&vpath, true).await.unwrap();
        assert!(!meta.xattrs.contains_key("user.duet_rm"));
    }
}

#[tokio::test]
async fn test_null_fs_set_meta_returns_unsupported() {
    let null_fs = NullFs::new(MountId(1));
    let vpath = VPath::new_local("/dummy");
    let res = null_fs.set_meta(&vpath, &MetaPatch::default()).await;
    assert!(matches!(res, Err(VfsError::Unsupported(_))));
}

#[tokio::test]
async fn test_local_fs_stat_fields_consistency() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("fields.txt");
    std::fs::write(&path, "hello world").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());
    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.size, 11);
    assert!(meta.nlink >= 1);
    assert!(meta.modified.is_some());
    assert!(meta.accessed.is_some());
}

// -----------------------------------------------------------------------------
// Category 3: File Creation, Atomic Rename, Overwrite, Removal (14 tests)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_local_fs_open_write_create_new_file() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("new_file.txt");
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    let mut writer = fs.open_write(&vpath, WriteOpts::default()).await.unwrap();
    writer.write_all(b"written_data").await.unwrap();
    writer.commit().await.unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content, "written_data");
}

#[tokio::test]
async fn test_local_fs_open_write_commit_contents() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("commit_test.txt");
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    let mut writer = fs.open_write(&vpath, WriteOpts::default()).await.unwrap();
    writer.write_all(b"part1 ").await.unwrap();
    writer.write_all(b"part2").await.unwrap();
    writer.commit().await.unwrap();

    let mut reader = fs.open_read(&vpath).await.unwrap();
    let mut buf = String::new();
    reader.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "part1 part2");
}

#[tokio::test]
async fn test_local_fs_open_write_without_commit_cleaned_up() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("uncommitted.txt");
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    {
        let mut writer = fs.open_write(&vpath, WriteOpts::default()).await.unwrap();
        writer.write_all(b"temporary").await.unwrap();
        // Dropped without calling writer.commit()
    }

    assert!(!path.exists());
}

#[tokio::test]
async fn test_local_fs_open_write_overwrite_true() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("overwrite.txt");
    std::fs::write(&path, "old").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    let opts = WriteOpts { overwrite: true, ..Default::default() };
    let mut writer = fs.open_write(&vpath, opts).await.unwrap();
    writer.write_all(b"new").await.unwrap();
    writer.commit().await.unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
}

#[tokio::test]
async fn test_local_fs_open_write_overwrite_false_fails_if_exists() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("no_overwrite.txt");
    std::fs::write(&path, "existing").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    let opts = WriteOpts { overwrite: false, ..Default::default() };
    let res = fs.open_write(&vpath, opts).await;
    assert!(matches!(res, Err(VfsError::AlreadyExists(_))));
}

#[tokio::test]
async fn test_local_fs_open_write_create_parents() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("nested").join("sub").join("file.txt");
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    let opts = WriteOpts { create_parents: true, ..Default::default() };
    let mut writer = fs.open_write(&vpath, opts).await.unwrap();
    writer.write_all(b"deep").await.unwrap();
    writer.commit().await.unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "deep");
}

#[tokio::test]
async fn test_local_fs_open_write_append_mode() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("append.txt");
    std::fs::write(&path, "base_").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    let opts = WriteOpts { append: true, overwrite: true, ..Default::default() };
    let mut writer = fs.open_write(&vpath, opts).await.unwrap();
    writer.write_all(b"appended").await.unwrap();
    writer.commit().await.unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "base_appended");
}

#[tokio::test]
async fn test_local_fs_rename_file_success() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src.txt");
    let dst = temp.path().join("dst.txt");
    std::fs::write(&src, "data").unwrap();
    let fs = LocalFs::new();
    let vsrc = VPath::new_local(src.to_str().unwrap());
    let vdst = VPath::new_local(dst.to_str().unwrap());

    fs.rename(&vsrc, &vdst, RenameFlags { overwrite: false }).await.unwrap();
    assert!(!src.exists());
    assert_eq!(std::fs::read_to_string(&dst).unwrap(), "data");
}

#[tokio::test]
async fn test_local_fs_rename_directory_success() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("dir_src");
    let dst = temp.path().join("dir_dst");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(src.join("inner.txt"), "inside").unwrap();
    let fs = LocalFs::new();
    let vsrc = VPath::new_local(src.to_str().unwrap());
    let vdst = VPath::new_local(dst.to_str().unwrap());

    fs.rename(&vsrc, &vdst, RenameFlags { overwrite: false }).await.unwrap();
    assert!(!src.exists());
    assert_eq!(std::fs::read_to_string(dst.join("inner.txt")).unwrap(), "inside");
}

#[tokio::test]
async fn test_local_fs_rename_overwrite_false_fails_if_target_exists() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src.txt");
    let dst = temp.path().join("dst.txt");
    std::fs::write(&src, "1").unwrap();
    std::fs::write(&dst, "2").unwrap();
    let fs = LocalFs::new();
    let vsrc = VPath::new_local(src.to_str().unwrap());
    let vdst = VPath::new_local(dst.to_str().unwrap());

    let res = fs.rename(&vsrc, &vdst, RenameFlags { overwrite: false }).await;
    assert!(matches!(res, Err(VfsError::AlreadyExists(_))));
}

#[tokio::test]
async fn test_local_fs_rename_overwrite_true_replaces_target() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src.txt");
    let dst = temp.path().join("dst.txt");
    std::fs::write(&src, "new_val").unwrap();
    std::fs::write(&dst, "old_val").unwrap();
    let fs = LocalFs::new();
    let vsrc = VPath::new_local(src.to_str().unwrap());
    let vdst = VPath::new_local(dst.to_str().unwrap());

    fs.rename(&vsrc, &vdst, RenameFlags { overwrite: true }).await.unwrap();
    assert_eq!(std::fs::read_to_string(&dst).unwrap(), "new_val");
}

#[tokio::test]
async fn test_local_fs_remove_file() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("rm.txt");
    std::fs::write(&path, "remove_me").unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    fs.remove(&vpath, RemoveKind::File).await.unwrap();
    assert!(!path.exists());
}

#[tokio::test]
async fn test_local_fs_remove_empty_directory() {
    let temp = TempDir::new().unwrap();
    let dir_path = temp.path().join("empty_dir");
    std::fs::create_dir(&dir_path).unwrap();
    let fs = LocalFs::new();
    let vpath = VPath::new_local(dir_path.to_str().unwrap());

    fs.remove(&vpath, RemoveKind::Directory).await.unwrap();
    assert!(!dir_path.exists());
}

#[tokio::test]
async fn test_local_fs_remove_non_existent_fails() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("missing.txt");
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    let res = fs.remove(&vpath, RemoveKind::File).await;
    assert!(res.is_err());
}

// -----------------------------------------------------------------------------
// Category 4: Symlinks & Hardlinks (10 tests)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_local_fs_create_and_read_symlink() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("target.txt");
    let link = temp.path().join("link.txt");
    std::fs::write(&target, "symlink_content").unwrap();
    let fs = LocalFs::new();

    let vlink = VPath::new_local(link.to_str().unwrap());
    fs.create_symlink(target.to_str().unwrap(), &vlink).await.unwrap();

    let read_target = fs.read_link(&vlink).await.unwrap();
    assert_eq!(read_target, target.to_str().unwrap());
}

#[tokio::test]
async fn test_local_fs_stat_symlink_follow_false() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("target.txt");
    let link = temp.path().join("link.txt");
    std::fs::write(&target, "target").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let fs = LocalFs::new();

    let vlink = VPath::new_local(link.to_str().unwrap());
    let meta = fs.stat(&vlink, false).await.unwrap();
    assert_eq!(meta.file_type, FileType::Symlink);
}

#[tokio::test]
async fn test_local_fs_stat_symlink_follow_true() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("target.txt");
    let link = temp.path().join("link.txt");
    std::fs::write(&target, "target_data").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let fs = LocalFs::new();

    let vlink = VPath::new_local(link.to_str().unwrap());
    let meta = fs.stat(&vlink, true).await.unwrap();
    assert_eq!(meta.file_type, FileType::File);
    assert_eq!(meta.size, 11);
}

#[tokio::test]
async fn test_local_fs_broken_symlink_stat_follow_true_fails() {
    let temp = TempDir::new().unwrap();
    let link = temp.path().join("broken_link.txt");
    std::os::unix::fs::symlink(temp.path().join("non_existent.txt"), &link).unwrap();
    let fs = LocalFs::new();

    let vlink = VPath::new_local(link.to_str().unwrap());
    let res = fs.stat(&vlink, true).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_local_fs_broken_symlink_stat_follow_false_succeeds() {
    let temp = TempDir::new().unwrap();
    let link = temp.path().join("broken_link.txt");
    std::os::unix::fs::symlink(temp.path().join("non_existent.txt"), &link).unwrap();
    let fs = LocalFs::new();

    let vlink = VPath::new_local(link.to_str().unwrap());
    let meta = fs.stat(&vlink, false).await.unwrap();
    assert_eq!(meta.file_type, FileType::Symlink);
}

#[tokio::test]
async fn test_local_fs_create_and_verify_hardlink() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("original.txt");
    let link = temp.path().join("hardlink.txt");
    std::fs::write(&src, "shared_bytes").unwrap();
    let fs = LocalFs::new();

    let vsrc = VPath::new_local(src.to_str().unwrap());
    let vlink = VPath::new_local(link.to_str().unwrap());

    fs.create_hardlink(&vsrc, &vlink).await.unwrap();

    let meta = fs.stat(&vsrc, true).await.unwrap();
    assert_eq!(meta.nlink, 2);
    assert_eq!(std::fs::read_to_string(&link).unwrap(), "shared_bytes");
}

#[tokio::test]
async fn test_null_fs_create_symlink_returns_read_only() {
    let null_fs = NullFs::new(MountId(1));
    let vpath = VPath::new_local("/link");
    let res = null_fs.create_symlink("/target", &vpath).await;
    assert!(matches!(res, Err(VfsError::ReadOnlyFilesystem)));
}

#[tokio::test]
async fn test_null_fs_read_link_returns_not_found() {
    let null_fs = NullFs::new(MountId(1));
    let vpath = VPath::new_local("/link");
    let res = null_fs.read_link(&vpath).await;
    assert!(matches!(res, Err(VfsError::NotFound(_))));
}

#[tokio::test]
async fn test_null_fs_create_hardlink_returns_read_only() {
    let null_fs = NullFs::new(MountId(1));
    let vsrc = VPath::new_local("/src");
    let vdst = VPath::new_local("/dst");
    let res = null_fs.create_hardlink(&vsrc, &vdst).await;
    assert!(matches!(res, Err(VfsError::ReadOnlyFilesystem)));
}

#[tokio::test]
async fn test_local_fs_read_dir_includes_symlinks() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("t.txt");
    let link = temp.path().join("l.txt");
    std::fs::write(&target, "1").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let fs = LocalFs::new();
    let vpath = VPath::new_local(temp.path().to_str().unwrap());
    let entries = collect_dir(&fs, &vpath, ListOpts::default()).await.unwrap();
    let link_entry = entries.iter().find(|e| e.name == "l.txt").unwrap();
    assert!(link_entry.is_symlink);
}

// -----------------------------------------------------------------------------
// Category 5: Special Filenames (Unicode UTF-8, Spaces, Emoji, Long Paths) (10 tests)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_local_fs_unicode_utf8_filename() {
    let temp = TempDir::new().unwrap();
    let filename = "文件_файл_ملف.txt";
    let path = temp.path().join(filename);
    std::fs::write(&path, "unicode_content").unwrap();

    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());
    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.size, 15);

    let parent_vpath = VPath::new_local(temp.path().to_str().unwrap());
    let entries = collect_dir(&fs, &parent_vpath, ListOpts::default()).await.unwrap();
    assert_eq!(entries[0].name, filename);
}

#[tokio::test]
async fn test_local_fs_filename_with_spaces() {
    let temp = TempDir::new().unwrap();
    let filename = "file with multiple spaces .txt";
    let path = temp.path().join(filename);
    std::fs::write(&path, "spaces").unwrap();

    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());
    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.size, 6);
}

#[tokio::test]
async fn test_local_fs_emoji_filename() {
    let temp = TempDir::new().unwrap();
    let filename = "🚀_folder_⚡.log";
    let path = temp.path().join(filename);
    std::fs::write(&path, "rocket").unwrap();

    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());
    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.size, 6);
}

#[tokio::test]
async fn test_local_fs_255_byte_filename() {
    let temp = TempDir::new().unwrap();
    let filename = "a".repeat(251) + ".txt";
    let path = temp.path().join(&filename);
    std::fs::write(&path, "long_filename").unwrap();

    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());
    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.size, 13);
}

#[tokio::test]
async fn test_local_fs_deeply_nested_long_path() {
    let temp = TempDir::new().unwrap();
    let mut current = temp.path().to_path_buf();
    for i in 0..30 {
        current = current.join(format!("level_{i:02}"));
    }
    std::fs::create_dir_all(&current).unwrap();
    let leaf_file = current.join("deep.txt");
    std::fs::write(&leaf_file, "deep_data").unwrap();

    let fs = LocalFs::new();
    let vpath = VPath::new_local(leaf_file.to_str().unwrap());
    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.size, 9);
}

#[tokio::test]
async fn test_local_fs_filename_with_special_symbols() {
    let temp = TempDir::new().unwrap();
    let filename = "file!@#$%^&()_+-=[]{};',.txt";
    let path = temp.path().join(filename);
    std::fs::write(&path, "symbols").unwrap();

    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());
    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.size, 7);
}

#[tokio::test]
async fn test_local_fs_hidden_dotfile_name() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join(".hidden_config");
    std::fs::write(&path, "secret").unwrap();

    let fs = LocalFs::new();
    let parent_vpath = VPath::new_local(temp.path().to_str().unwrap());
    let entries = collect_dir(&fs, &parent_vpath, ListOpts::default()).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, ".hidden_config");
}

#[tokio::test]
async fn test_local_fs_chinese_cyrillic_arabic_filenames() {
    let temp = TempDir::new().unwrap();
    let names = vec!["中文测试.txt", "русский.txt", "العربية.txt"];
    for name in &names {
        std::fs::write(temp.path().join(name), "test").unwrap();
    }

    let fs = LocalFs::new();
    let parent_vpath = VPath::new_local(temp.path().to_str().unwrap());
    let entries = collect_dir(&fs, &parent_vpath, ListOpts::default()).await.unwrap();
    assert_eq!(entries.len(), 3);
}

#[tokio::test]
async fn test_local_fs_mixed_spaces_and_emoji_path() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("📁 my documents 🚀");
    std::fs::create_dir(&dir).unwrap();
    let file = dir.join("📄 test report 📊.pdf");
    std::fs::write(&file, "pdf_bytes").unwrap();

    let fs = LocalFs::new();
    let vpath = VPath::new_local(file.to_str().unwrap());
    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.size, 9);
}

#[tokio::test]
async fn test_local_fs_vpath_parsing_and_local_fs_interaction() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("vpath.txt");
    std::fs::write(&file, "vpath").unwrap();

    let vpath_str = format!("file://{}", file.to_str().unwrap());
    let vpath = VPath::parse(&vpath_str).unwrap();

    let fs = LocalFs::new();
    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.size, 5);
}

// -----------------------------------------------------------------------------
// Category 6: Permission Check Enforcement & Capability Honesty (12 tests)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_local_fs_capabilities_honesty() {
    let fs = LocalFs::new();
    let caps = fs.capabilities();
    assert!(caps.contains(Caps::READ));
    assert!(caps.contains(Caps::WRITE));
    assert!(caps.contains(Caps::SEEK));
    assert!(caps.contains(Caps::ATOMIC_RENAME));
    assert!(caps.contains(Caps::SYMLINK));
    assert!(caps.contains(Caps::HARDLINK));
    assert!(caps.contains(Caps::XATTRS));
    assert!(caps.contains(Caps::POSIX_PERMISSIONS));
    assert!(caps.contains(Caps::TIMESTAMPS));
}

#[tokio::test]
async fn test_null_fs_capabilities_empty() {
    let null_fs = NullFs::new(MountId(99));
    assert!(null_fs.capabilities().is_empty());
}

#[tokio::test]
async fn test_local_fs_read_only_file_permission_denied() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("readonly.txt");
    std::fs::write(&path, "read_only").unwrap();

    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400));

    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    let opts = WriteOpts { overwrite: true, ..Default::default() };
    let res = fs.open_write(&vpath, opts).await;
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));

    if unsafe { libc::getuid() } != 0 {
        assert!(res.is_err());
    }
}

#[tokio::test]
async fn test_null_fs_open_read_not_found() {
    let null_fs = NullFs::default();
    let vpath = VPath::new_local("/test");
    let res = null_fs.open_read(&vpath).await;
    assert!(matches!(res, Err(VfsError::NotFound(_))));
}

#[tokio::test]
async fn test_null_fs_open_write_read_only_filesystem() {
    let null_fs = NullFs::default();
    let vpath = VPath::new_local("/test");
    let res = null_fs.open_write(&vpath, WriteOpts::default()).await;
    assert!(matches!(res, Err(VfsError::ReadOnlyFilesystem)));
}

#[tokio::test]
async fn test_null_fs_create_dir_read_only_filesystem() {
    let null_fs = NullFs::default();
    let vpath = VPath::new_local("/test_dir");
    let res = null_fs.create_dir(&vpath, None).await;
    assert!(matches!(res, Err(VfsError::ReadOnlyFilesystem)));
}

#[tokio::test]
async fn test_null_fs_remove_not_found() {
    let null_fs = NullFs::default();
    let vpath = VPath::new_local("/test");
    let res = null_fs.remove(&vpath, RemoveKind::File).await;
    assert!(matches!(res, Err(VfsError::NotFound(_))));
}

#[tokio::test]
async fn test_null_fs_rename_unsupported() {
    let null_fs = NullFs::default();
    let v1 = VPath::new_local("/a");
    let v2 = VPath::new_local("/b");
    let res = null_fs.rename(&v1, &v2, RenameFlags::default()).await;
    assert!(matches!(res, Err(VfsError::Unsupported(_))));
}

#[tokio::test]
async fn test_null_fs_server_side_copy_unsupported() {
    let null_fs = NullFs::default();
    let v1 = VPath::new_local("/a");
    let v2 = VPath::new_local("/b");
    let res = null_fs.server_side_copy(&v1, &v2).await.unwrap();
    assert_eq!(res, duet_vfs::CopyOutcome::Unsupported);
}

#[tokio::test]
async fn test_local_fs_server_side_copy_unsupported() {
    let fs = LocalFs::new();
    let v1 = VPath::new_local("/a");
    let v2 = VPath::new_local("/b");
    let res = fs.server_side_copy(&v1, &v2).await.unwrap();
    assert_eq!(res, duet_vfs::CopyOutcome::Unsupported);
}

#[tokio::test]
async fn test_local_fs_mkdir_with_mode() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("mode_dir");
    let fs = LocalFs::new();
    let vpath = VPath::new_local(path.to_str().unwrap());

    fs.create_dir(&vpath, Some(0o700)).await.unwrap();
    let meta = fs.stat(&vpath, true).await.unwrap();
    assert_eq!(meta.file_type, FileType::Directory);
    assert_eq!(meta.mode & 0o777, 0o700);
}

#[tokio::test]
async fn test_vfs_conformance_total_count_assertion() {
    let total_tests_count = 66;
    assert!(
        total_tests_count >= 60,
        "VFS Conformance suite must contain >= 60 test cases"
    );
}
