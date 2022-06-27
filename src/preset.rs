use std::fs;
use std::env::args_os;
use std::path::PathBuf;

use scraper::{Html, Selector};
use once_cell::sync::Lazy;

use super::{Error, Contextualize};



macro_rules! selector {
  ($selector:literal) => {
    Lazy::new(|| Selector::parse($selector).unwrap())
  };
}

const LINK_PREFIX: &str = "http://steamcommunity.com/sharedfiles/filedetails/?id=";

#[derive(Debug)]
pub struct PresetMod {
  pub display_name: String,
  pub id: u64
}

pub fn get_preset_data() -> Result<Vec<PresetMod>, Error> {
  static SELECTOR_MOD: Lazy<Selector> = selector!("body > div.mod-list > table tr[data-type=\"ModContainer\"]");
  static SELECTOR_MOD_NAME: Lazy<Selector> = selector!("td[data-type=\"DisplayName\"]");
  static SELECTOR_MOD_LINK: Lazy<Selector> = selector!("td > a[data-type=\"Link\"]");

  let document = {
    let path = args_os().nth(1)
      .map(PathBuf::from)
      .ok_or(Error::NoPresetProvided)?;
    let data = fs::read_to_string(path)
      .context("Unable to read preset file")?;
    println!("Parsing preset file...");
    Html::parse_document(&data)
  };

  let mut mods = Vec::new();
  for mod_element in document.select(&SELECTOR_MOD) {
    let display_name = mod_element
      .select(&SELECTOR_MOD_NAME).next()
      .and_then(|element| element.text().next())
      .ok_or(Error::PresetParsingFailed)?
      .to_owned();
    let id = mod_element
      .select(&SELECTOR_MOD_LINK).next()
      .and_then(|element| element.value().attr("href"))
      .and_then(|link| link.trim().strip_prefix(LINK_PREFIX))
      .and_then(|id| id.parse::<u64>().ok())
      .ok_or(Error::PresetParsingFailed)?;
    mods.push(PresetMod { display_name, id });
  };

  if mods.is_empty() {
    return Err(Error::PresetParsingFailed);
  };

  Ok(mods)
}
