use anyhow::Result;
use async_recursion::async_recursion;
use lazy_static::__Deref;
use rocket::http::Status;
use std::{collections::HashSet, fmt::Display, path::PathBuf};
use tokio::{
    fs::{self, DirEntry, File},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
};

use crate::{github::action::parser::AuthorizedAction, workspace::Workspace};

use super::MasonRegistryCommand;

#[async_recursion]
async fn read_dir_recursively(dir: &PathBuf, entries: &mut Vec<DirEntry>) -> Result<()> {
    let mut reader = fs::read_dir(dir).await?;
    while let Some(entry) = reader.next_entry().await? {
        match entry.path() {
            path if path.is_dir() => {
                read_dir_recursively(&path, entries).await?;
            }
            _ => {
                entries.push(entry);
            }
        };
    }
    Ok(())
}

async fn yml_to_yaml(workspace: &Workspace, changed_files: &HashSet<PathBuf>) -> Result<()> {
    let mut packages_dir = workspace.workdir.path().to_path_buf();
    packages_dir.push("packages");
    let mut entries = vec![];
    read_dir_recursively(&packages_dir, &mut entries).await?;
    for entry in entries {
        if !changed_files.contains(&entry.path()) {
            continue;
        }
        match entry.file_name().to_string_lossy().deref() {
            file_name if file_name.ends_with(".yml") => {
                let mut new_entry_path = entry.path();
                new_entry_path.set_extension("yaml");
                fs::rename(entry.path(), &new_entry_path).await?;
                let entry_path = entry.path();
                workspace
                    .spawn(
                        "git",
                        [
                            "add",
                            &entry_path.to_string_lossy(),
                            &new_entry_path.to_string_lossy(),
                        ],
                    )
                    .await?;
                workspace
                    .commit(&format!(
                        "fix({}): move {} to {}",
                        entry_path
                            .as_path()
                            .parent()
                            .unwrap()
                            .strip_prefix(&packages_dir)
                            .unwrap()
                            .to_string_lossy(),
                        entry_path.file_name().unwrap().to_string_lossy(),
                        new_entry_path.file_name().unwrap().to_string_lossy()
                    ))
                    .await?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn apply_styling_fixes<'a>(lines: &'a Vec<String>) -> Vec<&'a str> {
    let mut new_file_lines = vec![];
    let mut it = lines.windows(2).peekable();
    while let Some(slice) = it.next() {
        if let (Some(line1), Some(line2)) = (slice.get(0), slice.get(1)) {
            match (line1.as_str(), line2.as_str()) {
                (line1, _line2 @ ("source:" | "bin:" | "share:" | "opt:")) if line1 != "" => {
                    new_file_lines.push(line1);
                    new_file_lines.push("");
                }
                (line1, _) => {
                    new_file_lines.push(line1);
                }
            }
            if it.peek().is_none() {
                new_file_lines.push(line2);
            }
        } else {
            new_file_lines.extend(slice.iter().map(|s| s.as_str()));
        }
    }
    if new_file_lines.get(0) != Some(&"---") {
        new_file_lines.insert(0, "---");
    }
    return new_file_lines;
}

async fn fix_styling(workspace: &Workspace, changed_files: &HashSet<PathBuf>) -> Result<()> {
    let mut packages_dir = workspace.workdir.path().to_path_buf();
    packages_dir.push("packages");
    let mut entries = vec![];
    read_dir_recursively(&packages_dir, &mut entries).await?;
    for entry in entries {
        if !changed_files.contains(&entry.path()) {
            continue;
        }
        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }
        let file = File::open(&entry_path).await?;
        let mut reader = BufReader::new(file).lines();
        let mut lines = vec![];
        while let Some(line) = reader.next_line().await? {
            lines.push(line);
        }

        let new_file_lines = apply_styling_fixes(&lines);

        let mut new_file = File::create(&entry_path).await?;
        for line in new_file_lines {
            new_file.write_all(line.as_bytes()).await?;
            new_file.write_all(b"\n").await?;
        }

        workspace
            .spawn("git", ["add", &entry_path.to_string_lossy()])
            .await?;
        let _ = workspace
            .commit(&format!(
                "style({}): fix formatting",
                entry_path
                    .as_path()
                    .parent()
                    .unwrap()
                    .strip_prefix(&packages_dir)
                    .unwrap()
                    .to_string_lossy()
            ))
            .await;
    }
    Ok(())
}

pub(super) async fn run(
    action: &AuthorizedAction<MasonRegistryCommand>,
) -> Result<Box<dyn Display + Send>, (Status, anyhow::Error)> {
    let workspace = Workspace::create(&action).await?;

    async {
        let changed_files = workspace
            .get_changed_files()
            .await?
            .iter()
            .map(|path| {
                let mut new_path = PathBuf::new();
                new_path.push(workspace.workdir.path());
                new_path.push(path);
                new_path
            })
            .collect::<HashSet<PathBuf>>();
        yml_to_yaml(&workspace, &changed_files).await?;
        fix_styling(&workspace, &changed_files).await?;
        workspace.push().await?;
        Ok::<(), anyhow::Error>(())
    }
    .await
    .map_err(|err| (Status::InternalServerError, err))?;

    Ok(Box::new(format!(
        "Successfully ran mason-registry fixup in {:?}",
        workspace
    )))
}

#[cfg(test)]
mod tests {
    use super::apply_styling_fixes;
    use indoc::indoc;

    fn from_str(str: &str) -> Vec<String> {
        str.split("\n").map(ToOwned::to_owned).collect()
    }

    fn from_vec(vec: &Vec<&str>) -> String {
        vec.join("\n")
    }

    #[test]
    fn it_should_apply_yaml_document_header() {
        assert_eq!(
            indoc! {"
            ---
            name: lua-language-server
        "},
            from_vec(&apply_styling_fixes(&from_str(indoc! {"
            name: lua-language-server
        "})))
        );
    }

    #[test]
    fn it_should_apply_whitespaces() {
        assert_eq!(
            indoc! {r#"
                ---
                name: mdslw
                description: The MarkDown Sentence Line Wrapper, an auto-formatter that prepares your markdown for easy diff'ing.
                homepage: https://github.com/razziel89/mdslw
                licenses:
                  - GPL-3.0-only
                languages:
                  - Markdown
                categories:
                  - Formatter

                source:
                  id: pkg:github/razziel89/mdformat@0.6.1
                  asset:
                    - target: darwin_x64
                      file: mdslw_x86_64-apple-darwin
                    - target: linux_x64
                      file: mdslw_x86_64-unknown-linux-musl
                    - target: win_x64
                      file: mdslw_x86_64-pc-windows-gnu.exe

                bin:
                  mdslw: "{{source.asset.file}}"
        "#},
            from_vec(&apply_styling_fixes(&from_str(indoc! {r#"
                name: mdslw
                description: The MarkDown Sentence Line Wrapper, an auto-formatter that prepares your markdown for easy diff'ing.
                homepage: https://github.com/razziel89/mdslw
                licenses:
                  - GPL-3.0-only
                languages:
                  - Markdown
                categories:
                  - Formatter
                source:
                  id: pkg:github/razziel89/mdformat@0.6.1
                  asset:
                    - target: darwin_x64
                      file: mdslw_x86_64-apple-darwin
                    - target: linux_x64
                      file: mdslw_x86_64-unknown-linux-musl
                    - target: win_x64
                      file: mdslw_x86_64-pc-windows-gnu.exe
                bin:
                  mdslw: "{{source.asset.file}}"
        "#})))
        );
    }
}
