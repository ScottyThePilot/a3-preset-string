use std::io::{self, BufReader};
use std::fs::File;
use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Deserializer};
use serde::de::{Unexpected, Error as DeError};

use super::{Error, Game};



pub fn get_manifest_data(game: Game) -> Result<HashMap<u64, ManifestMod>, Error> {
  let game_manifest_dir = match game {
    Game::Arma => "Arma 3 Launcher/Steam.json",
    Game::DayZ => "DayZ Launcher/Steam.json"
  };

  let path = dirs::data_local_dir()
    .expect("unable to get data dir for this platform")
    .join(game_manifest_dir);
  //let path = PathBuf::from("steam-example.json");
  let file = match File::open(&path) {
    Err(err) if err.kind() == io::ErrorKind::NotFound => return Err(Error::NoManifestFound(path)),
    Err(err) => return Err(Error::IoError(err, "Unable to open Steam.json")),
    Ok(file) => file
  };

  println!("Parsing manifest file...");
  let file = BufReader::new(file);
  let manifest: ManifestRepr = serde_json::from_reader(file)
    .map_err(Error::ManifestParsingFailed)?;

  let mut mods = HashMap::new();
  for extension in manifest.extensions {
    let ExtensionRepr {
      id,
      display_name,
      extension_path: path,
      storage_info: StorageInfoRepr {
        file_system_size: file_size
      },
      steam_dependencies: dependencies
    } = extension;

    mods.insert(id, ManifestMod {
      id, display_name, path, file_size, dependencies
    });
  };

  Ok(mods)
}

pub struct ManifestMod {
  pub id: u64,
  pub display_name: String,
  pub path: PathBuf,
  pub file_size: u64,
  pub dependencies: Vec<u64>
}



#[derive(Debug, Deserialize)]
struct ManifestRepr {
  #[serde(rename = "Extensions")]
  extensions: Vec<ExtensionRepr>
}

#[derive(Debug, Deserialize)]
struct ExtensionRepr {
  #[serde(rename = "Id")]
  #[serde(deserialize_with = "deserialize_steam_id_str")]
  id: u64,
  #[serde(rename = "DisplayName")]
  display_name: String,
  #[serde(rename = "ExtensionPath")]
  extension_path: PathBuf,
  #[serde(rename = "StorageInfo")]
  storage_info: StorageInfoRepr,
  #[serde(rename = "SteamDependencies")]
  steam_dependencies: Vec<u64>
}

#[derive(Debug, Deserialize)]
struct StorageInfoRepr {
  #[serde(rename = "FileSystemSize")]
  file_system_size: u64
}

fn deserialize_steam_id_str<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
  fn error<E: DeError>(value: &str) -> E {
    E::invalid_value(Unexpected::Str(value), &"a string, prefixed with \"steam:\" followed by a u64")
  }

  let id = String::deserialize(deserializer)?;
  let id = id.strip_prefix("steam:").ok_or_else(|| error(&id))?;
  let id = id.parse::<u64>().map_err(|_| error(id))?;
  Ok(id)
}
