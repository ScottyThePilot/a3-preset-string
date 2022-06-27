#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
extern crate dirs;
extern crate rfd;
extern crate serde;
extern crate serde_json;
extern crate scraper;
#[macro_use]
extern crate thiserror;
extern crate once_cell;

mod manifest;
mod preset;

use std::io;
use std::fs;
use std::fmt::{self, Write};
use std::cmp::{PartialOrd, Ord, Ordering};
use std::error::Error as StdError;
use std::path::PathBuf;

use rfd::{MessageDialog, MessageButtons, MessageLevel};

use crate::preset::get_preset_data;
use crate::manifest::get_manifest_data;



#[derive(Debug, Error)]
pub enum Error {
  #[error("No preset file provided")]
  NoPresetProvided,
  #[error("No Steam.json found at {}", .0.display())]
  NoManifestFound(PathBuf),
  #[error("{1}: {0}")]
  IoError(io::Error, &'static str),
  #[error("Failed to parse preset HTML file")]
  PresetParsingFailed,
  #[error("Failed to parse Steam.json: {0}")]
  ManifestParsingFailed(serde_json::Error),
  #[error("Preset and Steam.json have conflicting display names (Is the mod up to date): \"{0}\", \"{1}\"\nSteam.json will take precedence")]
  ConflictingDisplayNames(String, String),
  #[error("Preset contains mods that Steam.json lacks (Are they subscribed?): {}", DisplayMods(.0))]
  UnknownMods(Vec<(String, u64)>),
  #[error("Mod name contains a semicolon: \"{0}\"")]
  ModNameContainsSemicolon(String),
  #[error("Preset contains no mods or is invalid")]
  NoMods
}

#[derive(Debug, Copy, Clone)]
struct DisplayMods<'a>(&'a [(String, u64)]);

impl<'a> fmt::Display for DisplayMods<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    for (i, &(ref display_name, id)) in self.0.into_iter().enumerate() {
      if i != 0 { f.write_str(", ")? };
      write!(f, "\"{display_name}\" ({id})")?;
    };

    Ok(())
  }
}



const SUCCESS: &str = "Successfully created name-list.txt and id-list.txt";

fn main() {
  match run() {
    Ok(()) => show_info("Success", SUCCESS),
    Err(err) => show_error(err)
  };
}

fn run() -> Result<(), Error> {
  let preset_mods = get_preset_data()?;
  let manifest_mods = get_manifest_data()?;

  let mut mods = Vec::new();
  let mut unknown_mods = Vec::new();
  for preset_mod in preset_mods {
    println!("Mod: \"{}\" SteamID: {}", preset_mod.display_name, preset_mod.id);
    if let Some(manifest_mod) = manifest_mods.get(&preset_mod.id) {
      if preset_mod.display_name != manifest_mod.display_name {
        show_warning(Error::ConflictingDisplayNames(preset_mod.display_name.clone(), manifest_mod.display_name.clone()));
      };

      if manifest_mod.display_name.contains(';') {
        return Err(Error::ModNameContainsSemicolon(manifest_mod.display_name.clone()));
      };

      mods.push(Mod {
        id: preset_mod.id,
        display_name: manifest_mod.display_name.clone(),
        file_size: manifest_mod.file_size,
        dependencies: manifest_mod.dependencies.clone()
      });
    } else {
      unknown_mods.push((preset_mod.display_name, preset_mod.id));
    };
  };

  println!();
  mods.sort();

  let mut name_list_output = String::new();
  let mut id_list_output = String::new();
  for Mod { display_name, id, .. } in mods {
    write!(&mut name_list_output, "@{display_name};").unwrap();
    write!(&mut id_list_output, "{id},").unwrap();
  };

  if id_list_output.ends_with(',') {
    id_list_output.pop();
  };

  println!("{}", name_list_output);
  println!("{}", id_list_output);

  fs::write("name-list.txt", name_list_output).context("Failed to write name-list.txt")?;
  fs::write("id-list.txt", id_list_output).context("Failed to write id-list.txt")?;

  Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub struct Mod {
  pub id: u64,
  pub display_name: String,
  pub file_size: u64,
  pub dependencies: Vec<u64>
}

impl Mod {
  #[inline]
  pub fn has_dependency(&self, id: u64) -> bool {
    self.dependencies.contains(&id)
  }
}

impl PartialOrd for Mod {
  #[inline]
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(Ord::cmp(self, other))
  }
}

impl Ord for Mod {
  fn cmp(&self, other: &Self) -> Ordering {
    if self.has_dependency(other.id) {
      Ordering::Greater
    } else if other.has_dependency(self.id) {
      Ordering::Less
    } else {
      Ord::cmp(&self.file_size, &other.file_size).reverse()
    }
  }
}



pub trait Contextualize {
  type Output;

  fn context(self, ctx: &'static str) -> Self::Output;
}

impl<T> Contextualize for Result<T, io::Error> {
  type Output = Result<T, Error>;

  #[inline]
  fn context(self, ctx: &'static str) -> Result<T, Error> {
    self.map_err(|err| Error::IoError(err, ctx))
  }
}



fn show_info(title: &str, desc: &str) {
  MessageDialog::new()
    .set_title(title)
    .set_description(desc)
    .set_level(MessageLevel::Info)
    .set_buttons(MessageButtons::Ok)
    .show();
}

fn show_warning(msg: impl StdError) {
  MessageDialog::new()
    .set_title("Warning")
    .set_description(&msg.to_string())
    .set_level(MessageLevel::Warning)
    .set_buttons(MessageButtons::Ok)
    .show();
}

fn show_error(msg: impl StdError) {
  MessageDialog::new()
    .set_title("Error")
    .set_description(&msg.to_string())
    .set_level(MessageLevel::Error)
    .set_buttons(MessageButtons::Ok)
    .show();
}
