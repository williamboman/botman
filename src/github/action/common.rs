use anyhow::{anyhow, bail, Result};

#[derive(Debug)]
pub struct GitApplyPatch {
    pub patch: String,
}

impl TryFrom<String> for GitApplyPatch {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self> {
        let massaged_value = value
            .trim_start_matches(char::is_whitespace)
            .replace("\r", "");
        let lines = massaged_value.split_inclusive("\n");
        let mut lines_iter = lines.clone().into_iter();
        let header = lines_iter.next().ok_or_else(|| anyhow!("No header."))?;
        if !header.starts_with("```diff") {
            bail!("Not a diff.")
        }
        let mut patch = String::new();
        for line in lines_iter {
            match line {
                "```" => break,
                _ => patch.push_str(line),
            }
        }
        Ok(Self { patch })
    }
}
