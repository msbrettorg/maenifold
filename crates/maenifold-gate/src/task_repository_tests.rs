    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;

    use tempfile::tempdir;

    use super::{TaskPathSet, TaskRepository};
    use crate::error::GateError;

    #[test]
    fn list_tasks_rejects_duplicate_canonical_task_ids() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");

        fs::write(
            root.join("todo").join("T-90.25-first.md"),
            "---\nid: T-90.25\ntitle: First duplicate\npipeline: sdd\n---\n# First duplicate\n",
        )
        .expect("write first task");
        fs::write(
            root.join("done").join("T-90.25-second.md"),
            "---\nid: T-90.25\ntitle: Second duplicate\npipeline: sdd\n---\n# Second duplicate\n",
        )
        .expect("write second task");

        let error = TaskRepository::list_tasks(root.to_path_buf()).expect_err("duplicate id");
        match error {
            GateError::DuplicateCanonicalTaskId {
                task_id,
                first_path,
                second_path,
            } => {
                assert_eq!(task_id, "T-90.25");
                assert!(first_path.ends_with("todo/T-90.25-first.md"));
                assert!(second_path.ends_with("done/T-90.25-second.md"));
            }
            other => panic!("expected duplicate canonical task id error, got {other}"),
        }
    }

    #[test]
    fn canonical_task_markdown_path_accepts_task_in_location_corpus() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");

        let task_path = root.join("todo").join("T-1.1.md");
        fs::write(&task_path, "# T-1.1\n").expect("write task");

        let canonical = TaskRepository::canonical_task_markdown_path(
            root,
            &TaskPathSet {
                task_id: "T-1.1".to_string(),
                path: task_path.clone(),
                location: "todo".to_string(),
            },
        )
        .expect("canonical task path");

        assert_eq!(
            canonical,
            fs::canonicalize(task_path).expect("canonicalize task path")
        );
    }

    #[cfg(unix)]
    #[test]
    fn canonical_task_markdown_path_rejects_symlink_escape_outside_root() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");

        let escaped = temp.path().join("escaped.md");
        fs::write(&escaped, "# escaped\n").expect("write escaped file");
        let task_path = root.join("todo").join("T-1.1.md");
        unix_fs::symlink(&escaped, &task_path).expect("create symlink task");

        let error = TaskRepository::canonical_task_markdown_path(
            root,
            &TaskPathSet {
                task_id: "T-1.1".to_string(),
                path: task_path,
                location: "todo".to_string(),
            },
        )
        .expect_err("escaped symlink should be rejected");

        assert!(
            error
                .to_string()
                .contains("resolves outside the gate root corpus")
        );
    }

    #[cfg(unix)]
    #[test]
    fn canonical_task_markdown_path_rejects_symlinked_task_inside_root() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");

        let target = root.join("todo").join("linked-target.md");
        fs::write(&target, "# linked target\n").expect("write linked target");
        let task_path = root.join("todo").join("T-1.1.md");
        unix_fs::symlink(&target, &task_path).expect("create in-root symlink task");

        let error = TaskRepository::canonical_task_markdown_path(
            root,
            &TaskPathSet {
                task_id: "T-1.1".to_string(),
                path: task_path,
                location: "todo".to_string(),
            },
        )
        .expect_err("symlinked task should be rejected");

        assert!(error.to_string().contains("through a symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn list_tasks_does_not_parse_frontmatter_from_symlink_target() {
        let temp = tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");

        let escaped = temp.path().join("escaped.md");
        fs::write(
            &escaped,
            "---\nid: T-999.9\ntitle: Escaped target\npipeline: sdd\n---\n# Escaped target\n",
        )
        .expect("write escaped target");
        let task_path = root.join("todo").join("T-1.1.md");
        unix_fs::symlink(&escaped, &task_path).expect("create task symlink");

        let tasks = TaskRepository::list_tasks(root.to_path_buf()).expect("list tasks");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_id, "T-1.1");
        assert_eq!(tasks[0].path, task_path);
    }
