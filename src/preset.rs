use std::fs;
use std::env::args_os;
use std::path::PathBuf;

use scraper::{Html, Selector};
use once_cell::sync::Lazy;

use super::{Error, Game, Contextualize};



macro_rules! selector {
  ($selector:literal) => {
    Lazy::new(|| Selector::parse($selector).unwrap())
  };
}

const LINK_PREFIX1: &str = "http://steamcommunity.com/sharedfiles/filedetails/?id=";
const LINK_PREFIX2: &str = "https://steamcommunity.com/sharedfiles/filedetails/?id=";

#[derive(Debug)]
pub struct PresetMod {
  pub display_name: String,
  pub id: u64
}

pub fn get_preset_data() -> Result<(Game, Vec<PresetMod>), Error> {
  static SELECTOR_TYPE_ARMA: Lazy<Selector> = selector!("head > meta[name=\"arma:Type\"]");
  static SELECTOR_TYPE_DAYZ: Lazy<Selector> = selector!("head > meta[name=\"dayz:Type\"]");
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

  let game = if document.select(&SELECTOR_TYPE_ARMA).next().is_some() {
    Ok(Game::Arma)
  } else if document.select(&SELECTOR_TYPE_DAYZ).next().is_some() {
    Ok(Game::DayZ)
  } else {
    Err(Error::PresetParsingFailed(Reason::UnknownType))
  }?;

  let mut mods = Vec::new();
  for (index, mod_element) in document.select(&SELECTOR_MOD).enumerate() {
    let display_name = mod_element
      .select(&SELECTOR_MOD_NAME).next()
      .and_then(|element| element.text().next())
      .ok_or_else(|| {
        println!("{}", mod_element.html());
        Error::PresetParsingFailed(Reason::DisplayNameSelector(index))
      })?
      .to_owned();
    let id = mod_element
      .select(&SELECTOR_MOD_LINK).next()
      .and_then(|element| element.value().attr("href"))
      .and_then(strip_workshop_prefix)
      .and_then(|id| id.parse::<u64>().ok())
      .ok_or_else(|| {
        println!("{}", mod_element.html());
        Error::PresetParsingFailed(Reason::LinkSelector(index))
      })?;
    mods.push(PresetMod { display_name, id });
  };

  if mods.is_empty() {
    return Err(Error::PresetParsingFailed(Reason::NoMatches));
  };

  Ok((game, mods))
}

#[derive(Debug, Error)]
pub enum Reason {
  #[error("Preset contains no mods or is invalid")]
  NoMatches,
  #[error("DisplayName selector failed, Index {0}")]
  DisplayNameSelector(usize),
  #[error("Link selector failed, Index {0}")]
  LinkSelector(usize),
  #[error("Unknown preset type, expected an Arma 3 or DayZ preset")]
  UnknownType
}

fn strip_workshop_prefix(link: &str) -> Option<&str> {
  let link = link.trim();
  Option::or(link.strip_prefix(LINK_PREFIX1), link.strip_prefix(LINK_PREFIX2))
}
