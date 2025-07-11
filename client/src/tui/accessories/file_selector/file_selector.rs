use anyhow::Result;
use std::{env, path::PathBuf};

use crate::util::{
    config::FILES_IMG_TO_ASCII,
    types::{FileAction, SelectorEntry, SelectorEntryKind},
};

pub struct FileSelector {
    pub current_location: PathBuf,
    pub selected_index: usize,
    pub entries: Vec<SelectorEntry>,
    pub scroll_offset: u16,
    pub active_action: FileAction,
}

impl FileSelector {
    pub fn new() -> Self {
        let back = SelectorEntry {
            name: "../".into(),
            kind: SelectorEntryKind::Folder,
            selected: false,
        };
        let mut instance = Self {
            current_location: env::current_dir().expect("Failed to get current directory"),
            selected_index: 0,
            entries: vec![back],
            scroll_offset: 0,
            active_action: FileAction::File,
        };

        instance.update_entries().unwrap();

        instance
    }

    pub fn switch_action(&mut self) -> Result<()> {
        self.active_action = match &self.active_action {
            FileAction::ASCII => FileAction::File,
            FileAction::File => FileAction::ASCII,
        };

        self.selected_index = 0;

        self.update_entries()?;
        Ok(())
    }

    pub fn open_folder(&mut self) -> Result<()> {
        let selected = &self.entries[self.selected_index];
        if (selected.kind != SelectorEntryKind::Folder) || (selected.name == String::from("../")) {
            return Ok(());
        };

        self.current_location.push(&selected.name);
        self.selected_index = 0;
        self.update_entries()?;
        Ok(())
    }

    pub fn close_current_folder(&mut self) -> Result<()> {
        self.current_location.pop();
        self.selected_index = 0;
        self.update_entries()?;
        Ok(())
    }

    pub fn move_down(&mut self) -> Result<()> {
        let len = self.entries.len();
        let i = self.selected_index;
        if i != len - 1 {
            self.selected_index = i + 1;
        };
        self.update_entries()?;
        Ok(())
    }

    pub fn move_up(&mut self) -> Result<()> {
        let i = self.selected_index;
        if i != 0 {
            self.selected_index = i - 1;
        };
        self.update_entries()?;
        Ok(())
    }

    pub fn reset_location(&mut self) -> Result<()> {
        self.current_location = env::current_dir().expect("Failed to get current directory");
        self.selected_index = 0;
        self.update_entries()?;
        Ok(())
    }

    pub fn update_entries(&mut self) -> Result<()> {
        let back = SelectorEntry {
            name: "../".into(),
            kind: SelectorEntryKind::Folder,
            selected: false,
        };

        let mut entries: Vec<SelectorEntry> = vec![back];
        let dir = std::fs::read_dir(&self.current_location)?;

        for file in dir {
            let file = match file {
                Err(_) => continue,
                Ok(entry) => entry,
            };

            let file_metadata = file.metadata()?;

            if (file_metadata.is_dir() && !std::fs::read_dir(file.path()).is_ok())
                || (file_metadata.is_file() && !std::fs::File::open(file.path()).is_ok())
            {
                continue;
            }

            let filename = file.file_name();
            let filename = filename.to_string_lossy();

            let suffix = filename.split(".").last().unwrap();
            let is_dir = file.file_type()?.is_dir();

            if self.active_action == FileAction::ASCII
                && !FILES_IMG_TO_ASCII.contains(&suffix)
                && !is_dir
            {
                continue;
            };

            let option_kind = match is_dir {
                true => SelectorEntryKind::Folder,
                false => SelectorEntryKind::File,
            };
            let entry = SelectorEntry {
                name: filename.into(),
                kind: option_kind,
                selected: false,
            };

            entries.push(entry);
        }

        self.entries = entries;
        if let Some(entry) = self.entries.get_mut(self.selected_index) {
            entry.selected = true;
        };
        Ok(())
    }
}
