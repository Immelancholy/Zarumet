use color_eyre::Result;
use mpd_client::{Client, protocol::command::Command};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub path: String,
    pub is_directory: bool,
}

impl DirectoryEntry {
    pub fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }
}

pub async fn browse(client: &Client, path: &str) -> Result<Vec<DirectoryEntry>, String> {
    // mpd_client has no typed lsinfo wrapper. Let its protocol builder escape paths.
    let mut command = Command::new("lsinfo");
    command.add_argument(path).map_err(|e| e.to_string())?;
    let frame = client
        .raw_command(command)
        .await
        .map_err(|e| e.to_string())?;
    let mut entries: Vec<_> = frame
        .into_iter()
        .filter_map(|(key, value)| match key.as_ref() {
            "directory" | "file" => Some(DirectoryEntry {
                path: value.to_string(),
                is_directory: key.as_ref() == "directory",
            }),
            _ => None,
        })
        .collect();
    entries.sort_by(|a, b| {
        b.is_directory
            .cmp(&a.is_directory)
            .then(a.path.cmp(&b.path))
    });
    Ok(entries)
}

pub async fn enqueue(client: &Client, entry: &DirectoryEntry) -> Result<()> {
    // Unlike addid, add accepts directories and expands them recursively on the server.
    let mut command = Command::new("add");
    command.add_argument(&entry.path)?;
    client.raw_command(command).await?;
    Ok(())
}
