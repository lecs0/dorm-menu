use anyhow::Result;
use dorm_shared::Store;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
};
pub fn load(dir: &Path) -> Result<Store> {
    let path = dir.join("menus.json");
    if !path.exists() {
        return Ok(Store::default());
    }
    let data = fs::read(path)?;
    anyhow::ensure!(data.len() < 4_000_000, "store too large");
    Ok(serde_json::from_slice(&data)?)
}
pub fn save(dir: &Path, store: &Store) -> Result<()> {
    fs::create_dir_all(dir)?;
    let path = dir.join("menus.tmp");
    let mut f = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&path)?;
    f.write_all(&serde_json::to_vec(store)?)?;
    f.sync_all()?;
    fs::rename(path, dir.join("menus.json"))?;
    File::open(dir)?.sync_all()?;
    Ok(())
}
