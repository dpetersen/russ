use anyhow::{Context, Result};
use rss::Channel as RSSChannel;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::io::{Seek, SeekFrom};

#[derive(Debug)]
pub struct FileDatabase {
    has_content: bool,
    file: std::fs::File,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Channel {
    title: String,
}

impl From<&RSSChannel> for Channel {
    fn from(from: &RSSChannel) -> Self {
        Channel {
            title: from.title.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Database {
    channels: Vec<Channel>,
}

impl FileDatabase {
    pub fn new_for_path(path: &std::path::Path) -> Result<FileDatabase> {
        let exists = path.exists();
        // TODO try and exclusivly lock this
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        Ok(FileDatabase {
            file,
            has_content: exists,
        })
    }

    pub fn persist_channel(&mut self, channel: &RSSChannel) -> Result<()> {
        let mut database = if self.has_content {
            self.load_database().context("reloading database")?
        } else {
            Database { channels: vec![] }
        };

        database.channels.push(channel.into());
        self.file
            .seek(SeekFrom::Start(0))
            .context("rewinding database file")?;
        let mut writer = BufWriter::new(&self.file);
        serde_json::to_writer(&mut writer, &database).context("serializing into file writer")?;
        writer.flush().context("flushing file writer")?;
        self.file.sync_all()?;
        self.has_content = true;
        Ok(())
    }

    pub fn get_channels(&mut self) -> Result<Vec<Channel>> {
        let database = self.load_database()?;
        Ok(database.channels)
    }

    fn load_database(&mut self) -> Result<Database> {
        self.file
            .seek(SeekFrom::Start(0))
            .context("rewinding database file")?;
        let reader = BufReader::new(&self.file);
        let database = serde_json::from_reader(reader).context("deserializing database file")?;
        Ok(database)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_persist_channel() -> Result<()> {
        let dir = tempdir()?.into_path().join("test.json");
        {
            let mut database =
                FileDatabase::new_for_path(&dir).context("loading initial database")?;
            let rss_channel = RSSChannel {
                title: "Test Channel".to_string(),
                ..Default::default()
            };
            database
                .persist_channel(&rss_channel)
                .context("initial persistence")?;
        }

        let mut database = FileDatabase::new_for_path(&dir).context("reloading database")?;
        let rss_channel = RSSChannel {
            title: "Test Channel 2".to_string(),
            ..Default::default()
        };
        database
            .persist_channel(&rss_channel)
            .context("second persistence")?;

        let channels = database.get_channels().context("loading channels")?;
        assert_eq!(2, channels.len());
        let mut sorted_titles: Vec<String> = channels.into_iter().map(|c| c.title).collect();
        sorted_titles.sort();
        assert_eq!("Test Channel", sorted_titles.first().unwrap());

        Ok(())
    }

    #[test]
    fn test_persist_channel_replaces_existing() -> Result<()> {
        let dir = tempdir()?.into_path().join("test.json");
        let mut database = FileDatabase::new_for_path(&dir).context("loading initial database")?;
        for _ in 0..=1 {
            let rss_channel = RSSChannel {
                title: "Test Channel".to_string(),
                ..Default::default()
            };
            database.persist_channel(&rss_channel)?;
        }

        let channels = database.get_channels().context("loading channels")?;
        todo!("here's your failure, need to make sure you are overwriting existing channels. Also, title shouldn't be the 'primary key'");
        assert_eq!(1, channels.len());

        Ok(())
    }
}
